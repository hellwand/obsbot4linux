// SPDX-License-Identifier: GPL-3.0-or-later

//! The in-memory camera behind both fake models. A [`Shape`] holds what differs between
//! them; [`Dev`] is one fake device, shared with the fake bus that plugs and controls it.

use crate::contract::{
    Camera, CameraError, Ctx, Group, Identity, Motion, Need, SendTable, Target, Tier,
};
use crate::models::fake::RESTART;
use crate::models::fake::bus::Op;
use obsbot_proto::validate::Refusal;
use obsbot_proto::values::{number, same};
use obsbot_proto::{Description, Params, Setting, SettingType, State, Usb, Values, Warning};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// What one fake model is.
pub(crate) struct Shape {
    pub key: &'static str,
    pub description: Description,
    pub plan: Vec<Group>,
    pub initial: Values,
    pub serial_prefix: &'static str,
    pub firmware: &'static str,
    /// The motion actions and stops; every other action is `Motion::None`.
    pub motion: &'static [(&'static str, Motion)],
    /// Groups touched in addition to a written leaf's own group, and by each action.
    pub touched: fn(Target<'_>) -> Vec<&'static str>,
    /// Leaves whose readback shows a write only after this time.
    pub lag: &'static [(&'static str, Duration)],
    /// Leaves the camera rounds down to a step, so the readback never shows some writes.
    pub floor: &'static [(&'static str, f64)],
    /// The model's actions other than `sleep`, `wake` and the generic restart and reset.
    pub act: fn(&mut Values, &str, &Params) -> Option<Params>,
    pub check: fn(&str, &Params, &State) -> Result<(), Refusal>,
}

/// One fake device.
pub(crate) struct Dev {
    pub shape: Option<&'static Shape>,
    pub serial: Option<String>,
    pub usb: Usb,
    pub video_node: String,
    pub devnum: u32,
    pub plugged: bool,
    pub restart_until: Option<Instant>,
    pub asleep: bool,
    pub fail_next: bool,
    pub delay: Duration,
    /// What the camera stores, leaf path → value.
    pub values: Values,
    /// Writes not visible yet: leaf path → (value, visible from).
    pub lagging: BTreeMap<String, (Value, Instant)>,
    pub ops: Vec<Op>,
}

impl Dev {
    pub fn id_matches(&self, id: &str) -> bool {
        match (id.strip_prefix("sn:"), id.strip_prefix("port:")) {
            (Some(serial), _) => self.serial.as_deref() == Some(serial),
            (_, Some(path)) => self.usb.path == path,
            _ => false,
        }
    }
}

pub(crate) fn lock(dev: &Mutex<Dev>) -> MutexGuard<'_, Dev> {
    dev.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A first value for every leaf with a readback: `null` where nullable, else the default,
/// the first enum value, `false`, 0 clamped into the bounds, the full range, an empty list.
pub(crate) fn initial_values(desc: &Description) -> Values {
    desc.settings
        .iter()
        .filter(|(_, s)| s.readback)
        .map(|(path, s)| (path.clone(), sample(s)))
        .collect()
}

fn clamp0(s: &Setting, component: Option<&str>) -> f64 {
    let lo = s
        .min
        .as_ref()
        .and_then(|b| b.get(component))
        .unwrap_or(f64::MIN);
    let hi = s
        .max
        .as_ref()
        .and_then(|b| b.get(component))
        .unwrap_or(f64::MAX);
    0f64.clamp(lo, hi)
}

fn sample(s: &Setting) -> Value {
    if s.nullable {
        return Value::Null;
    }
    if let Some(d) = &s.default {
        return d.clone();
    }
    match &s.kind {
        SettingType::Bool => json!(false),
        SettingType::Enum => json!(s.values[0]),
        SettingType::Number => number(clamp0(s, None)),
        SettingType::String => json!(""),
        SettingType::List => json!([]),
        SettingType::Range => {
            let lo = s
                .min
                .as_ref()
                .and_then(|b| b.get(Some("min")))
                .unwrap_or(0.0);
            let hi = s
                .max
                .as_ref()
                .and_then(|b| b.get(Some("max")))
                .unwrap_or(lo);
            json!({"min": number(lo), "max": number(hi)})
        }
        kind => {
            let mut m = Map::new();
            for c in kind.components().unwrap_or(&[]) {
                m.insert(c.to_string(), number(clamp0(s, Some(c))));
            }
            Value::Object(m)
        }
    }
}

