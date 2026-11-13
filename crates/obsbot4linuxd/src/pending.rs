// SPDX-License-Identifier: GPL-3.0-or-later

//! `pending` (4.1 rule 4, 5.6, 5.7): leaf path → the value written, while the camera accepted
//! the write and its readback does not show it yet. A path leaves `pending` when a read of its
//! group shows the written value (after rounding to the setting's precision), or when its
//! `settle_ms` has passed at a read of its group; then the camera's value stands. Nothing here
//! reads the camera: the checks run at the reads the poll plan and the write path make anyway.

use obsbot_proto::values::{number, rounded, same};
use obsbot_proto::{Setting, SettingType};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// A written value as the daemon publishes and compares it: in `pending`, as the state of a
/// `readback: false` leaf, and in the state a batch validates against (4.2 "Precision":
/// rounded where it is created, decode and echo). What is sent is [`sent`], never the echo. A
/// number is rounded to the setting's precision; without a
/// precision, a stepped number is snapped to its step counted from the minimum (the validator
/// accepts float noise such as `2.9999999999999996` on a stepped leaf); otherwise it is only
/// put in canonical form. `range`, `pose`, `point` and `box` go component by component.
/// Every other value is returned as it is.
pub fn echo(setting: &Setting, value: &Value) -> Value {
    let round = |x: &Value, component: Option<&str>| -> Value {
        let Some(f) = x.as_f64() else {
            return x.clone();
        };
        let get = |b: &Option<obsbot_proto::Bound>| b.as_ref().and_then(|b| b.get(component));
        if let Some(p) = get(&setting.precision) {
            return rounded(f, p);
        }
        if let Some(step) = get(&setting.step).filter(|st| *st > 0.0) {
            let base = get(&setting.min).unwrap_or(0.0);
            let snapped = base + ((f - base) / step).round() * step;
            return rounded(snapped, decimal_unit(step));
        }
        number(f)
    };
    match (&setting.kind, value) {
        (SettingType::Number, Value::Number(_)) => round(value, None),
        (kind, Value::Object(map)) if kind.components().is_some() => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), round(v, Some(k)));
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}

/// The unit of the last decimal of a step as written (`100` → 1, `0.1` → 0.1, `0.25` → 0.01).
fn decimal_unit(step: f64) -> f64 {
    let text = format!("{step}");
    let decimals = text.split_once('.').map_or(0, |(_, d)| d.len());
    10f64.powi(-(decimals as i32))
}

/// The value a request sends (section 2: exactly the value it names). The one change: a
/// stepped number the validator accepted lies within 1e-6 steps of a step
/// (`validate::check_number`), and it is sent on that step, so float noise such as
/// `2.9999999999999996` reaches the model as `3`, which an encoder of whole numbers takes.
/// Nothing is rounded to a precision here (that happens where values are created for the
/// state: decode and [`echo`], 4.2); a number without a step and every other value are sent
/// as they are. `range`, `pose`, `point` and `box` go component by component.
pub fn sent(setting: &Setting, value: &Value) -> Value {
    let snap = |x: &Value, component: Option<&str>| -> Value {
        let get = |b: &Option<obsbot_proto::Bound>| b.as_ref().and_then(|b| b.get(component));
        match (x.as_f64(), get(&setting.step).filter(|st| *st > 0.0)) {
            (Some(f), Some(step)) => {
                let base = get(&setting.min).unwrap_or(0.0);
                rounded(
                    base + ((f - base) / step).round() * step,
                    decimal_unit(step),
                )
            }
            _ => x.clone(),
        }
    };
    match (&setting.kind, value) {
        (SettingType::Number, Value::Number(_)) => snap(value, None),
        (kind, Value::Object(map)) if kind.components().is_some() => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), snap(v, Some(k))))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Whether a read value shows the written one, both rounded to the setting's precision.
pub fn shows(setting: &Setting, read: &Value, written: &Value) -> bool {
    same(&echo(setting, read), &echo(setting, written))
}

#[derive(Debug, Clone, PartialEq)]
struct Wait {
    value: Value,
    /// When the settle bound started: the write's acceptance, or (for a write whose group
    /// cannot be read while asleep) the first read of the group after it; `None` until then.
    since: Option<Instant>,
    settle: Duration,
}

/// The pending paths of one device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pending {
    waits: BTreeMap<String, Wait>,
}

impl Pending {
    pub fn new() -> Pending {
        Pending::default()
    }

    /// The `pending` map of the published state.
    pub fn map(&self) -> BTreeMap<String, Value> {
        self.waits
            .iter()
            .map(|(k, w)| (k.clone(), w.value.clone()))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.waits.is_empty()
    }

    pub fn contains(&self, leaf: &str) -> bool {
        self.waits.contains_key(leaf)
    }

