// SPDX-License-Identifier: GPL-3.0-only
#![cfg(all(target_os = "linux", target_env = "gnu"))]
use rfile::{Engine, Flags};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn scratch_directory(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("rfile-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    dir
}

#[test]
fn empty_metadata_order_respects_content_inspection_modes() {
    let dir = scratch_directory("unreadable");
    let path = dir.join("input");
    let mut engine = Engine::builtin().unwrap();
    for empty in [false, true] {
        for mode in [0, 0o100, 0o200, 0o300] {
            if path.exists() {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            fs::write(&path, if empty { &b""[..] } else { &b"hello\n"[..] }).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            assert!(
                fs::File::open(&path).is_err(),
                "requires an unprivileged test runner"
            );
            let expected = format!(
                "{}{}regular file, no read permission",
                if mode & 0o200 != 0 { "writable, " } else { "" },
                if mode & 0o100 != 0 {
                    "executable, "
                } else {
                    ""
                }
            );
            for sandbox in ["--sandbox=off", "--sandbox=auto"] {
                let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
                    .args([sandbox, "-b", "-i", "-E"])
                    .arg(&path)
                    .env("TZ", "UTC0")
                    .output()
                    .unwrap();
                assert!(output.status.success(), "{sandbox}: {output:?}");
                let expected_mime = if empty {
                    "inode/x-empty; charset=binary"
                } else {
                    &expected
                };
                assert_eq!(output.stdout, format!("{expected_mime}\n").as_bytes());
            }
            for (flags, empty_output) in [
                (0, &b"empty"[..]),
                (Flags::MIME, &b"inode/x-empty; charset=binary"[..]),
                (Flags::MIME_TYPE, &b"inode/x-empty"[..]),
                (Flags::MIME_ENCODING, &b"binary"[..]),
                (Flags::EXTENSION, &b""[..]),
                (Flags::APPLE, &b""[..]),
            ] {
                for extra in [
                    0,
                    Flags::ERROR,
                    Flags::DEVICES,
                    Flags::DEVICES | Flags::ERROR,
                    Flags::SYMLINK,
                ] {
                    engine.flags = Flags(flags | extra);
                    let expected_output = if empty
                        && extra & Flags::DEVICES == 0
                        && flags & (Flags::APPLE | Flags::EXTENSION) == 0
                    {
                        empty_output
                    } else {
                        expected.as_bytes()
                    };
                    assert_eq!(
                        engine.classify_path(&path).unwrap().output,
                        expected_output,
                        "empty={empty} mode={mode:o} flags={flags:x} extra={extra:x}"
                    );
                }
            }
        }
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    engine.flags = Flags(Flags::ERROR);
    assert!(engine.classify_path(&dir.join("missing")).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn fifo_content_requests_are_explicit_errors_instead_of_empty() {
    let dir = scratch_directory("fifo-diagnostic");
    let path = dir.join("fifo");
    assert!(
        Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let mut engine = Engine::builtin().unwrap();
    assert_eq!(
        engine.classify_path(&path).unwrap().output,
        b"fifo (named pipe)"
    );
    for extra in [0, Flags::MIME, Flags::ERROR, Flags::APPLE, Flags::EXTENSION] {
        engine.flags = Flags(Flags::DEVICES | extra);
        let error = engine.classify_path(&path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("content inspection of FIFO is not supported")
        );
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn annotations_follow_unreadable_regular_symlink_targets_without_dereference_flag() {
    // Oracle expectations from pass-5 tools_matrix.py rows l_e000/l_n000;
    // also retain the writable/executable prefixes for target mode 0300.
    use std::os::unix::fs::symlink;
    let dir = scratch_directory("annotation-links");
    let target = dir.join("target");
    let link = dir.join("link");
    symlink(&target, &link).unwrap();
    let mut engine = Engine::builtin().unwrap();
    for payload in [&b""[..], &b"hello\n"[..]] {
        for mode in [0, 0o300] {
            if target.exists() {
                fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
            }
            fs::write(&target, payload).unwrap();
            fs::set_permissions(&target, fs::Permissions::from_mode(mode)).unwrap();
            assert!(
                fs::File::open(&target).is_err(),
                "requires an unprivileged test runner"
            );
            let expected = if mode == 0 {
                "regular file, no read permission"
            } else {
                "writable, executable, regular file, no read permission"
            };
            for flags in [
                Flags::APPLE,
                Flags::EXTENSION,
                Flags::APPLE | Flags::EXTENSION,
                Flags::DEVICES | Flags::APPLE,
            ] {
                engine.flags = Flags(flags);
                assert_eq!(
                    engine.classify_path(&link).unwrap().output,
                    expected.as_bytes()
                );
            }
            // Deliberate mixed-flag difference: one permission description,
            // rather than the oracle's symlink-MIME prefix plus description.
            engine.flags = Flags(Flags::MIME | Flags::APPLE);
            assert_eq!(
                engine.classify_path(&link).unwrap().output,
                expected.as_bytes()
            );
            for args in [
                vec!["--apple"],
                vec!["--extension"],
                vec!["--apple", "--extension"],
                vec!["-s", "--apple"],
                vec!["-i", "--apple"],
            ] {
                for sandbox in ["--sandbox=off", "--sandbox=auto"] {
                    let output = Command::new(env!("CARGO_BIN_EXE_rfile"))
                        .args([sandbox, "-b"])
                        .args(&args)
                        .arg(&link)
                        .env("TZ", "UTC0")
                        .output()
                        .unwrap();
                    assert!(output.status.success(), "{output:?}");
                    assert_eq!(output.stdout, format!("{expected}\n").as_bytes());
                }
            }
        }
    }
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    let directory_link = dir.join("directory-link");
    symlink(&dir, &directory_link).unwrap();
    engine.flags = Flags(Flags::APPLE);
    assert!(
        engine
            .classify_path(&directory_link)
            .unwrap_err()
            .to_string()
            .contains("annotation content inspection of special symlink target")
    );
    fs::remove_dir_all(dir).unwrap();
}
