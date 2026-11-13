// SPDX-License-Identifier: GPL-3.0-or-later

//! The small rules inside the contract types: registry lookup (8.1), the operation context
//! (4.1 rule 9, 5.6), tiers (5.7) and the send table (8.3).

use obsbot_device::contract::camera::Result as CamResult;
use obsbot_device::contract::{
    CameraError, ControlRange, Ctx, Frame, IdentitySource, Lookup, ModelEntry, OnAir, Registry,
    SendEntry, SendTable, Tier, Transport, UsbId,
};
use obsbot_proto::{Support, Usb};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const A: ModelEntry = ModelEntry {
    key: "model-a",
    display_name: "Model A",
    usb: &[
        UsbId {
            vendor: 0x3564,
            product: 0x0001,
        },
        UsbId {
            vendor: 0x3564,
            product: 0x0002,
        },
    ],
    support: Support::Tested,
    identity: IdentitySource::UsbSerial,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: Some("00000000-0000-0000-0000-000000000001"),
    selectors: &[2, 6],
};
const B: ModelEntry = ModelEntry {
    key: "model-b",
    display_name: "Model B",
    usb: &[UsbId {
        vendor: 0x3564,
        product: 0x0002,
    }],
    support: Support::Untested,
    identity: IdentitySource::Absent,
    firmware_query: false,
    identity_tier: Tier::FiveSeconds,
    xu_guid: None,
    selectors: &[],
};
static ONE: Registry = Registry {
    vendors: &[0x3564],
    models: &[A],
};
static CLASH: Registry = Registry {
    vendors: &[0x3564],
    models: &[A, B],
};

#[test]
fn lookup_uses_the_vendor_table_and_the_usb_pairs_only() {
    assert!(
        matches!(
            ONE.lookup(UsbId {
                vendor: 0x046d,
                product: 0x0001
            }),
            Lookup::NotListed
        ),
        "unknown vendor: not listed (5.3)"
    );
    assert!(
        matches!(
            ONE.lookup(UsbId {
                vendor: 0x3564,
                product: 0x0009
            }),
            Lookup::Unsupported
        ),
        "known vendor, unknown product (5.4)"
    );
    assert!(
        matches!(ONE.lookup(UsbId { vendor: 0x3564, product: 0x0002 }), Lookup::Model(m) if m.key == "model-a"),
        "a model may have several pairs"
    );
}

#[test]
fn a_pair_of_two_models_is_found() {
    assert!(ONE.shared_pairs().is_empty());
    assert_eq!(
        CLASH.shared_pairs(),
        [UsbId {
            vendor: 0x3564,
            product: 0x0002
        }]
    );
}

#[test]
fn usb_ids_come_from_the_hex_strings_of_sysfs() {
    let usb = Usb {
        vendor: "3564".into(),
        product: "fef9".into(),
        product_name: None,
        bcd_device: "0100".into(),
        path: "3-2.1".into(),
        serial_present: false,
        interfaces: vec![],
        extension_units: vec![],
    };
    assert_eq!(
        UsbId::from_usb(&usb),
        Some(UsbId {
            vendor: 0x3564,
            product: 0xfef9
        })
    );
    assert_eq!(
        UsbId::from_usb(&Usb {
            vendor: "zz".into(),
            ..usb
        }),
        None
    );
}

#[test]
fn a_context_cuts_at_its_deadline_or_when_interrupted() {
    assert!(Ctx::none().check().is_ok());
    assert!(
        Ctx::until(Instant::now() + Duration::from_secs(60))
            .check()
            .is_ok()
    );
    assert!(
        Ctx::until(Instant::now() - Duration::from_millis(1))
            .check()
            .is_err(),
        "deadline passed"
    );
    let flag = Arc::new(AtomicBool::new(false));
    let ctx = Ctx::interruptible(flag.clone());
    assert!(ctx.check().is_ok());
    flag.store(true, Ordering::SeqCst);
    assert!(ctx.check().is_err(), "a stop arrived");

    // A request's touched re-read: under the request's deadline and cut by a stop (5.6).
    let flag = Arc::new(AtomicBool::new(false));
    let both = Ctx::new(
        Some(Instant::now() + Duration::from_secs(60)),
        Some(flag.clone()),
    );
    assert!(both.check().is_ok());
    flag.store(true, Ordering::SeqCst);
    assert!(
        both.check().is_err(),
        "the stop cuts it before the deadline"
    );
    let late = Ctx::new(
        Some(Instant::now() - Duration::from_millis(1)),
        Some(Arc::new(AtomicBool::new(false))),
    );
    assert!(late.check().is_err(), "the deadline cuts it without a stop");
}

