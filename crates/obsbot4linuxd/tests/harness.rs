// SPDX-License-Identifier: GPL-3.0-or-later

//! The test harness compiles from the skeleton on, and its timing keeps the spec's request
//! deadline: only tier periods and scans are shortened for tests (spec 11).

mod common;

#[test]
fn the_test_timing_shortens_only_tiers_and_scans() {
    let t = common::fast();
    assert_eq!(t.request_deadline, obsbot_proto::REQUEST_DEADLINE);
    assert!(t.second < t.five_seconds && t.five_seconds < t.minute);
    assert_eq!(
        common::temp_dir().parent(),
        Some(std::env::temp_dir().as_path())
    );
}
