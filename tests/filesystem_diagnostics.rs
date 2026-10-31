// SPDX-License-Identifier: GPL-3.0-only
#![cfg(all(target_os = "linux", target_env = "gnu"))]
use rfile::{Engine, Flags};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn empty_metadata_order_respects_content_inspection_modes() {
    let dir = std::env::temp_dir().join(format!("rfile-unreadable-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
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
    let path = std::env::temp_dir().join(format!("rfile-fifo-diagnostic-{}", std::process::id()));
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
    fs::remove_file(path).unwrap();
}
