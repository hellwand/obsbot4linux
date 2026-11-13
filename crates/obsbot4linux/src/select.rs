// SPDX-License-Identifier: GPL-3.0-or-later

//! Device selection (design spec 6.2, "Device selection", "Unsupported devices", "No usable
//! device"). Only the device list of the `hello` reply is used; nothing is sent.

use crate::VERSION;
use crate::errors::{CliCode, CliError};
use crate::session::ISSUES;
use obsbot_proto::{Device, ErrorCode, Support};

/// The hint that ends the `no_device` and `unsupported_device` messages.
pub fn report_hint() -> String {
    format!("`obsbot4linux devices --report` prints what an issue report needs: {ISSUES}")
}

/// Whether a command may use the device without `--device`: available, and `tested` or
/// `untested`. Unsupported devices never count as usable.
pub fn usable(d: &Device) -> bool {
    d.available && matches!(d.support, Support::Tested | Support::Untested)
}

/// The listed devices `--device` matches: by full id, by a serial suffix of at least four
/// characters, or by model key (`unknown` included). One leading `…` is dropped first, so the
/// shortened id that human output prints (`…0001`) can be pasted back.
pub fn matching<'a>(devices: &'a [Device], wanted: &str) -> Vec<&'a Device> {
    let wanted = wanted.strip_prefix('…').unwrap_or(wanted);
    devices
        .iter()
        .filter(|d| {
            d.id == wanted
                || d.model == wanted
                || (wanted.chars().count() >= 4
                    && d.serial.as_deref().is_some_and(|s| s.ends_with(wanted)))
        })
        .collect()
}

/// The one device a command acts on. With `--device` it must match exactly one listed
/// device, whatever its support or availability; without, there must be exactly one usable
/// device.
pub fn select<'a>(devices: &'a [Device], wanted: Option<&str>) -> Result<&'a Device, CliError> {
    match wanted {
        Some(w) => {
            let found = matching(devices, w);
            match found.as_slice() {
                [one] => Ok(one),
                [] => Err(CliError::new(
                    CliCode::Proto(ErrorCode::UnknownDevice),
                    format!("--device {w} matches no listed device"),
                )
                .with_hint("`obsbot4linux devices` lists them")),
                many => Err(CliError::new(
                    CliCode::AmbiguousDevice,
                    format!("--device {w} matches {} devices", many.len()),
                )
                .with_devices(many.iter().map(|d| (*d).clone()).collect())),
            }
        }
        None => {
            let found: Vec<&Device> = devices.iter().filter(|d| usable(d)).collect();
            match found.as_slice() {
                [one] => Ok(one),
                [] => Err(CliError::new(CliCode::NoDevice, "no usable OBSBOT camera")
                    .with_devices(devices.to_vec())
                    .with_hint(report_hint())),
                many => Err(CliError::new(
                    CliCode::AmbiguousDevice,
                    format!("{} usable devices: choose one with --device", many.len()),
                )
                .with_devices(many.iter().map(|d| (*d).clone()).collect())),
            }
        }
    }
}

