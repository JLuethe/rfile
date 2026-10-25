// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, SourceActionOptions, SourceFragment, TimeContext};
fn options(count: usize, limit: usize) -> SourceActionOptions {
    SourceActionOptions {
        check: true,
        warning_count: count,
        warning_limit: limit,
        output_limit: 4096,
    }
}
#[test]
fn native_diagnostics_continue_in_parse_order_without_changing_strict_api() {
    let source = b"0 bogus x invalid\n0 string ABC middle\nwrong byte 0 invalid\n";
    let result = Database::source_action(
        &[SourceFragment {
            name: b"rules".to_vec(),
            bytes: source.to_vec(),
        }],
        &TimeContext::default(),
        options(0, 64),
    );
    assert!(result.database.is_err());
    assert!(result.native_failure);
    assert_eq!(result.warning_count, 2);
    assert_eq!(result.last_source, Some((b"rules".to_vec(), 4)));
    assert_eq!(result.output,b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\nrules, 1: Warning: type `bogus x invalid' invalid\n*unknown*, 2: > 0 string,=ABC,\"middle\"]\nrules, 3: Warning: offset `wrong byte 0 invalid' invalid\n");
    let strict = Database::check_source_dump(source, "rules", &TimeContext::default(), 4096);
    assert!(strict.database.is_err());
    assert_eq!(
        strict.output,
        b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n"
    );
}
#[test]
fn diagnostic_cap_stops_next_physical_line_and_carries_counter() {
    let sources = [SourceFragment {
        name: b"a".to_vec(),
        bytes: b"0 bogus x first\n0 bogus x second\n0 string A third\n".to_vec(),
    }];
    let result = Database::source_action(&sources, &TimeContext::default(), options(1, 2));
    assert!(result.native_failure);
    assert_eq!(result.warning_count, 2);
    assert_eq!(result.last_source, Some((b"a".to_vec(), 2)));
    assert_eq!(result.output,b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\na, 1: Maximum number of warnings (2) exceeded.\na, 1: Additional warnings are suppressed.\n");
    let stopped = Database::source_action(&sources, &TimeContext::default(), options(2, 2));
    assert_eq!(stopped.database.unwrap().rule_count(), 0);
    assert_eq!(stopped.last_source, Some((b"a".to_vec(), 1)));
}
#[test]
fn raw_provenance_and_first_nul_boundary_are_preserved() {
    let result = Database::source_action(
        &[SourceFragment {
            name: b"r\xff".to_vec(),
            bytes: b"bad byte 0\0ignored\n".to_vec(),
        }],
        &TimeContext::default(),
        options(0, 64),
    );
    assert!(result.native_failure);
    assert!(
        result
            .output
            .ends_with(b"r\xff, 1: Warning: offset `bad byte 0' invalid\n")
    );
}
#[test]
fn unmapped_diagnostics_and_output_limits_remain_explicit_errors() {
    let result = Database::source_action(
        &[SourceFragment {
            name: b"r".to_vec(),
            bytes: b"0 string/X A test\n".to_vec(),
        }],
        &TimeContext::default(),
        options(0, 64),
    );
    assert!(result.database.is_err());
    assert!(!result.native_failure);
    let mut small = options(0, 64);
    small.output_limit = 2;
    let result = Database::source_action(&[], &TimeContext::default(), small);
    assert!(result.database.unwrap_err().is_limit());
    assert!(!result.native_failure);
    assert!(result.output.len() <= 2);
}

#[test]
fn skipped_levels_are_not_allowed_to_reuse_stale_descendant_matches() {
    let source = "0 string ABC root\n>0 string ABC one\n>>0 string ABC two\n>>>0 string ABC three\n>0 string XYZ miss\n>>>>0 string ABC stale\n>0 string ABC last\n";
    assert!(Database::from_source(source).is_err()); // existing strict helper policy
    let report = Database::source_action(
        &[SourceFragment {
            name: b"rules".to_vec(),
            bytes: source.as_bytes().to_vec(),
        }],
        &TimeContext::default(),
        options(0, 64),
    );
    assert_eq!(report.warning_count, 1);
    assert!(!report.native_failure);
    let db = report.database.unwrap();
    let bytes = db.compile().unwrap();
    for db in [db, Database::from_compiled(&bytes).unwrap()] {
        let engine = rfile::Engine::new(std::sync::Arc::new(db));
        let result = engine.inspect_complete_buffer(b"ABC");
        assert_eq!(result.last_magic_line, Some(7));
        assert_eq!(result.result.unwrap().output, b"root one two three last");
    }
}
#[test]
fn cookie_errors_are_separate_from_warning_counters() {
    let report = Database::source_action(
        &[SourceFragment {
            name: b"rules".to_vec(),
            bytes: b"!:unknown x\n!:mime text/plain\n".to_vec(),
        }],
        &TimeContext::default(),
        options(2, 64),
    );
    assert!(report.native_failure);
    assert_eq!(report.warning_count, 2);
    assert_eq!(
        report.native_error,
        Some(b"Unknown !: entry `!:unknown x'".to_vec())
    );
    assert_eq!(
        report.output,
        b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n"
    );
    assert_eq!(report.last_source, Some((b"rules".to_vec(), 3)));
}

#[test]
fn source_acceptance_cannot_emit_an_unimportable_pstring_record() {
    let source = format!("0 pstring/L {} long\n", "A".repeat(127));
    assert!(Database::from_source(&source).is_err());
    let mut configuration = options(0, 64);
    configuration.check = false;
    let report = Database::source_action(
        &[SourceFragment {
            name: b"rules".to_vec(),
            bytes: source.into_bytes(),
        }],
        &TimeContext::default(),
        configuration,
    );
    assert!(report.database.is_err());
    assert!(!report.native_failure);
}