    /// After the touched re-read of a write (5.6): `read` is the leaf's value from that re-read,
    /// `None` when its group was not read or the read failed. A value that shows the write
    /// leaves (or never enters) `pending`; otherwise the leaf enters it with `echo`, replacing an
    /// earlier pending value of the same path and restarting the bound. `settle_from` is the
    /// write's acceptance, or `None` when the group cannot be read now (asleep, 5.7).
    pub fn written(
        &mut self,
        setting: &Setting,
        leaf: &str,
        echo: Value,
        read: Option<&Value>,
        settle_from: Option<Instant>,
    ) {
        if read.is_some_and(|r| shows(setting, r, &echo)) {
            self.waits.remove(leaf);
            return;
        }
        let settle = Duration::from_millis(u64::from(setting.settle_ms.unwrap_or(0)));
        self.waits.insert(
            leaf.to_string(),
            Wait {
                value: echo,
                since: settle_from,
                settle,
            },
        );
    }

    /// A successful read of one group at `now`: `values` holds every leaf of the group. Each
    /// pending leaf of the group leaves when the read shows its value, or when its bound has
    /// passed; a bound that had not started starts with this read (5.7). Returns whether
    /// `pending` changed.
    pub fn group_read(
        &mut self,
        settings: &BTreeMap<String, Setting>,
        values: &BTreeMap<String, Value>,
        now: Instant,
    ) -> bool {
        let before = self.waits.len();
        self.waits.retain(|leaf, wait| {
            let Some(read) = values.get(leaf) else {
                return true;
            };
            let Some(setting) = settings.get(leaf) else {
                return false;
            };
            let since = *wait.since.get_or_insert(now);
            if shows(setting, read, &wait.value) {
                return false;
            }
            now.duration_since(since) < wait.settle
        });
        before != self.waits.len()
    }

