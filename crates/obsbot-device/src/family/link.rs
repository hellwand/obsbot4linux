// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! One camera's link over its [`Transport`]: short commands, framed commands, queries and the
//! polling of their answers, reads and write-backs of whole selector buffers, UVC controls.
//! It knows no selector, receiver or command of a model: the model passes them in.
//!
//! Every exchange checks the operation's [`Ctx`] before its frame (a request stops at its
//! deadline, a group read stops when a stop arrives: 4.1 rule 9, 5.6) and holds the transport's
//! lock on the video node for the whole exchange, a query and the reads of its answer included
//! (5.8 `NODE_LOCK_WAIT`).

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx, Transport};
use crate::family::frame::{
    Answer, FRAME_SIZE, Sequence, parse_answer, query_frame, query_frame_with,
};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// `ANSWER_POLL` (5.8): the gap before each read of an answer.
pub const REPLY_GAP: Duration = Duration::from_millis(50);
/// `ANSWER_POLL` (5.8): reads of an answer before the query counts as unanswered.
pub const REPLY_ATTEMPTS: usize = 20;
/// `ANSWER_POLL` (5.8): [`REPLY_ATTEMPTS`] reads, [`REPLY_GAP`] apart, per query.
pub const ANSWER_POLL: (usize, Duration) = (REPLY_ATTEMPTS, REPLY_GAP);
/// `FRAME_GAP` (5.8): between two frames of one command where the model needs it (a pair).
pub const FRAME_GAP: Duration = Duration::from_millis(100);

/// The waits of a link. Tests use [`Timing::instant`]; the camera needs [`Timing::CAMERA`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub reply_gap: Duration,
    pub frame_gap: Duration,
}

impl Timing {
    pub const CAMERA: Timing = Timing {
        reply_gap: REPLY_GAP,
        frame_gap: FRAME_GAP,
    };

    /// No waiting: for tests on the replay transport.
    pub const fn instant() -> Timing {
        Timing {
            reply_gap: Duration::ZERO,
            frame_gap: Duration::ZERO,
        }
    }
}

/// The sequence counter of the process: every link of every camera takes its numbers from it,
/// so two frames in a row never repeat a number.
pub fn process_sequence() -> &'static Sequence {
    static SEQ: OnceLock<Sequence> = OnceLock::new();
    SEQ.get_or_init(Sequence::new)
}

/// Polls `read` for the answer to the query with `seq` and `command`: waits `gap` before each of
/// [`REPLY_ATTEMPTS`] reads. A read that fails ends the poll at once with its error (an unplug
/// is `Gone` at once, not "no answer" after a second); a read that is not the answer yet (a
/// stale buffer) is read again.
pub fn poll(
    command: u16,
    seq: u16,
    gap: Duration,
    mut read: impl FnMut() -> Result<[u8; FRAME_SIZE]>,
) -> Result<Vec<u8>> {
    for _ in 0..REPLY_ATTEMPTS {
        sleep(gap);
        let reply = read()?;
        match parse_answer(&reply, seq, command) {
            Some(Answer::Value(data)) => return Ok(data.to_vec()),
            Some(Answer::Empty) => return Ok(Vec::new()),
            Some(Answer::Rejected(code)) => {
                return Err(CameraError::Rejected(format!(
                    "query {command:#06x} (code {code})"
                )));
            }
            None => {}
        }
    }
    Err(CameraError::NoAnswer(format!("query {command:#06x}")))
}

/// The link of one camera. The model owns it; nothing else talks to the transport.
pub struct Link {
    transport: Box<dyn Transport>,
    seq: &'static Sequence,
    timing: Timing,
    /// When the last frame of [`Link::send_spaced`] went out.
    spaced: Option<Instant>,
}

impl Link {
    pub fn new(transport: Box<dyn Transport>, seq: &'static Sequence, timing: Timing) -> Link {
        Link {
            transport,
            seq,
            timing,
            spaced: None,
        }
    }