/// (group name, tier, readable asleep, live topic and its faster tier).
pub(crate) type Rule<'a> = (&'a str, Tier, bool, Option<(&'a str, Tier)>);

/// Builds a poll plan: one group per first path segment, `status` for `asleep`. `rules`
/// gives (segment, tier, readable asleep, live topic and its tier); other segments get
/// `default`. Leaves without a readback are in no group.
pub(crate) fn plan_by_segment(desc: &Description, rules: &[Rule<'_>], default: Tier) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for path in desc
        .settings
        .iter()
        .filter(|(_, s)| s.readback)
        .map(|(p, _)| p)
    {
        let segment = path.split('.').next().unwrap_or(path);
        let name = if segment == "asleep" {
            "status"
        } else {
            segment
        };
        if let Some(g) = groups.iter_mut().find(|g| g.name == name) {
            g.leaves.push(path.clone());
            continue;
        }
        let rule = rules.iter().find(|r| r.0 == name);
        groups.push(Group {
            name: name.to_string(),
            tier: rule.map_or(default, |r| r.1),
            asleep: rule.is_some_and(|r| r.2),
            live: rule
                .and_then(|r| r.3)
                .map(|(topic, tier)| crate::contract::LiveTier {
                    topic: topic.into(),
                    tier,
                }),
            leaves: vec![path.clone()],
        });
    }
    groups
}

/// The live pose `{pan, tilt, zoom}` from the stored values.
pub(crate) fn live_pose(values: &Values) -> Value {
    json!({
        "pan": values.get("gimbal.pan").cloned().unwrap_or(json!(0)),
        "tilt": values.get("gimbal.tilt").cloned().unwrap_or(json!(0)),
        "zoom": values.get("zoom.factor").cloned().unwrap_or(json!(1)),
    })
}

/// `conflict` unless `list` was read and has an item whose `key` equals `value`.
pub(crate) fn needs_item(
    state: &State,
    list: &str,
    key: &str,
    value: Option<&Value>,
) -> Result<(), Refusal> {
    if state.unread.iter().any(|p| p == list) {
        return Err(Refusal::new(
            obsbot_proto::ErrorCode::Conflict,
            list,
            &format!("{list} was not read yet"),
        ));
    }
    let Some(value) = value else { return Ok(()) };
    let found = state
        .at(list)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|i| i.get(key).is_some_and(|v| same(v, value)))
        });
    if found {
        Ok(())
    } else {
        Err(Refusal::new(
            obsbot_proto::ErrorCode::Conflict,
            key,
            &format!("no item of {list} has {key} {value}"),
        ))
    }
}

pub(crate) struct FakeCamera {
    pub shape: &'static Shape,
    pub dev: Arc<Mutex<Dev>>,
    pub devnum: u32,
}

impl FakeCamera {
    fn gone(&self, d: &Dev) -> bool {
        !d.plugged || d.restart_until.is_some() || d.devnum != self.devnum
    }

    /// Records the operation, waits the model's delay (cut by `ctx` like a frame boundary),
    /// then fails once if `fail_next` is set.
    fn begin(&self, op: Op, ctx: &Ctx) -> Result<MutexGuard<'_, Dev>, CameraError> {
        let delay = {
            let mut d = lock(&self.dev);
            if self.gone(&d) {
                return Err(CameraError::Gone);
            }
            d.ops.push(op);
            d.delay
        };
        let end = Instant::now() + delay;
        loop {
            ctx.check()?;
            let now = Instant::now();
            if now >= end {
                break;
            }
            std::thread::sleep((end - now).min(Duration::from_millis(10)));
        }
        let mut d = lock(&self.dev);
        if self.gone(&d) {
            return Err(CameraError::Gone);
        }
        if d.fail_next {
            d.fail_next = false;
            return Err(CameraError::NoAnswer(
                "the next operation (fail_next)".into(),
            ));
        }
        Ok(d)
    }

    fn restart(&self, d: &mut Dev) {
        d.restart_until = Some(Instant::now() + RESTART);
        d.lagging.clear();
    }
}

impl Camera for FakeCamera {
    fn description(&self) -> &Description {
        &self.shape.description
    }

    fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity, CameraError> {
        let d = self.begin(Op::Identity(need), ctx)?;
        Ok(Identity {
            serial: if need.serial { d.serial.clone() } else { None },
            firmware: need.firmware.then(|| self.shape.firmware.to_string()),
        })
    }

