// SPDX-License-Identifier: GPL-3.0-only
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
fn run(root: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(args)
        .current_dir(root)
        .env("HOME", root)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .env_remove("MAGIC")
        .env_remove("POSIXLY_CORRECT")
        .env_remove("LD_PRELOAD")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
fn supported(root: &Path) -> bool {
    let output = run(root, &["--sandbox=strict", "--sandbox-status"], b"");
    if !cfg!(all(
        feature = "sandbox",
        target_os = "linux",
        target_env = "gnu",
        target_arch = "x86_64"
    )) {
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("not compiled"));
        assert!(
            std::env::var("RFILE_REQUIRE_SANDBOX").as_deref() != Ok("1"),
            "required CLI sandbox certification not compiled"
        );
        println!(
            "{}",
            r#"RFILE_COVERAGE {"component":"sandbox_cli","status":"skipped","reason":"not-compiled"}"#
        );
        return false;
    }
    if String::from_utf8_lossy(&output.stderr).contains("required confinement needs") {
        assert!(
            std::env::var("RFILE_REQUIRE_SANDBOX").as_deref() != Ok("1"),
            "required CLI sandbox certification unavailable: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!(
            "{}",
            r#"RFILE_COVERAGE {"component":"sandbox_cli","status":"skipped","reason":"unsupported-kernel"}"#
        );
        eprintln!(
            "SKIP strict CLI certification: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return false;
    }
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
#[test]
fn cli_policy_modes_order_and_identity() {
    let root = std::env::temp_dir().join(format!("rfile-cli-sandbox-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("text"), b"hello kernel\n").unwrap();
    fs::write(root.join("binary"), [0u8, 255, 0, 255]).unwrap();
    fs::write(root.join("empty"), []).unwrap();
    fs::write(root.join("list"), b"text\nbinary\nempty\nmissing\n").unwrap();
    fs::write(
        root.join("magic"),
        b"0 string hello greeting\n!:mime text/x-greeting\n",
    )
    .unwrap();
    let off = run(
        &root,
        &["--sandbox=off", "--sandbox-status", "-b", "text"],
        b"",
    );
    assert!(off.status.success());
    let status: serde_json::Value = serde_json::from_slice(&off.stderr).unwrap();
    assert_eq!(status["sandbox"]["seccomp_enforced"], false);
    let sandbox_executed = supported(&root);
    if sandbox_executed {
        let cases: &[&[&str]] = &[
            &["-b", "text", "binary", "empty", "missing"],
            &["-bi", "text"],
            &["-00", "text", "binary"],
            &["-p", "text"],
            &["-b", "-"],
            &["-m", "magic", "-b", "text"],
            &["-f", "list", "-i", "text"],
            &["-f", "list", "--unknown"],
            &["-f", "list", "--help"],
            &["-f", "list", "--version"],
            &["-f", "list", "--capabilities"],
            &["-E", "missing"],
        ];
        for args in cases {
            let mut disabled = vec!["-S"];
            disabled.extend_from_slice(args);
            let expected = run(&root, &disabled, b"hello stdin\n");
            for mode in ["--sandbox=auto", "--sandbox=strict"] {
                let mut enabled = vec![mode];
                enabled.extend_from_slice(args);
                let actual = run(&root, &enabled, b"hello stdin\n");
                assert_eq!(
                    (&actual.stdout, &actual.stderr, actual.status.code()),
                    (&expected.stdout, &expected.stderr, expected.status.code()),
                    "{mode} {args:?}"
                );
            }
        }
        for (options, enforced) in [
            (["-S", "--sandbox=strict"], true),
            (["--sandbox=strict", "-S"], false),
        ] {
            let out = run(
                &root,
                &[options[0], options[1], "--sandbox-status", "-b", "text"],
                b"",
            );
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let status: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
            assert_eq!(status["sandbox"]["input_content_confined"], enforced);
        }
        for args in [
            vec!["--sandbox=strict", "-f", "-"],
            vec!["--sandbox=strict", "-n", "-f", "list"],
            vec!["--sandbox=auto", "-f", "list", "-C", "-m", "magic"],
        ] {
            let out = run(&root, &args, b"text\n");
            assert!(!out.status.success());
            assert!(out.stdout.is_empty());
        }
        let mut at_cap = vec!["--sandbox=strict", "-b", "-p"];
        at_cap.extend(std::iter::repeat_n("text", 128));
        let out = run(&root, &at_cap, b"");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout.iter().filter(|&&b| b == b'\n').count(), 128);
        let mut args = vec!["--sandbox=strict"];
        args.extend(std::iter::repeat_n("text", 129));
        let out = run(&root, &args, b"");
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        let out = run(
            &root,
            &["--sandbox=strict", "--sandbox-status", "-C", "-m", "magic"],
            b"",
        );
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let db = fs::read(root.join("magic.mgc")).unwrap();
        let out = run(&root, &["--sandbox=auto", "-C", "-m", "magic"], b"");
        assert!(out.status.success());
        assert_eq!(db, fs::read(root.join("magic.mgc")).unwrap());
        assert!(!fs::read_dir(&root).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".rfile-")
        }));
        #[cfg(unix)]
        {
            fs::write(root.join("untouched"), b"keep original target").unwrap();
            fs::remove_file(root.join("magic.mgc")).unwrap();
            std::os::unix::fs::symlink("untouched", root.join("magic.mgc")).unwrap();
            let out = run(&root, &["--sandbox=auto", "-C", "-m", "magic"], b"");
            assert!(out.status.success());
            assert!(
                !fs::symlink_metadata(root.join("magic.mgc"))
                    .unwrap()
                    .is_symlink()
            );
            assert_eq!(
                fs::read(root.join("untouched")).unwrap(),
                b"keep original target"
            );
            std::os::unix::fs::symlink("text", root.join("link")).unwrap();
            std::os::unix::fs::symlink("absent", root.join("broken")).unwrap();
            for args in [
                vec!["link", "broken", "."],
                vec!["-L", "link"],
                vec!["--extension", "link"],
            ] {
                let mut disabled = vec!["-S"];
                disabled.extend(&args);
                let mut strict = vec!["--sandbox=strict"];
                strict.extend(&args);
                let a = run(&root, &disabled, b"");
                let b = run(&root, &strict, b"");
                assert_eq!(
                    (a.stdout, a.stderr, a.status.code()),
                    (b.stdout, b.stderr, b.status.code())
                );
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
    if sandbox_executed {
        println!(
            "{}",
            r#"RFILE_COVERAGE {"component":"sandbox_cli","status":"executed"}"#
        );
    }
}
