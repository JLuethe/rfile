// SPDX-License-Identifier: GPL-3.0-only
use rfile::{
    Engine, ExecutionControl, Flags, InspectionBudget, Limits, SemanticProfile, elf,
    input::SliceView,
};
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/elf")
            .join(format!("{name}.elf")),
    )
    .unwrap()
}
fn inspect(
    bytes: &[u8],
    limits: Limits,
    profile: SemanticProfile,
) -> rfile::Result<Option<elf::Info>> {
    let budget = InspectionBudget::new(&limits, ExecutionControl::default())?;
    elf::inspect(
        &mut SliceView::new(bytes),
        &bytes[..bytes.len().min(512)],
        elf::Options {
            profile,
            ..elf::Options::default()
        },
        &budget,
    )
}
fn info(name: &str) -> elf::Info {
    inspect(
        &fixture(name),
        Limits::default(),
        SemanticProfile::Pinned548,
    )
    .unwrap()
    .unwrap()
}
#[test]
fn all_generated_layouts_parse_without_native_struct_casts() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/elf/manifest.json")).unwrap();
    for row in manifest.as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let result = inspect(
            &fixture(name),
            Limits::default(),
            SemanticProfile::Pinned548,
        );
        if name == "auxv-limit" {
            assert!(matches!(result, Err(rfile::Error::LegacyLimit(_))));
        } else if name == "interpreter-long" {
            assert!(matches!(
                result,
                Err(rfile::Error::LegacyOutput {
                    attempted: 1615,
                    previous: 19
                })
            ));
        } else {
            assert!(result.unwrap().is_some(), "{name}");
        }
    }
}
#[test]
fn process_and_auxv_fields_work_in_all_byte_orders() {
    for prefix in ["32-le", "32-be", "64-le", "64-be"] {
        let i = info(&format!("{prefix}-core-auxv"));
        let text = String::from_utf8(i.suffix).unwrap();
        assert!(
            text.contains(", SVR4-style, from 'rfile --probe'"),
            "{prefix}: {text}"
        );
        assert!(text.contains("execfn: '/usr/bin/rfile'"));
        assert!(text.contains("platform: 'x86_64'"));
        assert!(
            text.contains(
                "real uid: 1000, effective uid: 1001, real gid: 1002, effective gid: 1003"
            )
        );
        assert_eq!(i.notes.len(), 2);
        assert_eq!(i.notes[0].owner, b"CORE\0");
        assert!(i.notes[0].source_offset > 0);
        let text = String::from_utf8(info(&format!("{prefix}-freebsd-core")).suffix).unwrap();
        assert!(text.contains("FreeBSD-style, from 'hello', pid=1234"));
        let text = String::from_utf8(info(&format!("{prefix}-netbsd-core")).suffix).unwrap();
        assert!(text.contains("NetBSD-style, from 'hello', pid=1234, uid=1000, gid=1001, nlwps=4, lwp=3 (signal 11/code 2)"));
    }
}
#[test]
fn repeated_go_notes_remain_repeated_and_os_flags_suppress_duplicates() {
    let text = String::from_utf8(info("64-le-duplicate-go").suffix).unwrap();
    assert!(text.contains("Go BuildID=first, Go BuildID=second"));
    let text = String::from_utf8(info("64-le-android").suffix).unwrap();
    assert!(text.contains("for Android 34, built by NDK r26d (12345)"));
}
#[test]
fn source_version_boundary_encodings_and_flag_labels_are_exact() {
    for (name, expected) in [
        ("freebsd-460002", "for FreeBSD 4.6.2"),
        ("freebsd-460100", "for FreeBSD 4.6 (460100)"),
        ("freebsd-491000", "for FreeBSD 4.10"),
        ("netbsd-old", "for NetBSD 8.0ZD"),
        ("netbsd-new", "for NetBSD 10.0.1502"),
        (
            "pax",
            "PaX: +mprotect,-mprotect,+segvguard,-segvguard,+ASLR,-ASLR",
        ),
        ("memtag", "Android Memtag: none,async,sync,heap,stack"),
        ("netbsd-emulation", "emulation:: linux"),
    ] {
        let text = String::from_utf8(info(&format!("64-le-{name}")).suffix).unwrap();
        assert!(text.contains(expected), "{name}: {text}");
    }
}
#[test]
fn zero_alignment_defaults_to_four_and_nonpower_alignment_uses_division() {
    for name in [
        "64-le-align-0",
        "64-le-align-4",
        "64-le-align-6",
        "64-le-align-8",
    ] {
        let text = String::from_utf8(info(name).suffix).unwrap();
        assert!(text.contains("BuildID[sha1]"), "{name}: {text}");
        assert!(!text.contains("invalid note alignment"));
    }
    assert!(
        String::from_utf8(info("64-le-align-2147483648").suffix)
            .unwrap()
            .contains("invalid note alignment 0x80000000")
    );
}
#[test]
fn solaris_capabilities_keep_source_table_order() {
    let text = String::from_utf8(info("64-le-sun-capabilities").suffix).unwrap();
    assert!(
        text.ends_with(", stripped, uses FPU SSE SSE2, uses frame pointer"),
        "{text}"
    );
}
#[test]
fn descriptor_enrichment_is_not_added_to_buffer_classification() {
    let bytes = fixture("64-le-core-auxv");
    let e = Engine::builtin().unwrap();
    let c = e.classify(&bytes).unwrap();
    assert!(c.elf.is_none());
    assert!(!c.description.contains("execfn:"));
    let path = std::env::temp_dir().join(format!("rfile-elf-{}", std::process::id()));
    std::fs::write(&path, &bytes).unwrap();
    let c = e.classify_path(&path).unwrap();
    assert!(c.elf.is_some());
    assert!(c.description.contains("execfn:"));
    let mut e = e;
    e.flags = Flags(Flags::NO_ELF);
    assert!(e.classify_path(&path).unwrap().elf.is_none());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn io_memory_and_output_limits_remain_errors() {
    let bytes = fixture("64-le-core-auxv");
    for limits in [
        Limits {
            io_reads: 0,
            ..Limits::default()
        },
        Limits {
            memory: 1,
            ..Limits::default()
        },
        Limits {
            output: 1,
            ..Limits::default()
        },
    ] {
        assert!(
            inspect(&bytes, limits, SemanticProfile::Pinned548)
                .unwrap_err()
                .is_limit()
        );
    }
}
#[test]
fn truncations_and_mutated_note_headers_are_bounded() {
    for name in [
        "64-le-core-auxv",
        "32-be-section-notes",
        "64-le-sun-capabilities",
    ] {
        let bytes = fixture(name);
        for cut in 0..bytes.len() {
            let _ = inspect(&bytes[..cut], Limits::default(), SemanticProfile::Pinned548);
        }
        for p in 16..bytes.len().saturating_sub(8) {
            let mut b = bytes.clone();
            b[p..p + 8].fill(255);
            let _ = inspect(
                &b,
                Limits {
                    steps: 10000,
                    io_reads: 1000,
                    ..Limits::default()
                },
                SemanticProfile::Pinned548,
            );
        }
    }
}

#[test]
fn extended_counts_are_a_named_corrected_profile_behavior() {
    let bytes = fixture("extended-counts");
    let pin = inspect(&bytes, Limits::default(), SemanticProfile::Pinned548)
        .unwrap()
        .unwrap();
    assert_eq!(pin.suffix, b", too many program (65535)");
    let corrected = inspect(&bytes, Limits::default(), SemanticProfile::CorrectedV1)
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(corrected.suffix)
            .unwrap()
            .contains("not stripped")
    );
}
#[test]
fn note_count_is_legacy_annotation_and_large_printf_is_exact_error() {
    let text = String::from_utf8(info("note-count-256").suffix).unwrap();
    assert!(text.ends_with(", stripped, too many notes (256)"), "{text}");
    let e = inspect(
        &fixture("interpreter-long"),
        Limits::default(),
        SemanticProfile::Pinned548,
    )
    .unwrap_err();
    assert_eq!(e.to_string(), "Output buffer space exceeded 1615+19");
}
#[test]
fn reader_spools_elf_for_random_access_with_a_separate_finite_cap() {
    let bytes = fixture("64-le-core-auxv");
    let mut e = Engine::builtin().unwrap();
    e.limits.bytes = 128;
    let c = e.classify_reader(&mut bytes.as_slice()).unwrap();
    assert!(c.elf.is_some());
    assert!(c.description.contains("platform: 'x86_64'"));
    assert!(c.truncated);
    e.limits.stream_bytes = bytes.len() - 1;
    assert!(
        e.classify_reader(&mut bytes.as_slice())
            .unwrap_err()
            .is_limit()
    );
    e.flags = Flags(Flags::NO_ELF);
    assert!(
        e.classify_reader(&mut bytes.as_slice())
            .unwrap()
            .elf
            .is_none()
    );
}