    fn poll_plan(&self) -> &[Group] {
        &self.shape.plan
    }

    fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values, CameraError> {
        let mut d = self.begin(Op::Read(group.to_string()), ctx)?;
        let Some(g) = self.shape.plan.iter().find(|g| g.name == group) else {
            return Err(CameraError::NoAnswer(format!("the unknown group {group}")));
        };
        if d.asleep && !g.asleep {
            return Err(CameraError::NoAnswer(format!("group {group} while asleep")));
        }
        let now = Instant::now();
        let due: Vec<String> = d
            .lagging
            .iter()
            .filter(|(_, (_, at))| *at <= now)
            .map(|(k, _)| k.clone())
            .collect();
        for leaf in due {
            if let Some((value, _)) = d.lagging.remove(&leaf) {
                d.values.insert(leaf, value);
            }
        }
        let asleep = d.asleep;
        Ok(g.leaves
            .iter()
            .map(|leaf| {
                let value = if leaf == "asleep" {
                    json!(asleep)
                } else {
                    d.values.get(leaf).cloned().unwrap_or(Value::Null)
                };
                (leaf.clone(), value)
            })
            .collect())
    }

    fn order(&self, leaves: &[&str]) -> Vec<String> {
        let required_later = |leaf: &str| {
            let reqs = &self
                .shape
                .description
                .settings
                .get(leaf)
                .map(|s| s.requires.clone())
                .unwrap_or_default();
            reqs.iter().any(|r| leaves.contains(&r.path.as_str()))
        };
        let mut out: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
        out.sort_by_key(|l| required_later(l));
        out
    }

    fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<(), CameraError> {
        let mut d = self.begin(Op::Apply(leaf.to_string(), value.clone()), ctx)?;
        let mut stored = value.clone();
        if let Some((_, step)) = self.shape.floor.iter().find(|(l, _)| *l == leaf)
            && let Some(x) = value.as_f64()
        {
            stored = number((x / step).floor() * step);
        }
        match self.shape.lag.iter().find(|(l, _)| *l == leaf) {
            Some((_, lag)) => {
                d.lagging
                    .insert(leaf.to_string(), (stored, Instant::now() + *lag));
            }
            None => {
                d.values.insert(leaf.to_string(), stored);
            }
        }
        let restarts = self
            .shape
            .description
            .settings
            .get(leaf)
            .is_some_and(|s| s.warning.contains(&Warning::Restart));
        if restarts {
            self.restart(&mut d);
        }
        Ok(())
    }

    fn check(&self, action: &str, params: &Params, state: &State) -> Result<(), Refusal> {
        (self.shape.check)(action, params, state)
    }

    fn act(
        &mut self,
        action: &str,
        params: &Params,
        _state: &State,
        ctx: &Ctx,
    ) -> Result<Option<Params>, CameraError> {
        let mut d = self.begin(Op::Act(action.to_string(), params.clone()), ctx)?;
        let warning = self
            .shape
            .description
            .actions
            .get(action)
            .map(|a| a.warning.clone())
            .unwrap_or_default();
        let result = match action {
            "sleep" => {
                d.asleep = true;
                None
            }
            "wake" => {
                d.asleep = false;
                None
            }
            _ => (self.shape.act)(&mut d.values, action, params),
        };
        if warning.contains(&Warning::Destructive) {
            d.values = self.shape.initial.clone();
        }
        if warning.contains(&Warning::Restart) {
            self.restart(&mut d);
        }
        Ok(result)
    }

    fn touched(&self, target: Target<'_>) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Target::Leaf(leaf) = target
            && let Some(g) = self
                .shape
                .plan
                .iter()
                .find(|g| g.leaves.iter().any(|l| l == leaf))
        {
            out.push(g.name.clone());
        }
        for g in (self.shape.touched)(target) {
            if !out.iter().any(|o| o == g) {
                out.push(g.to_string());
            }
        }
        out
    }

    fn motion(&self, action: &str) -> Motion {
        self.shape
            .motion
            .iter()
            .find(|(a, _)| *a == action)
            .map_or(Motion::None, |(_, m)| *m)
    }

    fn send_table(&self) -> &SendTable {
        static EMPTY: SendTable = SendTable {
            entries: Vec::new(),
        };
        &EMPTY
    }
}