    /// The device became unavailable: `pending` is emptied (4.1 rule 4).
    pub fn clear(&mut self) {
        self.waits.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::SettingType;
    use serde_json::json;

    fn led() -> Setting {
        Setting::number("LED", 0.0, 3.0).step(1.0).settle_ms(1000)
    }

    fn zoom() -> Setting {
        Setting::number("Zoom", 1.0, 4.0)
            .precision(0.01)
            .settle_ms(3000)
    }

    fn settings() -> BTreeMap<String, Setting> {
        [
            ("led.brightness".to_string(), led()),
            ("zoom.factor".to_string(), zoom()),
        ]
        .into()
    }

    fn read(leaf: &str, v: Value) -> BTreeMap<String, Value> {
        [(leaf.to_string(), v)].into()
    }

    #[test]
    fn echo_rounds_to_the_precision_and_writes_integral_numbers_without_a_fraction() {
        assert_eq!(echo(&zoom(), &json!(1.5000000001)), json!(1.5));
        assert_eq!(echo(&zoom(), &json!(2.0)).to_string(), "2");
        assert_eq!(
            echo(&led(), &json!(2.0)).to_string(),
            "2",
            "no precision: canonical"
        );
        let pose = Setting::composite(
            SettingType::Pose,
            "Pose",
            &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
            &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
        )
        .precisions(&[("pan", 0.1), ("tilt", 0.1), ("zoom", 0.01)]);
        assert_eq!(
            echo(
                &pose,
                &json!({"pan": 10.04, "tilt": -0.3000001, "zoom": 1.234})
            ),
            json!({"pan": 10, "tilt": -0.3, "zoom": 1.23})
        );
        assert_eq!(echo(&Setting::boolean("B"), &json!(true)), json!(true));
    }

    /// Section 2 and review focus 1: a request sends the value it names; only float noise
    /// within the validator's step tolerance is put on the step, and nothing is rounded to a
    /// precision before it is sent.
    #[test]
    fn sent_keeps_the_value_and_only_puts_float_noise_on_its_step() {
        assert_eq!(sent(&led(), &json!(2.9999999999999996)).to_string(), "3");
        assert_eq!(sent(&led(), &json!(2)), json!(2), "on the step: unchanged");
        let tenths = Setting::number("Ratio", 1.0, 4.0).step(0.1);
        assert_eq!(sent(&tenths, &json!(0.1 + 0.2 + 1.0)).to_string(), "1.3");
        assert_eq!(
            sent(&zoom(), &json!(2.345)),
            json!(2.345),
            "no step: not rounded to the precision"
        );
        let pose = Setting::composite(
            SettingType::Pose,
            "Pose",
            &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
            &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
        )
        .precisions(&[("pan", 0.1), ("tilt", 0.1), ("zoom", 0.01)]);
        let named = json!({"pan": 10.04, "tilt": -0.3000001, "zoom": 1.234});
        assert_eq!(sent(&pose, &named), named);
        assert_eq!(sent(&Setting::boolean("B"), &json!(true)), json!(true));
    }

    /// Review focus 1: the echo of a slider's float noise on a stepped leaf without a
    /// precision is on the step, so the readback 3 ends `pending` at once instead of after the
    /// settle bound.
    #[test]
    fn echo_snaps_float_noise_to_the_step_when_there_is_no_precision() {
        let stepped = Setting::number("Times", 1.0, 3.0).step(1.0).settle_ms(1000);
        assert_eq!(echo(&stepped, &json!(2.9999999999999996)).to_string(), "3");
        let hundreds = Setting::number("Temperature", 2000.0, 10000.0).step(100.0);
        assert_eq!(echo(&hundreds, &json!(5000.000000001)).to_string(), "5000");
        let tenths = Setting::number("Ratio", 1.0, 4.0).step(0.1);
        assert_eq!(echo(&tenths, &json!(0.1 + 0.2 + 1.0)).to_string(), "1.3");
        let quarters = Setting::number("Q", 0.0, 1.0).step(0.25);
        assert_eq!(echo(&quarters, &json!(0.7500000001)).to_string(), "0.75");
        let mut p = Pending::new();
        p.written(
            &stepped,
            "beacon.times",
            echo(&stepped, &json!(2.9999999999999996)),
            Some(&json!(3)),
            Some(Instant::now()),
        );
        assert!(p.is_empty(), "the readback shows it");
    }

    /// 4.1 rules 3 and 4: the re-read shows the write: nothing pending.
    #[test]
    fn a_write_the_reread_shows_is_not_pending() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), Some(&json!(2)), Some(t));
        assert!(p.is_empty());
        p.written(
            &zoom(),
            "zoom.factor",
            json!(1.5),
            Some(&json!(1.504)),
            Some(t),
        );
        assert!(p.is_empty(), "equal after rounding to 0.01");
    }

    /// The golden session: the re-read does not show it yet, the next read of the group does.
    #[test]
    fn a_lagging_readback_is_pending_until_a_read_shows_it() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), Some(&json!(1)), Some(t));
        assert_eq!(p.map(), [("led.brightness".to_string(), json!(2))].into());
        assert!(!p.group_read(&settings(), &read("led.brightness", json!(1)), t));
        assert!(p.group_read(
            &settings(),
            &read("led.brightness", json!(2)),
            t + Duration::from_millis(300)
        ));
        assert!(p.is_empty());
    }

    /// A camera that rounds the value (fake-b's exposure time) never shows it: the bound ends it,
    /// at a read of its group, and the camera's value stands.
    #[test]
    fn the_settle_bound_ends_pending_at_a_read_of_the_group() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(3), None, Some(t));
        let other = read("led.brightness", json!(1));
        assert!(!p.group_read(&settings(), &other, t + Duration::from_millis(999)));
        assert!(
            !p.group_read(
                &settings(),
                &read("zoom.factor", json!(1)),
                t + Duration::from_secs(5)
            ),
            "a read of another group does not end it"
        );
        assert!(p.contains("led.brightness"));
        assert!(p.group_read(&settings(), &other, t + Duration::from_millis(1000)));
        assert!(p.is_empty());
    }

    /// 5.7: written while asleep, its group unreadable asleep: the bound starts at the first
    /// read after wake, however long the camera slept.
    #[test]
    fn a_bound_that_had_not_started_starts_at_the_first_read() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(3), None, None);
        let wake = t + Duration::from_secs(600);
        assert!(!p.group_read(&settings(), &read("led.brightness", json!(0)), wake));
        assert!(
            p.contains("led.brightness"),
            "the bound starts now, it has not passed"
        );
        assert!(p.group_read(
            &settings(),
            &read("led.brightness", json!(0)),
            wake + Duration::from_millis(1000)
        ));
    }

    /// 4.1 rule 4: a new write of the same path replaces its pending value and restarts the bound.
    #[test]
    fn a_new_write_replaces_the_value_and_restarts_the_bound() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), None, Some(t));
        let later = t + Duration::from_millis(900);
        p.written(&led(), "led.brightness", json!(3), None, Some(later));
        assert_eq!(p.map()["led.brightness"], json!(3));
        assert!(!p.group_read(
            &settings(),
            &read("led.brightness", json!(0)),
            t + Duration::from_millis(1500)
        ));
        assert!(
            p.contains("led.brightness"),
            "1000 ms count from the second write"
        );
    }

    #[test]
    fn a_write_the_reread_shows_removes_an_older_pending_value() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), None, Some(t));
        p.written(&led(), "led.brightness", json!(1), Some(&json!(1)), Some(t));
        assert!(p.is_empty());
    }

    #[test]
    fn unavailable_empties_pending() {
        let mut p = Pending::new();
        p.written(&led(), "led.brightness", json!(2), None, None);
        p.clear();
        assert!(p.is_empty());
    }
}
