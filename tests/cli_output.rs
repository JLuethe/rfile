// SPDX-License-Identifier: GPL-3.0-only
#![cfg(all(target_os = "linux", target_env = "gnu"))]
use std::{
    fs::{File, OpenOptions},
    process::{Command, Stdio},
};

#[test]
fn filesystem_diagnostics_escape_control_bytes_unless_raw() {
    use std::os::unix::ffi::OsStringExt;
    let name = std::ffi::OsString::from_vec(b"missing-\x1b]52;c;bad\x07\r\n\xff".to_vec());
    let safe = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(["--sandbox=off", "-E"])
        .arg(&name)
        .output()
        .unwrap();
    assert_eq!(safe.status.code(), Some(1));
    assert!(!safe.stdout.contains(&0x1b));
    assert!(!safe.stdout.contains(&b'\r'));
    assert!(!safe.stdout.contains(&0x07));
    assert!(safe.stdout.windows(4).any(|w| w == b"\\u{1"));
    assert!(safe.stdout.windows(4).any(|w| w == b"\\377"));
    let raw = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(["--sandbox=off", "-E", "--raw"])
        .arg(&name)
        .output()
        .unwrap();
    assert!(raw.stdout.contains(&0x1b));
}

#[test]
fn diagnostic_renderer_keeps_unicode_and_escapes_bidi() {
    use std::os::unix::ffi::OsStringExt;
    let name = std::ffi::OsString::from_vec("missing-é-\u{202e}".as_bytes().to_vec());
    let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(["--sandbox=off", "-E", "-b"])
        .arg(name)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.windows(2).any(|w| w == "é".as_bytes()));
    assert!(output.stdout.windows(8).any(|w| w == b"\\u{202e}"));
    assert!(!output.stdout.windows(3).any(|w| w == "\u{202e}".as_bytes()));
}

#[test]
fn normal_results_and_labels_escape_bidi_under_utf8_locale() {
    let name = "missing-\u{202e}-\u{200f}-\u{061c}";
    for error_mode in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rfile"));
        command
            .args(["--sandbox=off", name])
            .env("LC_ALL", "C.UTF-8");
        if error_mode {
            command.arg("-E");
        }
        let output = command.output().unwrap();
        assert!(!output.stdout.windows(3).any(|w| w == "\u{202e}".as_bytes()));
        assert!(!output.stdout.windows(3).any(|w| w == "\u{200f}".as_bytes()));
        assert!(!output.stdout.windows(2).any(|w| w == "\u{061c}".as_bytes()));
        assert!(output.stdout.windows(8).any(|w| w == b"\\u{202e}"));
        assert!(output.stdout.windows(8).any(|w| w == b"\\u{200f}"));
        assert!(output.stdout.windows(7).any(|w| w == b"\\u{61c}"));
    }
}

#[test]
fn output_failures_are_errors_instead_of_panics_or_silent_success() {
    for args in [
        vec!["-b", "-"],
        vec!["--help"],
        vec!["-v"],
        vec!["--capabilities"],
    ] {
        for sink in [
            File::open("/dev/null").unwrap(),
            OpenOptions::new().write(true).open("/dev/full").unwrap(),
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
                .args(&args)
                .env("TZ", "UTC0")
                .stdin(Stdio::null())
                .stdout(sink)
                .stderr(Stdio::piped())
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
            assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
        }
    }
}

#[test]
fn sandbox_status_is_best_effort_on_unwritable_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(["--sandbox=off", "--sandbox-status", "--help"])
        .stderr(OpenOptions::new().write(true).open("/dev/full").unwrap())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}

#[test]
fn broken_output_stream_exits_quietly_without_panicking() {
    use std::os::{fd::OwnedFd, unix::net::UnixStream};
    for args in [
        vec!["-b", "-"],
        vec!["--help"],
        vec!["-v"],
        vec!["--capabilities"],
    ] {
        let (reader, writer) = UnixStream::pair().unwrap();
        drop(reader);
        let fd: OwnedFd = writer.into();
        let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
            .args(args)
            .env("TZ", "UTC0")
            .stdin(Stdio::null())
            .stdout(Stdio::from(fd))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[test]
fn debug_diagnostics_do_not_panic_on_unwritable_stderr() {
    let path =
        std::env::temp_dir().join(format!("rfile-debug-output-{}.magic", std::process::id()));
    std::fs::write(&path, b"0 byte x matched\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(["-d", "-b", "-m"])
        .arg(&path)
        .arg(&path)
        .env("TZ", "UTC0")
        .stderr(OpenOptions::new().write(true).open("/dev/full").unwrap())
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"matched\n");
}

#[test]
fn silent_compilation_does_not_require_writable_stdout() {
    let dir = std::env::temp_dir().join(format!("rfile-silent-compile-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let rules = dir.join("rules.magic");
    std::fs::write(&rules, b"0 byte x matched\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(["--sandbox=off", "-C", "-m"])
        .arg(&rules)
        .current_dir(&dir)
        .env("TZ", "UTC0")
        .stdout(File::open("/dev/null").unwrap())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(dir.join("rules.magic.mgc").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