    /// The next sequence number, for a frame the model builds itself.
    pub fn seq(&self) -> u16 {
        self.seq.next()
    }

    pub fn timing(&self) -> Timing {
        self.timing
    }

    /// The transport, for the facts that are no exchange (`usb`, `usb_serial`, `on_air`).
    pub fn transport(&mut self) -> &mut dyn Transport {
        self.transport.as_mut()
    }

    /// Runs `f` under the transport's lock, after the context check.
    fn exchange<T>(
        &mut self,
        ctx: &Ctx,
        f: impl FnOnce(&mut dyn Transport, Timing) -> Result<T>,
    ) -> Result<T> {
        ctx.check()?;
        self.transport.hold()?;
        let result = f(self.transport.as_mut(), self.timing);
        self.transport.release();
        result
    }

    /// A short command: `payload` written to `selector`.
    pub fn short(&mut self, selector: u8, payload: &[u8], ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.xu_set(selector, payload))
    }

    /// One framed command written to `selector`.
    pub fn send(&mut self, selector: u8, frame: &[u8; FRAME_SIZE], ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.xu_set(selector, frame))
    }

    /// A framed command that keeps [`FRAME_GAP`] to the previous one sent this way: for commands
    /// that the camera drops when two follow each other back to back (the two framing offsets,
    /// which a patch may write one after the other).
    pub fn send_spaced(&mut self, selector: u8, frame: &[u8; FRAME_SIZE], ctx: &Ctx) -> Result<()> {
        if let Some(last) = self.spaced {
            let wait = self.timing.frame_gap.saturating_sub(last.elapsed());
            sleep(wait);
        }
        let sent = self.send(selector, frame, ctx);
        self.spaced = Some(Instant::now());
        sent
    }

    /// Two framed commands of one operation, [`FRAME_GAP`] apart: back to back the camera
    /// applies neither. The context is checked before each.
    pub fn pair(
        &mut self,
        selector: u8,
        first: &[u8; FRAME_SIZE],
        second: &[u8; FRAME_SIZE],
        ctx: &Ctx,
    ) -> Result<()> {
        self.send(selector, first, ctx)?;
        sleep(self.timing.frame_gap);
        self.send(selector, second, ctx)
    }

    /// A query without a request (frame type `0x01`) and its answer.
    pub fn query(
        &mut self,
        selector: u8,
        receiver: u8,
        command: u16,
        ctx: &Ctx,
    ) -> Result<Vec<u8>> {
        let seq = self.seq.next();
        let frame = query_frame(receiver, command, seq);
        self.ask(selector, &frame, command, seq, ctx)
    }

    /// A query with a request (frame type `0x21`, for example a view id) and its answer.
    pub fn query_with(
        &mut self,
        selector: u8,
        receiver: u8,
        command: u16,
        request: &[u8],
        ctx: &Ctx,
    ) -> Result<Vec<u8>> {
        let seq = self.seq.next();
        let frame = query_frame_with(receiver, command, request, seq);
        self.ask(selector, &frame, command, seq, ctx)
    }

    fn ask(
        &mut self,
        selector: u8,
        frame: &[u8; FRAME_SIZE],
        command: u16,
        seq: u16,
        ctx: &Ctx,
    ) -> Result<Vec<u8>> {
        self.exchange(ctx, |t, timing| {
            t.xu_set(selector, frame)?;
            poll(command, seq, timing.reply_gap, || {
                let mut buf = [0u8; FRAME_SIZE];
                t.xu_get(selector, &mut buf)?;
                Ok(buf)
            })
        })
    }

    /// One read of a selector's whole buffer (the status block, a preset list entry).
    pub fn read(&mut self, selector: u8, ctx: &Ctx) -> Result<[u8; FRAME_SIZE]> {
        self.exchange(ctx, |t, _| {
            let mut buf = [0u8; FRAME_SIZE];
            t.xu_get(selector, &mut buf)?;
            Ok(buf)
        })
    }

    /// A selector's buffer written back as it was read (the preset list rewind, 8.8).
    pub fn write_back(&mut self, selector: u8, block: &[u8; FRAME_SIZE], ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.xu_set(selector, block))
    }

    pub fn ctrl_get(&mut self, id: u32, ctx: &Ctx) -> Result<i32> {
        self.exchange(ctx, |t, _| t.ctrl_get(id))
    }

    pub fn ctrl_set(&mut self, id: u32, value: i32, ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.ctrl_set(id, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ControlRange, OnAir};
    use crate::family::frame::frame;
    use obsbot_proto::Usb;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// What the test transport saw, in order.
    #[derive(Debug, Clone, PartialEq)]
    enum Call {
        Hold,
        Release,
        Set(u8, Vec<u8>),
        Get(u8),
        CtrlGet(u32),
        CtrlSet(u32, i32),
    }

    /// A transport that logs every call and answers selector reads from a script: the n-th
    /// read gets `reads[n]` (the last entry repeats).
    struct Wire {
        log: Arc<Mutex<Vec<Call>>>,
        reads: Vec<Result<[u8; FRAME_SIZE]>>,
        n: usize,
        locked: bool,
    }

    impl Wire {
        fn new(reads: Vec<Result<[u8; FRAME_SIZE]>>) -> (Wire, Arc<Mutex<Vec<Call>>>) {
            let log = Arc::new(Mutex::new(Vec::new()));
            (
                Wire {
                    log: log.clone(),
                    reads,
                    n: 0,
                    locked: false,
                },
                log,
            )
        }
        fn push(&self, call: Call) {
            self.log.lock().unwrap().push(call);
        }
    }

    impl Transport for Wire {
        fn xu_len(&mut self, _selector: u8) -> Result<u16> {
            Ok(60)
        }
        fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
            self.push(Call::Get(selector));
            let i = self.n.min(self.reads.len() - 1);
            self.n += 1;
            buf.copy_from_slice(&self.reads[i].clone()?);
            Ok(())
        }
        fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
            assert!(self.locked, "every write happens under the lock");
            self.push(Call::Set(selector, data.to_vec()));
            Ok(())
        }
        fn ctrl_get(&mut self, id: u32) -> Result<i32> {
            self.push(Call::CtrlGet(id));
            Ok(7)
        }
        fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
            self.push(Call::CtrlSet(id, value));
            Ok(())
        }
        fn ctrl_range(&mut self, _id: u32) -> Result<Option<ControlRange>> {
            Ok(None)
        }
        fn usb(&mut self) -> Result<Usb> {
            Err(CameraError::Gone)
        }
        fn usb_serial(&mut self) -> Result<Option<String>> {
            Ok(None)
        }
        fn on_air(&mut self) -> Result<OnAir> {
            Ok(OnAir::default())
        }
        fn hold(&mut self) -> Result<()> {
            self.locked = true;
            self.push(Call::Hold);
            Ok(())
        }
        fn release(&mut self) {
            self.locked = false;
            self.push(Call::Release);
        }
    }

    fn seq_at(first: u16) -> &'static Sequence {
        Box::leak(Box::new(Sequence::starting_at(first)))
    }

    /// An answer of `kind` to the query with `seq` and `command`, as the camera sends it.
    fn reply(kind: u8, seq: u16, command: u16, code: Option<i16>, data: &[u8]) -> [u8; 60] {
        crate::transport::replay::answer_frame(kind, seq, 0x04, command, code, data)
    }

    fn link(reads: Vec<Result<[u8; 60]>>, first: u16) -> (Link, Arc<Mutex<Vec<Call>>>) {
        let (wire, log) = Wire::new(reads);
        (
            Link::new(Box::new(wire), seq_at(first), Timing::instant()),
            log,
        )
    }

    fn gets(log: &Arc<Mutex<Vec<Call>>>) -> usize {
        log.lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, Call::Get(_)))
            .count()
    }

    #[test]
    fn a_query_is_one_frame_and_the_reads_of_its_answer_under_one_hold() {
        let (mut link, log) = link(vec![Ok(reply(0x29, 7, 0x0104, None, &[1, 2, 3]))], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()).unwrap(),
            [1, 2, 3]
        );
        assert_eq!(
            *log.lock().unwrap(),
            [
                Call::Hold,
                Call::Set(2, query_frame(0x04, 0x0104, 7).to_vec()),
                Call::Get(2),
                Call::Release
            ]
        );
    }

    #[test]
    fn a_stale_answer_is_read_again_until_the_answer_comes() {
        // the previous query's answer (sequence 6) is still in the buffer for two reads
        let stale = reply(0x29, 6, 0x0104, None, &[9]);
        let (mut link, log) = link(
            vec![Ok(stale), Ok(stale), Ok(reply(0x29, 7, 0x0104, None, &[4]))],
            7,
        );
        assert_eq!(link.query(2, 0x04, 0x0104, &Ctx::none()).unwrap(), [4]);
        assert_eq!(gets(&log), 3);
    }

    #[test]
    fn a_failed_read_ends_the_poll_at_once() {
        // an unplug while waiting for the answer is reported at the first failed read, not as
        // "no answer" after the whole poll (5.3, audit: the answer poll took every read error
        // for "not ready")
        let (mut link, log) = link(vec![Err(CameraError::Gone)], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::Gone)
        );
        assert_eq!(gets(&log), 1);
        assert_eq!(
            log.lock().unwrap().last(),
            Some(&Call::Release),
            "the lock is given back"
        );
    }

    #[test]
    fn a_rejected_query_returns_the_cameras_code_at_the_first_read() {
        let (mut link, log) = link(vec![Ok(reply(0x89, 7, 0x0104, Some(-3), &[]))], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::Rejected("query 0x0104 (code -3)".into()))
        );
        assert_eq!(gets(&log), 1);
    }

    #[test]
    fn an_empty_answer_is_an_empty_value() {
        let (mut link, _) = link(vec![Ok(reply(0x09, 7, 0x1344, None, &[]))], 7);
        assert_eq!(
            link.query(2, 0x04, 0x1344, &Ctx::none()).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn without_an_answer_the_query_gives_up_after_all_reads() {
        let (mut link, log) = link(vec![Ok([0u8; 60])], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::NoAnswer("query 0x0104".into()))
        );
        assert_eq!(gets(&log), REPLY_ATTEMPTS);
    }

    #[test]
    fn the_answer_to_another_query_with_our_number_is_not_ours() {
        // a process without the node lock asked 0x0144 with the same sequence number (#61)
        let (mut link, log) = link(vec![Ok(reply(0x29, 7, 0x0144, None, &[1]))], 7);
        assert!(matches!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::NoAnswer(_))
        ));
        assert_eq!(gets(&log), REPLY_ATTEMPTS);
    }

    #[test]
    fn a_query_with_a_request_is_a_type_0x21_frame() {
        let (mut link, log) = link(vec![Ok(reply(0x29, 450, 0x1504, None, b"Desk"))], 450);
        assert_eq!(
            link.query_with(2, 0x04, 0x1504, &0u32.to_le_bytes(), &Ctx::none())
                .unwrap(),
            b"Desk"
        );
        assert_eq!(
            log.lock().unwrap()[1],
            Call::Set(
                2,
                query_frame_with(0x04, 0x1504, &[0, 0, 0, 0], 450).to_vec()
            )
        );
    }

    #[test]
    fn a_cut_context_sends_nothing_and_takes_no_lock() {
        let (mut link, log) = link(vec![Ok([0u8; 60])], 7);
        let stopped = Arc::new(AtomicBool::new(true));
        let ctx = Ctx::interruptible(stopped);
        assert_eq!(
            link.short(6, &[0x1a, 0x01, 0x02], &ctx),
            Err(CameraError::Cut)
        );
        assert_eq!(link.query(2, 0x04, 0x0104, &ctx), Err(CameraError::Cut));
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn a_stop_between_the_frames_of_a_pair_keeps_the_second_frame_back() {
        let (mut link, log) = link(vec![Ok([0u8; 60])], 7);
        let flag = Arc::new(AtomicBool::new(false));
        let ctx = Ctx::interruptible(flag.clone());
        let first = frame(0x04, 0x3984, &[1, 0, 0, 0], 1);
        let second = frame(0x04, 0x3d84, &[0], 2);
        link.send(2, &first, &ctx).unwrap();
        flag.store(true, Ordering::SeqCst);
        assert_eq!(link.send(2, &second, &ctx), Err(CameraError::Cut));
        let sets = log
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, Call::Set(..)))
            .count();
        assert_eq!(sets, 1);
    }

    #[test]
    fn the_two_frames_of_a_pair_are_a_frame_gap_apart() {
        let (wire, log) = Wire::new(vec![Ok([0u8; 60])]);
        let mut link = Link::new(
            Box::new(wire),
            seq_at(1),
            Timing {
                reply_gap: Duration::ZERO,
                frame_gap: Duration::from_millis(30),
            },
        );
        let a = frame(0x04, 0x3984, &[1, 0, 0, 0], 1);
        let b = frame(0x04, 0x3d84, &[0], 2);
        let start = Instant::now();
        link.pair(2, &a, &b, &Ctx::none()).unwrap();
        assert!(start.elapsed() >= Duration::from_millis(30));
        let sets: Vec<Call> = log
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, Call::Set(..)))
            .cloned()
            .collect();
        assert_eq!(sets, [Call::Set(2, a.to_vec()), Call::Set(2, b.to_vec())]);
    }

    #[test]
    fn spaced_frames_keep_the_frame_gap_between_them() {
        let (wire, log) = Wire::new(vec![Ok([0u8; 60])]);
        let mut link = Link::new(
            Box::new(wire),
            seq_at(1),
            Timing {
                reply_gap: Duration::ZERO,
                frame_gap: Duration::from_millis(30),
            },
        );
        let x = frame(0x04, 0x0bc4, &0.1f32.to_le_bytes(), 1);
        let y = frame(0x04, 0x0c44, &0.1f32.to_le_bytes(), 2);
        let start = Instant::now();
        link.send_spaced(2, &x, &Ctx::none()).unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(30),
            "the first one does not wait"
        );
        link.send_spaced(2, &y, &Ctx::none()).unwrap();
        assert!(start.elapsed() >= Duration::from_millis(30));
        assert_eq!(
            log.lock()
                .unwrap()
                .iter()
                .filter(|c| matches!(c, Call::Set(..)))
                .count(),
            2
        );
    }

    #[test]
    fn controls_and_buffers_go_through_the_lock_too() {
        let block = [3u8; 60];
        let (mut link, log) = link(vec![Ok(block)], 7);
        assert_eq!(link.read(6, &Ctx::none()).unwrap(), block);
        link.write_back(0x0c, &block, &Ctx::none()).unwrap();
        assert_eq!(link.ctrl_get(0x0098_0900, &Ctx::none()).unwrap(), 7);
        link.ctrl_set(0x0098_0900, 5, &Ctx::none()).unwrap();
        assert_eq!(
            *log.lock().unwrap(),
            [
                Call::Hold,
                Call::Get(6),
                Call::Release,
                Call::Hold,
                Call::Set(0x0c, block.to_vec()),
                Call::Release,
                Call::Hold,
                Call::CtrlGet(0x0098_0900),
                Call::Release,
                Call::Hold,
                Call::CtrlSet(0x0098_0900, 5),
                Call::Release,
            ]
        );
    }

    #[test]
    fn one_counter_numbers_the_frames_of_the_process() {
        assert!(std::ptr::eq(process_sequence(), process_sequence()));
        let s = process_sequence();
        let a = s.next();
        assert_ne!(s.next(), a);
    }
}
