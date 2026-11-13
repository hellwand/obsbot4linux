// SPDX-License-Identifier: GPL-3.0-or-later

//! The poll schedule of one device (5.7). Each periodic tier ticks on its own: its interval
//! counts from the end of its previous tick, so a slow read never starves the 1 s tier. A
//! tick reads the groups whose tier is that tier, or whose live tier is that tier while a
//! connection subscribes the group's live topic. While the camera is asleep a tick reads only
//! the groups readable asleep. A `touched` group is never read by a tick, except a `touched`
//! group whose last read failed or was cut short: it is read with the next tick of the 60 s
//! tier, or at the next wake, whichever comes first.

use crate::Timing;
use obsbot_device::contract::{Group, Tier};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

/// The periodic tiers, fastest first.
pub const PERIODIC: [Tier; 3] = [Tier::Second, Tier::FiveSeconds, Tier::Minute];

/// The tier a group is read in now: its live tier while a connection subscribes its topic.
pub fn effective(group: &Group, topics: &BTreeSet<String>) -> Tier {
    match &group.live {
        Some(live) if topics.contains(&live.topic) => live.tier,
        _ => group.tier,
    }
}

/// When each periodic tier is due next, and which `touched` groups wait for a retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    next: BTreeMap<Tier, Instant>,
    retry: BTreeSet<String>,
}

impl Schedule {
    /// The schedule after the first read attempt, which ended at `end`: every periodic tier
    /// ticks one period later.
    pub fn new(end: Instant, timing: &Timing) -> Schedule {
        let next = PERIODIC
            .iter()
            .map(|t| (*t, end + timing.period(*t).expect("periodic")))
            .collect();
        Schedule {
            next,
            retry: BTreeSet::new(),
        }
    }

    /// The tier to tick now: the one due longest ago, the faster one on a tie; `None` when no
    /// tier is due.
    pub fn due(&self, now: Instant) -> Option<Tier> {
        PERIODIC
            .iter()
            .filter(|t| self.next[t] <= now)
            .min_by_key(|t| self.next[t])
            .copied()
    }

    /// When the next tier is due.
    pub fn next_due(&self) -> Instant {
        *self.next.values().min().expect("three tiers")
    }

    /// The groups one tick of `tier` reads, in poll-plan order.
    pub fn groups(
        &self,
        plan: &[Group],
        tier: Tier,
        topics: &BTreeSet<String>,
        asleep: bool,
    ) -> Vec<String> {
        plan.iter()
            .filter(|g| !asleep || g.asleep)
            .filter(|g| {
                let t = effective(g, topics);
                t == tier
                    || (tier == Tier::Minute && t == Tier::Touched && self.retry.contains(&g.name))
            })
            .map(|g| g.name.clone())
            .collect()
    }

    /// The tick of `tier` ended at `end`: its next tick is one period later.
    pub fn ticked(&mut self, tier: Tier, end: Instant, timing: &Timing) {
        if let Some(period) = timing.period(tier) {
            self.next.insert(tier, end + period);
        }
    }

    /// The outcome of a read of a `touched` group (at plug-in, after a request, at a retry):
    /// a failed or cut read is retried (5.7 "Wake and retries"), a good read ends the retry.
    pub fn touched_read(&mut self, group: &str, ok: bool) {
        if ok {
            self.retry.remove(group);
        } else {
            self.retry.insert(group.to_string());
        }
    }

    /// The groups the wake read reads (5.7): every group with a leaf in `unread`, `touched`
    /// groups included, in poll-plan order.
    pub fn wake_groups(plan: &[Group], unread: &[String]) -> Vec<String> {
        plan.iter()
            .filter(|g| g.leaves.iter().any(|l| unread.contains(l)))
            .map(|g| g.name.clone())
            .collect()
    }

