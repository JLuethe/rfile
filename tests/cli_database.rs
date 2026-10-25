// SPDX-License-Identifier: GPL-3.0-only
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rfile-db-actions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        run(&self.0, args)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rfile"))
        .args(args)
        .current_dir(path)
        .env("LC_ALL", "C")
        .env("TZ", "UTC0")
        .env("HOME", path)
        .env_remove("MAGIC")
        .env_remove("LD_PRELOAD")
        .env_remove("POSIXLY_CORRECT")
        .output()
        .unwrap()
}
#[test]
fn check_preserves_parse_order_and_compiled_preference() {
    let f = Fixture::new();
    fs::write(f.0.join("rules"), b"0 byte 1 first\n0 string ABC second\n").unwrap();
    let check = f.run(&["-S", "-c", "-m", "rules"]);
    assert!(check.status.success());
    assert!(check.stdout.is_empty());
    let text = String::from_utf8(check.stderr).unwrap();
    assert!(text.starts_with(
        "Warning: using regular magic file `rules'\ncont\toffset\ttype\topcode\tmask\tvalue\tdesc\n"
    ));
    assert!(text.find("first").unwrap() < text.find("second").unwrap());
    assert!(!text.contains("Strength ="));
    assert!(f.run(&["-S", "-C", "-m", "rules"]).status.success());
    fs::write(f.0.join("rules"), b"broken source\n").unwrap();
    let compiled = f.run(&["-S", "-c", "-m", "rules"]);
    assert!(compiled.status.success());
    assert!(compiled.stderr.is_empty());
}
#[test]
fn missing_members_preserve_headers_and_partial_success() {
    let f = Fixture::new();
    fs::write(f.0.join("rules"), b"0 string ABC found\n").unwrap();
    let output = f.run(&["-S", "-c", "-m", "missing:rules"]);
    assert!(output.status.success());
    assert_eq!(output.stderr,b"Warning: using regular magic file `missing'\ncont\toffset\ttype\topcode\tmask\tvalue\tdesc\nmissing, 0: Warning: using regular magic file `rules'\ncont\toffset\ttype\topcode\tmask\tvalue\tdesc\n*unknown*, 1: > 0 string,=ABC,\"found\"]\n");
    let empty = f.run(&["-S", "-c", "-m", ":rules"]);
    assert_eq!(empty.status.code(), Some(1));
    assert_eq!(
        empty.stderr,
        b"rfile: could not find any valid magic files!\n"
    );
}
#[test]
fn action_warning_cap_is_native_default_and_list_accumulates() {
    let f = Fixture::new();
    fs::write(f.0.join("bad"), b"0 bogus x invalid\n0 string ABC found\n").unwrap();
    let plain = f.run(&["-S", "-c", "-m", "bad"]);
    let parameter = f.run(&["-S", "-c", "-m", "bad", "-P", "magwarn=0"]);
    assert_eq!(plain.status.code(), Some(1));
    assert_eq!(plain.stderr, parameter.stderr);
    fs::write(f.0.join("a"), b"0 string ABC first\n").unwrap();
    fs::write(f.0.join("b"), b"0 string XYZ second\n").unwrap();
    let output = f.run(&["-S", "-l", "-m", "a:b"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.matches("first").count(), 2);
    assert_eq!(text.matches("second").count(), 1);
}
#[cfg(unix)]
#[test]
fn raw_cookie_error_preserves_non_utf8_bytes() {
    let f = Fixture::new();
    fs::write(f.0.join("bad"), b"!:unknown \xff\n").unwrap();
    let output = f.run(&["-S", "-c", "-m", "bad"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output
            .stderr
            .ends_with(b"rfile: Unknown !: entry `!:unknown \xff'\n")
    );
}
