// SPDX-License-Identifier: GPL-3.0-or-later

//! The difference between two scans (5.3). An enumeration is identified by the USB path
//! together with the device number: when the device number under a path changes between two
//! scans, the old enumeration went away and a new one came, even if no scan saw the path
//! missing.

use obsbot_device::contract::Found;

/// One enumeration the registry knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub path: String,
    pub devnum: u32,
}

impl Seen {
    pub fn of(found: &Found) -> Seen {
        Seen {
            path: found.usb.path.clone(),
            devnum: found.devnum,
        }
    }
}

/// The result of comparing a scan with the known enumerations: indexes into `known` of the
/// enumerations that went away, and indexes into `found` of the new ones, in scan order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub gone: Vec<usize>,
    pub new: Vec<usize>,
}

pub fn diff(known: &[Seen], found: &[Found]) -> Diff {
    let now: Vec<Seen> = found.iter().map(Seen::of).collect();
    Diff {
        gone: (0..known.len())
            .filter(|i| !now.contains(&known[*i]))
            .collect(),
        new: (0..now.len())
            .filter(|i| !known.contains(&now[*i]))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::contract::Bus;
    use obsbot_device::models::fake::{FakeBus, KEYS};

    #[test]
    fn a_first_scan_finds_everything_new() {
        let found = FakeBus::new(&KEYS).unwrap().scan();
        assert_eq!(
            diff(&[], &found),
            Diff {
                gone: vec![],
                new: vec![0, 1, 2]
            }
        );
        let known: Vec<Seen> = found.iter().map(Seen::of).collect();
        assert_eq!(
            diff(&known, &found),
            Diff {
                gone: vec![],
                new: vec![]
            },
            "an unchanged scan changes nothing"
        );
    }

    #[test]
    fn a_missing_path_is_gone() {
        let bus = FakeBus::new(&KEYS).unwrap();
        let known: Vec<Seen> = bus.scan().iter().map(Seen::of).collect();
        bus.unplug("sn:FAKEB0000001").unwrap();
        assert_eq!(
            diff(&known, &bus.scan()),
            Diff {
                gone: vec![1],
                new: vec![]
            }
        );
    }

    /// 5.3: a device-number change under the same path is an unplug followed by a plug-in.
    #[test]
    fn a_new_device_number_under_a_known_path_is_gone_and_new() {
        let bus = FakeBus::new(&KEYS).unwrap();
        let known: Vec<Seen> = bus.scan().iter().map(Seen::of).collect();
        bus.unplug("sn:FAKEA0000001").unwrap();
        bus.plug("fake-a").unwrap();
        let found = bus.scan();
        let d = diff(&known, &found);
        assert_eq!(d.gone, vec![0]);
        assert_eq!(d.new.len(), 1);
        assert_eq!(found[d.new[0]].usb.path, known[0].path, "same path");
        assert_ne!(found[d.new[0]].devnum, known[0].devnum);
    }
}