    /// The wake read of `groups` ended at `end`: the next tick of each tier one of them is read
    /// in counts from this read.
    pub fn woke(
        &mut self,
        plan: &[Group],
        groups: &[String],
        topics: &BTreeSet<String>,
        end: Instant,
        timing: &Timing,
    ) {
        for g in plan.iter().filter(|g| groups.contains(&g.name)) {
            let tier = effective(g, topics);
            self.ticked(tier, end, timing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::contract::LiveTier;
    use std::time::Duration;

    fn group(name: &str, tier: Tier, asleep: bool, live: Option<(&str, Tier)>) -> Group {
        Group {
            name: name.into(),
            tier,
            asleep,
            live: live.map(|(t, tier)| LiveTier {
                topic: t.into(),
                tier,
            }),
            leaves: vec![format!("{name}.x")],
        }
    }

    fn plan() -> Vec<Group> {
        vec![
            group("status", Tier::Second, true, None),
            group(
                "gimbal",
                Tier::FiveSeconds,
                false,
                Some(("pose", Tier::Second)),
            ),
            group("led", Tier::FiveSeconds, false, None),
            group("zone", Tier::Minute, false, None),
            group("presets", Tier::Touched, false, None),
        ]
    }

    fn none() -> BTreeSet<String> {
        BTreeSet::new()
    }

    #[test]
    fn every_tier_ticks_one_period_after_the_first_read() {
        let t = Instant::now();
        let s = Schedule::new(t, &Timing::SPEC);
        assert_eq!(s.due(t), None);
        assert_eq!(s.due(t + Duration::from_secs(1)), Some(Tier::Second));
        assert_eq!(s.next_due(), t + Duration::from_secs(1));
        assert_eq!(
            s.due(t + Duration::from_secs(60)),
            Some(Tier::Second),
            "the one due longest ago first"
        );
    }

    /// 5.7: intervals count from the end of the previous tick of that tier.
    #[test]
    fn a_tick_counts_from_its_end() {
        let t = Instant::now();
        let mut s = Schedule::new(t, &Timing::SPEC);
        let end = t + Duration::from_millis(1700);
        s.ticked(Tier::Second, end, &Timing::SPEC);
        assert_eq!(s.due(t + Duration::from_millis(2600)), None);
        assert_eq!(s.due(end + Duration::from_secs(1)), Some(Tier::Second));
    }

    #[test]
    fn a_tick_reads_its_tier_and_asleep_only_the_groups_readable_asleep() {
        let s = Schedule::new(Instant::now(), &Timing::SPEC);
        assert_eq!(s.groups(&plan(), Tier::Second, &none(), false), ["status"]);
        assert_eq!(
            s.groups(&plan(), Tier::FiveSeconds, &none(), false),
            ["gimbal", "led"]
        );
        assert_eq!(
            s.groups(&plan(), Tier::FiveSeconds, &none(), true),
            Vec::<String>::new()
        );
        assert_eq!(s.groups(&plan(), Tier::Second, &none(), true), ["status"]);
        assert_eq!(
            s.groups(&plan(), Tier::Minute, &none(), false),
            ["zone"],
            "touched: never"
        );
    }

    /// 5.7: a group with a live topic is read in its faster tier while the topic is subscribed.
    #[test]
    fn a_subscribed_live_topic_moves_its_group_to_the_live_tier() {
        let s = Schedule::new(Instant::now(), &Timing::SPEC);
        let pose: BTreeSet<String> = ["pose".to_string()].into();
        assert_eq!(
            s.groups(&plan(), Tier::Second, &pose, false),
            ["status", "gimbal"]
        );
        assert_eq!(s.groups(&plan(), Tier::FiveSeconds, &pose, false), ["led"]);
        let other: BTreeSet<String> = ["target".to_string()].into();
        assert_eq!(s.groups(&plan(), Tier::Second, &other, false), ["status"]);
    }

    /// 5.7: a failed `touched` read is read again with the next 60 s tick until a read succeeds.
    #[test]
    fn a_failed_touched_read_rides_the_minute_tier_until_it_succeeds() {
        let mut s = Schedule::new(Instant::now(), &Timing::SPEC);
        s.touched_read("presets", false);
        assert_eq!(
            s.groups(&plan(), Tier::Minute, &none(), false),
            ["zone", "presets"]
        );
        assert_eq!(
            s.groups(&plan(), Tier::FiveSeconds, &none(), false),
            ["gimbal", "led"]
        );
        assert_eq!(
            s.groups(&plan(), Tier::Minute, &none(), true),
            Vec::<String>::new(),
            "not while asleep: the wake read takes it"
        );
        s.touched_read("presets", true);
        assert_eq!(s.groups(&plan(), Tier::Minute, &none(), false), ["zone"]);
    }

    /// 5.7: at wake every group with a leaf in `unread` is read once, `touched` groups
    /// included, and the next tick of their tiers counts from that read.
    #[test]
    fn the_wake_read_takes_every_group_with_an_unread_leaf() {
        let unread = vec!["led.x".to_string(), "presets.x".to_string()];
        assert_eq!(Schedule::wake_groups(&plan(), &unread), ["led", "presets"]);
        let t = Instant::now();
        let mut s = Schedule::new(t, &Timing::SPEC);
        let end = t + Duration::from_secs(4);
        s.woke(
            &plan(),
            &["led".into(), "presets".into()],
            &none(),
            end,
            &Timing::SPEC,
        );
        assert_eq!(s.due(t + Duration::from_secs(5)), Some(Tier::Second));
        assert_eq!(s.due(t + Duration::from_millis(8500)), Some(Tier::Second));
        let mut later = s.clone();
        later.ticked(Tier::Second, t + Duration::from_secs(9), &Timing::SPEC);
        assert_eq!(
            later.due(t + Duration::from_secs(9)),
            Some(Tier::FiveSeconds),
            "4 + 5 s"
        );
    }
}
