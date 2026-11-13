// SPDX-License-Identifier: GPL-3.0-or-later

//! Stale test of the generated description files. `scripts/regenerate.sh` sets
//! `OBSBOT_REGENERATE=1`, which writes them instead.

use obsbot_device::descriptions::{all, compare, render};
use obsbot_proto::Description;
use obsbot_proto::validate::check_description;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn description_files_are_current() {
    let regenerate = std::env::var_os("OBSBOT_REGENERATE").is_some();
    let mut stale = Vec::new();
    for (path, description) in all() {
        let file = root().join(path);
        if regenerate {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, render(&description)).unwrap();
            continue;
        }
        let have = std::fs::read_to_string(&file).unwrap_or_default();
        if let Err(e) = compare(path, &description, &have) {
            stale.push(e);
        }
    }
    assert!(stale.is_empty(), "{}", stale.join("\n"));
}

#[test]
fn a_changed_description_is_reported_stale() {
    let (path, description) = all().remove(0);
    let file = render(&description);
    let mut changed = description.clone();
    changed.settings.get_mut("led.brightness").unwrap().label = "LED level".into();
    assert!(compare(path, &description, &file).is_ok());
    assert!(
        compare(path, &changed, &file)
            .unwrap_err()
            .contains("scripts/regenerate.sh")
    );
}

#[test]
fn every_description_file_round_trips_and_is_consistent() {
    for (path, description) in all() {
        assert_eq!(
            check_description(&description),
            Vec::<String>::new(),
            "{path}"
        );
        let text = render(&description);
        let parsed: Description = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed, description, "{path}");
        assert_eq!(render(&parsed), text, "{path}: not byte-equal");
    }
}
