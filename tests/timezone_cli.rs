// SPDX-License-Identifier: GPL-3.0-only
use std::process::Command;

#[test]
fn unavailable_explicit_timezone_reports_the_setting_for_mime_and_brief() {
    for zone in [
        ":/rfile-test-no-such-zone-6e797a",
        "Rfile_Test_No_Such_Zone/Example",
    ] {
        for flag in ["-b", "--mime"] {
            let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
                .env("TZ", zone)
                .args([flag, "-"])
                .output()
                .expect("run rfile");
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.contains(&format!("TZ={zone:?}")), "{stderr}");
        }
    }
}
