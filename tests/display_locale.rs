// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, DisplayContext, Engine, Error, Flags};
use std::sync::Arc;
fn engine(description: &str, display: DisplayContext) -> Engine {
    let mut e = Engine::new(Arc::new(
        Database::from_source(&format!("0 byte x {description}\n")).unwrap(),
    ));
    e.flags = Flags(0xffb000);
    e.with_display_context(display)
}
#[test]
fn result_escaping_happens_once_and_preserves_c_string_termination() {
    for flags in [0, Flags::RAW] {
        let mut e = engine("before%cafter", DisplayContext::C);
        e.flags.0 |= flags;
        assert_eq!(e.classify(b"\0AAA").unwrap().output, b"before");
    }
}
#[test]
fn final_escape_checks_expanded_output_before_growth() {
    let mut e = engine("é", DisplayContext::C);
    e.limits.output = 4;
    assert!(e.classify(b"ABCD").unwrap_err().is_limit());
    e.limits.output = 8;
    assert_eq!(e.classify(b"ABCD").unwrap().output, b"\\303\\251");
}
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn explicit_utf8_context_and_default_c_context_stay_isolated() {
    let e = engine("café 漢", DisplayContext::C);
    let unicode = e
        .clone()
        .with_display_context(DisplayContext::locale("C.UTF-8").unwrap());
    assert_eq!(e.capabilities().display_locale, "C");
    assert_eq!(unicode.capabilities().display_locale, "C.UTF-8");
    let workers: Vec<_> = (0..8)
        .map(|i| {
            let e = if i % 2 == 0 {
                e.clone()
            } else {
                unicode.clone()
            };
            std::thread::spawn(move || {
                for _ in 0..100 {
                    let expected = if i % 2 == 0 {
                        b"caf\\303\\251 \\346\\274\\242".as_slice()
                    } else {
                        "café 漢".as_bytes()
                    };
                    assert_eq!(e.classify(b"ABCD").unwrap().output, expected);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}
#[test]
fn invalid_locale_configuration_is_explicit_and_bounded() {
    assert!(
        DisplayContext::locale("rfile-no-such-locale")
            .unwrap_err()
            .to_string()
            .contains("locale")
    );
    assert!(matches!(
        DisplayContext::locale(&"x".repeat(1025)),
        Err(Error::Limit("locale name"))
    ));
    assert!(DisplayContext::locale("").is_err());
}
#[cfg(unix)]
#[test]
fn filesystem_errors_keep_original_bytes_and_os_error() {
    use std::{error::Error as _, os::unix::ffi::OsStringExt};
    let name = std::env::temp_dir().join(std::ffi::OsString::from_vec(
        format!("rfile-absent-{}-", std::process::id())
            .into_bytes()
            .into_iter()
            .chain([255])
            .collect(),
    ));
    let mut e = Engine::builtin().unwrap();
    e.flags = Flags(Flags::ERROR);
    let error = e.classify_path(&name).unwrap_err();
    assert!(error.raw_diagnostic().unwrap().contains(&255));
    let source = error
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
    assert!(source.raw_os_error().is_some());
}
#[test]
fn successful_empty_matches_and_later_c_string_appends_match_the_pin() {
    let mut e = Engine::new(Arc::new(
        Database::from_source("0 regex/b a* %s\n").unwrap(),
    ));
    e.flags = Flags(0xffb000);
    assert!(e.classify(b"\0AAA").unwrap().output.is_empty());
    e.flags.0 |= Flags::CONTINUE;
    assert_eq!(e.classify(b"\0AAA").unwrap().output, b"\\012- ");
    let mut e = engine("before%cafter", DisplayContext::C);
    e.flags.0 |= Flags::CONTINUE;
    assert_eq!(e.classify(b"\0AAA").unwrap().output, b"before");
}
#[test]
fn oversized_paths_fail_before_os_conversion_or_diagnostic_allocation() {
    let path = "x".repeat(65537);
    let e = Engine::builtin().unwrap();
    assert!(matches!(
        e.classify_path(std::path::Path::new(&path)),
        Err(Error::Limit("path length"))
    ));
}