/// `status`, `get`, `set` and `do` refuse an unsupported device before anything is sent
/// (spec 6.2, exit 3, code `unsupported_device`).
pub fn refuse_unsupported(d: &Device) -> Result<(), CliError> {
    if d.support != Support::Unsupported {
        return Ok(());
    }
    Err(CliError::new(
        CliCode::Proto(ErrorCode::UnsupportedDevice),
        format!(
            "{} (USB {}:{}) is not supported by obsbot4linux {VERSION}. {}",
            d.name,
            d.usb.vendor,
            d.usb.product,
            report_hint()
        ),
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use obsbot_proto::{Description, Usb};

    pub(crate) fn device(id: &str, model: &str, support: Support, available: bool) -> Device {
        let serial = id.strip_prefix("sn:").map(str::to_string);
        Device {
            id: id.into(),
            name: format!("{model} device"),
            model: model.into(),
            support,
            serial,
            firmware: None,
            video_node: Some("/dev/video0".into()),
            available,
            replaced_by: None,
            usb: Usb {
                vendor: "3564".into(),
                product: "fa00".into(),
                product_name: None,
                bcd_device: "0100".into(),
                path: "9-3".into(),
                serial_present: false,
                interfaces: vec!["0e".into()],
                extension_units: vec![],
            },
            description: Description::default(),
        }
    }

    fn three() -> Vec<Device> {
        vec![
            device("sn:FAKEA0000001", "fake-a", Support::Tested, true),
            device("sn:FAKEB0000001", "fake-b", Support::Untested, true),
            device("port:9-3", "unknown", Support::Unsupported, true),
        ]
    }

    fn ids(e: &CliError) -> Vec<&str> {
        e.devices.iter().map(|d| d.id.as_str()).collect()
    }

    #[test]
    fn device_takes_a_full_id_a_serial_suffix_or_a_model_key() {
        let list = three();
        assert_eq!(
            select(&list, Some("sn:FAKEB0000001")).unwrap().model,
            "fake-b"
        );
        assert_eq!(select(&list, Some("fake-a")).unwrap().id, "sn:FAKEA0000001");
        assert_eq!(select(&list, Some("B0000001")).unwrap().model, "fake-b");
        assert_eq!(select(&list, Some("unknown")).unwrap().id, "port:9-3");
        assert_eq!(select(&list, Some("port:9-3")).unwrap().model, "unknown");
    }

    #[test]
    fn a_suffix_needs_four_characters_and_must_be_unique() {
        let list = three();
        let e = select(&list, Some("001")).unwrap_err();
        assert_eq!(e.code.as_str(), "unknown_device");
        assert_eq!(e.code.exit(), 2);
        let e = select(&list, Some("0001")).unwrap_err();
        assert_eq!(e.code, CliCode::AmbiguousDevice);
        assert_eq!(e.code.exit(), 2);
        assert_eq!(ids(&e), ["sn:FAKEA0000001", "sn:FAKEB0000001"]);
    }

    #[test]
    fn without_device_the_only_usable_device_is_taken() {
        let mut list = three();
        let e = select(&list, None).unwrap_err();
        assert_eq!(e.code, CliCode::AmbiguousDevice);
        assert_eq!(ids(&e), ["sn:FAKEA0000001", "sn:FAKEB0000001"]);
        list[1].available = false;
        assert_eq!(select(&list, None).unwrap().model, "fake-a");
    }

    /// Spec 8.4 step 5: with a supported camera connected too, the unsupported one is ignored.
    #[test]
    fn unsupported_devices_never_count_as_usable() {
        let list = three();
        assert!(!usable(&list[2]));
        let only_unknown = vec![list[2].clone()];
        let e = select(&only_unknown, None).unwrap_err();
        assert_eq!(e.code, CliCode::NoDevice);
        assert_eq!(e.code.exit(), 3);
        assert_eq!(e.message, "no usable OBSBOT camera");
        assert_eq!(ids(&e), ["port:9-3"]);
        assert!(e.hint.as_deref().unwrap().contains("devices --report"));
    }

    #[test]
    fn an_empty_list_is_no_device() {
        let e = select(&[], None).unwrap_err();
        assert_eq!(e.code, CliCode::NoDevice);
        assert!(e.devices.is_empty());
    }

    #[test]
    fn an_unsupported_device_is_refused_with_the_report_hint() {
        let list = three();
        assert!(refuse_unsupported(&list[0]).is_ok());
        let e = refuse_unsupported(&list[2]).unwrap_err();
        assert_eq!(e.code.as_str(), "unsupported_device");
        assert_eq!(e.code.exit(), 3);
        assert_eq!(
            e.message,
            format!(
                "unknown device (USB 3564:fa00) is not supported by obsbot4linux {VERSION}. \
                 `obsbot4linux devices --report` prints what an issue report needs: \
                 https://github.com/hellwand/obsbot4linux/issues"
            )
        );
    }

    /// Human output shortens ids (spec 4.2); `--json` carries them in full.
    #[test]
    fn an_ambiguous_choice_lists_short_ids_for_people_and_full_ids_in_json() {
        let e = select(&three(), Some("0001")).unwrap_err();
        let human = e.human();
        assert!(human.contains("…0001  fake-a device  fake-a\n"), "{human}");
        assert!(!human.contains("FAKEA0000001"), "{human}");
        assert_eq!(
            e.json_line(),
            r#"{"error":{"code":"ambiguous_device","devices":["sn:FAKEA0000001","sn:FAKEB0000001"],"message":"--device 0001 matches 2 devices"}}"#
        );
    }

    /// Human output shows `…0001` (spec 4.2); pasted into `--device` it is the serial suffix
    /// `0001`: a unique match selects, several matches stay `ambiguous_device`.
    #[test]
    fn a_pasted_short_id_matches_as_the_serial_suffix() {
        let list = three();
        let e = select(&list, Some("…0001")).unwrap_err();
        assert_eq!(e.code, CliCode::AmbiguousDevice);
        assert_eq!(e.code.exit(), 2);
        assert_eq!(ids(&e), ["sn:FAKEA0000001", "sn:FAKEB0000001"]);
        assert_eq!(select(&list, Some("…B0000001")).unwrap().model, "fake-b");
        let one = vec![list[0].clone()];
        assert_eq!(select(&one, Some("…0001")).unwrap().id, "sn:FAKEA0000001");
        // Only one `…` is dropped, and the rest still needs four characters.
        let e = select(&list, Some("……0001")).unwrap_err();
        assert_eq!(e.code.as_str(), "unknown_device");
        let e = select(&list, Some("…001")).unwrap_err();
        assert_eq!(e.code.as_str(), "unknown_device");
    }

    /// Review focus: a support level from a newer daemon (4.1 rule 7) is shown as it is and
    /// never counts as usable.
    #[test]
    fn an_unknown_support_level_is_shown_and_not_usable() {
        let d = device(
            "sn:NEWER0000001",
            "newer",
            Support::Other("experimental".into()),
            true,
        );
        assert!(!usable(&d));
        assert_eq!(
            crate::errors::device_line(&d),
            "…0001  newer device  newer  experimental"
        );
        let e = select(std::slice::from_ref(&d), None).unwrap_err();
        assert_eq!(e.code, CliCode::NoDevice);
    }
}