#[test]
fn tiers_and_send_tables() {
    assert_eq!(Tier::Second.period(), Some(Duration::from_secs(1)));
    assert_eq!(Tier::FiveSeconds.period(), Some(Duration::from_secs(5)));
    assert_eq!(Tier::Minute.period(), Some(Duration::from_secs(60)));
    assert_eq!(Tier::Touched.period(), None, "never periodic");
    let table = SendTable {
        entries: vec![SendEntry {
            frame: Frame::Short {
                selector: 6,
                command: 0x1a,
            },
            evidence: "capture 07",
            identity: false,
        }],
    };
    assert!(table.allows(&Frame::Short {
        selector: 6,
        command: 0x1a
    }));
    assert!(!table.allows(&Frame::Short {
        selector: 6,
        command: 0x1b
    }));
    assert!(!SendTable::empty().allows(&Frame::Uvc {
        control: 0x0098_0900
    }));
}

#[test]
fn queries_and_block_writes_are_frames_of_their_own() {
    // A query (frame type 0x01 or 0x21) and a command (type 0x25) with the same receiver and
    // id are two entries: allowing the query must not allow a write with that id (8.3).
    let query = Frame::Query {
        selector: 2,
        receiver: 0x0d,
        command: 0x0408,
    };
    let command = Frame::Framed {
        selector: 2,
        receiver: 0x0d,
        command: 0x0408,
    };
    let table = SendTable {
        entries: vec![
            SendEntry {
                frame: query,
                evidence: "capture 01",
                identity: true,
            },
            SendEntry {
                frame: Frame::Block { selector: 0x0c },
                evidence: "measured 2026-10-31 (#104)",
                identity: false,
            },
        ],
    };
    assert!(table.allows(&query));
    assert!(!table.allows(&command), "a query entry allows no command");
    assert!(table.allows(&Frame::Block { selector: 0x0c }));
    assert!(!table.allows(&Frame::Block { selector: 0x0d }));
}

/// A transport that does not override `hold` and `release`.
struct Plain;

impl Transport for Plain {
    fn xu_len(&mut self, _selector: u8) -> CamResult<u16> {
        Ok(60)
    }
    fn xu_get(&mut self, _selector: u8, _buf: &mut [u8]) -> CamResult<()> {
        Ok(())
    }
    fn xu_set(&mut self, _selector: u8, _data: &[u8]) -> CamResult<()> {
        Ok(())
    }
    fn ctrl_get(&mut self, _id: u32) -> CamResult<i32> {
        Ok(0)
    }
    fn ctrl_set(&mut self, _id: u32, _value: i32) -> CamResult<()> {
        Ok(())
    }
    fn ctrl_range(&mut self, _id: u32) -> CamResult<Option<ControlRange>> {
        Ok(None)
    }
    fn usb(&mut self) -> CamResult<Usb> {
        Err(CameraError::Gone)
    }
    fn usb_serial(&mut self) -> CamResult<Option<String>> {
        Ok(None)
    }
    fn on_air(&mut self) -> CamResult<OnAir> {
        Ok(OnAir::default())
    }
}

#[test]
fn a_transport_without_a_lock_holds_and_releases_without_effect() {
    let mut t = Plain;
    assert!(t.hold().is_ok());
    t.release();
    t.release();
    assert!(t.hold().is_ok(), "a second hold after release works too");
}
