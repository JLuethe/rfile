// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, Engine, SemanticProfile, TimeContext};
use std::sync::Arc;
fn date(kind: &str, n: u64, time: TimeContext, profile: SemanticProfile) -> Vec<u8> {
    let source = format!("0 {kind} x %s\n");
    Engine::new(Arc::new(Database::from_source(&source).unwrap()))
        .with_time_context(time)
        .with_profile(profile)
        .classify(&n.to_le_bytes())
        .unwrap()
        .output
}
#[test]
fn local_and_utc_families_use_distinct_immutable_contexts() {
    let pst = TimeContext::posix("PST8PDT,M3.2.0,M11.1.0").unwrap();
    assert_eq!(
        date("leqldate", 0, pst.clone(), SemanticProfile::Pinned548),
        b"Wed Dec 31 16:00:00 1969"
    );
    assert_eq!(
        date("leqdate", 0, pst, SemanticProfile::Pinned548),
        b"Thu Jan  1 00:00:00 1970"
    );
    assert_eq!(
        date(
            "leqldate",
            0,
            TimeContext::default(),
            SemanticProfile::Pinned548
        ),
        b"Thu Jan  1 00:00:00 1970"
    );
}
#[test]
fn dst_transition_uses_timestamp_not_ambiguous_wall_time() {
    let tz = TimeContext::posix("PST8PDT,M3.2.0,M11.1.0").unwrap();
    assert_eq!(
        date(
            "leqldate",
            1_710_064_799,
            tz.clone(),
            SemanticProfile::Pinned548
        ),
        b"Sun Mar 10 01:59:59 2024"
    );
    assert_eq!(
        date("leqldate", 1_710_064_800, tz, SemanticProfile::Pinned548),
        b"Sun Mar 10 03:00:00 2024"
    );
}
#[test]
fn corrected_filetime_is_explicit_and_preserves_pinned_quirk() {
    let n = 116_470_656_000_000_000;
    assert_eq!(
        date(
            "leqwdate",
            n,
            TimeContext::default(),
            SemanticProfile::CorrectedV1
        ),
        b"Sat Jan 31 00:00:00 1970"
    );
    // The reference calendar normalization moves this boundary one day back.
    assert_eq!(
        date(
            "leqwdate",
            n,
            TimeContext::default(),
            SemanticProfile::Pinned548
        ),
        b"Wed Dec 31 00:00:00 1969"
    );
}
#[test]
fn checked_tzif_and_named_path_fail_before_parsing_or_traversal() {
    assert!(
        TimeContext::from_tzif("large", &vec![0; 1024 * 1024 + 1])
            .unwrap_err()
            .is_limit()
    );
    assert!(TimeContext::from_tzif("invalid", b"TZif").is_err());
    assert!(
        TimeContext::named("../etc/passwd", std::path::Path::new("/usr/share/zoneinfo")).is_err()
    );
}
#[test]
fn known_backreference_matches_source_and_imported_database() {
    let good = Database::from_source("0 regex (ab)\\\\1 match=%s\n").unwrap();
    for db in [
        Database::from_compiled(&good.compile().unwrap()).unwrap(),
        good,
    ] {
        assert_eq!(
            Engine::new(Arc::new(db))
                .classify(b"abab\n")
                .unwrap()
                .output,
            b"match=abab, ASCII text"
        );
    }
    assert!(
        Database::from_source("0 regex (ab)\\\\2 match=%s\n")
            .unwrap_err()
            .to_string()
            .contains("backreference")
    );
}

#[test]
fn escaped_digits_and_bracket_predicates_are_not_references() {
    assert!(Database::from_source("0 regex [1] match=%s\n").is_ok());
    // Magic unescaping leaves two regex backslashes, then a literal digit.
    assert!(Database::from_source("0 regex \\\\\\\\1 match=%s\n").is_ok());
    assert!(Database::from_source("0 regex [[:digit:]1] match=%s\n").is_ok());
    assert!(Database::from_source("0 regex (?i:a) match=%s\n").is_err());
}
#[test]
fn concurrent_contexts_do_not_modify_process_timezone() {
    let original = std::env::var_os("TZ");
    let threads: Vec<_> = ["IST-5:30", "NPT-5:45", "UTC0"]
        .into_iter()
        .map(|zone| {
            std::thread::spawn(move || {
                let time = TimeContext::posix(zone).unwrap();
                let expected = date("leqldate", 0, time.clone(), SemanticProfile::Pinned548);
                for _ in 0..20 {
                    assert_eq!(
                        date("leqldate", 0, time.clone(), SemanticProfile::Pinned548),
                        expected
                    );
                }
                expected
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_ne!(results[0], results[1]);
    assert_ne!(results[1], results[2]);
    assert_eq!(std::env::var_os("TZ"), original);
}

#[test]
fn upper_year_local_boundary_uses_checked_repeating_footer() {
    let utc = TimeContext::default();
    assert_eq!(
        date("leqldate", 253402300799, utc, SemanticProfile::Pinned548),
        b"Fri Dec 31 23:59:59 9999"
    );
    let pst = TimeContext::posix("PST8PDT,M3.2.0,M11.1.0").unwrap();
    assert_eq!(
        date("leqldate", 253402300800, pst, SemanticProfile::Pinned548),
        b"Fri Dec 31 16:00:00 9999"
    );
    let pst = TimeContext::posix("PST8PDT,M3.2.0,M11.1.0").unwrap();
    assert_eq!(
        date(
            "leqwdate",
            116444736000000000,
            pst,
            SemanticProfile::Pinned548
        ),
        b"Thu Jan  1 08:00:00 1970"
    );
}

#[test]
fn pinned_posix_uses_native_utc_year_boundaries() {
    let n = (-15_811_200i64) as u64;
    let tz = TimeContext::posix("PST8PDT,M3.2.0,M11.1.0").unwrap();
    assert_eq!(
        date("leqldate", n, tz.clone(), SemanticProfile::Pinned548),
        b"Tue Jul  1 16:00:00 1969"
    );
    assert_eq!(
        date("leqldate", n, tz, SemanticProfile::CorrectedV1),
        b"Tue Jul  1 17:00:00 1969"
    );
    let tz = TimeContext::posix("EST5EDT,0/0,J365/25").unwrap();
    assert_eq!(
        date(
            "leqldate",
            (-1i64) as u64,
            tz.clone(),
            SemanticProfile::Pinned548
        ),
        b"Wed Dec 31 18:59:59 1969"
    );
    assert_eq!(
        date("leqldate", (-1i64) as u64, tz, SemanticProfile::CorrectedV1),
        b"Wed Dec 31 19:59:59 1969"
    );
}

#[test]
fn pinned_filetime_preserves_short_standard_period_search() {
    let tz = TimeContext::posix("<STD>-5:45<DST>-6:15,0/0,J365/25").unwrap();
    assert_eq!(
        date(
            "leqwdate",
            133643520000000000,
            tz,
            SemanticProfile::Pinned548
        ),
        b"Mon Jul  1 18:45:00 2024"
    );
}
