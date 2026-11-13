// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance check itself: a small in-memory camera passes, and each kind of flaw it
//! must catch makes it fail. (The fakes are checked in tests/fakes.rs.)

use obsbot_device::contract::conformance::check;
use obsbot_device::contract::{
    Camera, CameraError, Ctx, Group, Identity, Motion, Need, SendTable, Target, Tier,
};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Action, Asleep, Description, Params, Setting, State, Values};
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq)]
enum Flaw {
    None,
    WritableInTouchedGroup,
    TouchedLacksOwnGroup,
    ReadsOutOfRange,
    OrderIgnoresRequires,
    LeafInNoGroup,
    ContinuingWithoutStop,
}

struct Mini {
    desc: Description,
    plan: Vec<Group>,
    values: Values,
    flaw: Flaw,
}

fn group(name: &str, tier: Tier, asleep: bool, leaves: &[&str]) -> Group {
    Group {
        name: name.into(),
        tier,
        asleep,
        live: None,
        leaves: leaves.iter().map(|l| l.to_string()).collect(),
    }
}

impl Mini {
    fn new(flaw: Flaw) -> Mini {
        let w = |s: Setting| s.asleep(Asleep::Unknown).settle_ms(3000);
        let desc = Description::default()
            .setting("asleep", Setting::boolean("Asleep").readonly())
            .setting(
                "led.brightness",
                w(Setting::number("LED brightness", 0.0, 3.0).step(1.0)),
            )
            .setting(
                "led.blink",
                Setting::boolean("LED blink")
                    .asleep(Asleep::Unknown)
                    .no_readback(),
            )
            .setting(
                "exposure.mode",
                w(Setting::enumeration("Exposure mode", &["auto", "manual"])),
            )
            .setting(
                "exposure.time",
                w(Setting::number("Exposure time", 100.0, 1000.0))
                    .requires("exposure.mode", json!("manual")),
            )
            .action("gimbal.turn", Action::new("Turn", Asleep::Unknown))
            .action("gimbal.stop", Action::new("Stop", Asleep::Unknown));
        let led_tier = if flaw == Flaw::WritableInTouchedGroup {
            Tier::Touched
        } else {
            Tier::Second
        };
        let mut plan = vec![
            group("status", Tier::Second, true, &["asleep"]),
            group("led", led_tier, false, &["led.brightness"]),
            group(
                "exposure",
                Tier::FiveSeconds,
                false,
                &["exposure.mode", "exposure.time"],
            ),
        ];
        if flaw == Flaw::LeafInNoGroup {
            plan[2].leaves.pop();
        }
        let values = [
            ("asleep", json!(false)),
            ("led.brightness", json!(1)),
            ("exposure.mode", json!("auto")),
            ("exposure.time", json!(300)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        Mini {
            desc,
            plan,
            values,
            flaw,
        }
    }
}

impl Camera for Mini {
    fn description(&self) -> &Description {
        &self.desc
    }
    fn identity(&mut self, _need: Need, _ctx: &Ctx) -> Result<Identity, CameraError> {
        Ok(Identity::default())
    }
    fn poll_plan(&self) -> &[Group] {
        &self.plan
    }
    fn read(&mut self, name: &str, _ctx: &Ctx) -> Result<Values, CameraError> {
        let g = self
            .plan
            .iter()
            .find(|g| g.name == name)
            .ok_or(CameraError::NoAnswer(name.into()))?;
        let mut out: Values = g
            .leaves
            .iter()
            .map(|l| (l.clone(), self.values[l].clone()))
            .collect();
        if self.flaw == Flaw::ReadsOutOfRange && name == "led" {
            out.insert("led.brightness".into(), json!(9));
        }
        Ok(out)
    }
    fn order(&self, leaves: &[&str]) -> Vec<String> {
        let mut out: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
        out.sort_by_key(|l| if l == "exposure.mode" { 0 } else { 1 });
        if self.flaw == Flaw::OrderIgnoresRequires {
            out.reverse();
        }
        out
    }
    fn apply(&mut self, leaf: &str, value: &Value, _ctx: &Ctx) -> Result<(), CameraError> {
        self.values.insert(leaf.into(), value.clone());
        Ok(())
    }
    fn check(&self, _action: &str, _params: &Params, _state: &State) -> Result<(), Refusal> {
        Ok(())
    }
    fn act(
        &mut self,
        _action: &str,
        _params: &Params,
        _state: &State,
        _ctx: &Ctx,
    ) -> Result<Option<Params>, CameraError> {
        Ok(None)
    }
    fn touched(&self, target: Target<'_>) -> Vec<String> {
        match target {
            Target::Leaf(_) if self.flaw == Flaw::TouchedLacksOwnGroup => vec![],
            Target::Leaf(leaf) => self
                .plan
                .iter()
                .filter(|g| g.leaves.iter().any(|l| l == leaf))
                .map(|g| g.name.clone())
                .collect(),
            Target::Action(_) => vec![],
        }
    }
    fn motion(&self, action: &str) -> Motion {
        match action {
            "gimbal.turn" => Motion::Continuing {
                stop: "gimbal.stop",
            },
            "gimbal.stop" if self.flaw == Flaw::ContinuingWithoutStop => Motion::Move,
            "gimbal.stop" => Motion::Stop,
            _ => Motion::None,
        }
    }
    fn send_table(&self) -> &SendTable {
        static EMPTY: SendTable = SendTable {
            entries: Vec::new(),
        };
        &EMPTY
    }
}

#[test]
fn a_conforming_camera_passes() {
    assert_eq!(check(&mut Mini::new(Flaw::None)), Vec::<String>::new());
}

#[test]
fn each_flaw_is_caught() {
    let rows = [
        (
            Flaw::WritableInTouchedGroup,
            "led.brightness: writable with a readback, so its group needs a periodic tier",
        ),
        (
            Flaw::TouchedLacksOwnGroup,
            "led.brightness: touched [] lacks its own group led",
        ),
        (Flaw::ReadsOutOfRange, "led.brightness: read 9 does not fit"),
        (Flaw::OrderIgnoresRequires, "exposure.time: order"),
        (
            Flaw::LeafInNoGroup,
            "exposure.time: has a readback but is in no group",
        ),
        (
            Flaw::ContinuingWithoutStop,
            "gimbal.turn: continuing, but its stop gimbal.stop is not a stop",
        ),
    ];
    for (flaw, expected) in rows {
        let problems = check(&mut Mini::new(flaw)).join("\n");
        assert!(
            problems.contains(expected),
            "expected {expected:?} in:\n{problems}"
        );
    }
}
