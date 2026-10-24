// SPDX-License-Identifier: GPL-3.0-only
use rfile::{
    Engine, ExecutionControl, Flags, InspectionBudget, Limits, SemanticProfile, TimeContext,
    cdf::{self, PropertyValue},
    input::{InputView, SliceView},
};
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cdf")
            .join(format!("{name}.cfb")),
    )
    .unwrap()
}
fn engine(profile: SemanticProfile) -> Engine {
    Engine::builtin().unwrap().with_profile(profile)
}
fn budget() -> InspectionBudget {
    InspectionBudget::new(&Limits::default(), ExecutionControl::default()).unwrap()
}
#[test]
fn v3_v4_regular_mini_profiles_have_ordered_properties() {
    for name in [
        "summary-v3-mini",
        "summary-v3-regular",
        "summary-v4-mini",
        "summary-v4-regular",
    ] {
        let bytes = fixture(name);
        let pin = engine(SemanticProfile::Pinned548).classify(&bytes).unwrap();
        assert_eq!(
            pin.output, b"Composite Document File V2 Document, Cannot read section info",
            "{name}"
        );
        let facts = pin.compound_document.unwrap();
        assert_eq!(facts.property_sets.len(), 1, "{name}");
        assert_eq!(
            facts.property_sets[0].properties[0].value,
            PropertyValue::I16(1252)
        );
        assert_eq!(
            facts.property_sets[0].properties[1].value,
            PropertyValue::Bytes(b"Microsoft Word\0".to_vec())
        );
        let corrected = engine(SemanticProfile::CorrectedV1)
            .classify(&bytes)
            .unwrap();
        assert_eq!(corrected.output,b"Composite Document File V2 Document, Little Endian, Os: Windows, Version 6.0, Code page: 1252, Name of Creating Application: Microsoft Word");
        let mut mime = engine(SemanticProfile::CorrectedV1);
        mime.flags = Flags(Flags::MIME);
        assert_eq!(
            mime.classify(&bytes).unwrap().output,
            b"application/msword; charset=binary"
        );
    }
}
#[test]
fn scalar_bits_duplicate_ids_and_packed_vectors_are_retained() {
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("all-properties"))
        .unwrap();
    let facts = c.compound_document.unwrap();
    let props = &facts.property_sets[0].properties;
    assert_eq!(props.iter().filter(|p| p.id == 2).count(), 2);
    assert_eq!(
        props.iter().find(|p| p.id == 205).unwrap().value,
        PropertyValue::Vector(vec![
            PropertyValue::I16(-1),
            PropertyValue::I16(2),
            PropertyValue::I16(3)
        ])
    );
    assert_eq!(
        props.iter().find(|p| p.id == 208).unwrap().value,
        PropertyValue::Vector(vec![PropertyValue::I32(7), PropertyValue::I16(8)])
    );
    assert_eq!(
        props.iter().find(|p| p.id == 201).unwrap().value,
        PropertyValue::F32(1.25f32.to_bits())
    );
    assert_eq!(
        props.iter().find(|p| p.id == 202).unwrap().value,
        PropertyValue::F64((-2.5f64).to_bits())
    );
    assert!(matches!(
        props.iter().find(|p| p.id == 209).unwrap().value,
        PropertyValue::Opaque { .. }
    ));
    assert!(
        c.description
            .contains("Title: café, Title: duplicate title")
    );
    assert!(c.description.contains("Subject: 雪人!"));
    assert!(c.description.contains("0xcf: A; B"));
}
#[test]
fn additional_fmtid_has_separate_labels() {
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("two-sets"))
        .unwrap();
    assert_eq!(c.compound_document.unwrap().property_sets.len(), 2);
    assert!(c.description.contains("Title: first"));
    assert!(!c.description.contains("Title: custom"));
}
#[test]
fn clsid_overrides_application_mime() {
    let mut e = engine(SemanticProfile::CorrectedV1);
    e.flags = Flags(Flags::MIME);
    assert_eq!(
        e.classify(&fixture("msi-precedence")).unwrap().output,
        b"application/vnd.ms-msi; charset=binary"
    );
}
#[test]
fn unknown_codepage_is_a_diagnostic_with_original_bytes() {
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("unsupported-codepage"))
        .unwrap();
    assert!(c.diagnostics.iter().any(|s| s.contains("codepage 932")));
    assert_eq!(
        c.compound_document.unwrap().property_sets[0].properties[1].value,
        PropertyValue::Bytes(b"original\0".to_vec())
    );
}
#[test]
fn hwp_wins_and_missing_root_is_tolerated() {
    let c = engine(SemanticProfile::Pinned548)
        .classify(&fixture("hwp-precedence"))
        .unwrap();
    assert_eq!(
        c.output,
        b"Hancom HWP (Hangul Word Processor) file, version 5.0"
    );
    assert!(c.compound_document.unwrap().property_sets.is_empty());
    assert_eq!(
        engine(SemanticProfile::Pinned548)
            .classify(&fixture("missing-root"))
            .unwrap()
            .output,
        b"CDFV2 Microsoft Word"
    );
}
#[test]
fn catalog_is_bounded_and_keeps_original_utf16_names() {
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("catalog"))
        .unwrap();
    assert_eq!(c.output, b"Microsoft Thumbs.db [first.jpg, second.png]");
    assert_eq!(c.compound_document.unwrap().catalog.len(), 3);
}
#[test]
fn cycles_and_truncated_property_tables_do_not_fabricate_values() {
    let c = engine(SemanticProfile::Pinned548)
        .classify(&fixture("directory-chain-cycle"))
        .unwrap();
    assert_eq!(
        c.output,
        b"Composite Document File V2 Document, Can't read directory"
    );
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("property-overlap"))
        .unwrap();
    assert!(c.compound_document.unwrap().property_sets.is_empty());
    assert!(c.diagnostics.iter().any(|s| s.contains("overlap")));
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("directory-tree-cycle"))
        .unwrap();
    assert!(c.diagnostics.iter().any(|s| s.contains("cyclic directory")));
}
#[test]
fn vector_and_io_limits_propagate_instead_of_generic_fallback() {
    assert!(
        engine(SemanticProfile::Pinned548)
            .classify(&fixture("vector-limit"))
            .unwrap_err()
            .is_limit()
    );
    let data = fixture("summary-v3-mini");
    let b = InspectionBudget::new(
        &Limits {
            io_reads: 1,
            ..Limits::default()
        },
        ExecutionControl::default(),
    )
    .unwrap();
    let error = cdf::inspect(
        &mut SliceView::new(&data),
        &b,
        SemanticProfile::Pinned548,
        &TimeContext::default(),
    )
    .unwrap_err();
    assert_eq!(error.limit_detail().unwrap().resource, "IO requests");
}
#[test]
fn positional_inspection_reaches_beyond_initial_sample() {
    use std::io::{Seek, SeekFrom, Write};
    let path = std::env::temp_dir().join(format!("rfile-cdf-{}", std::process::id()));
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    f.write_all(&fixture("summary-v4-mini")).unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut e = engine(SemanticProfile::CorrectedV1);
    e.limits.bytes = 512;
    let result = e.classify_file(&mut f).unwrap();
    assert!(result.truncated);
    assert!(result.description.contains("Microsoft Word"));
    assert_eq!(f.stream_position().unwrap(), 0);
    f.seek(SeekFrom::Start(7)).unwrap();
    let offset_result = e.classify_file(&mut f).unwrap();
    assert!(offset_result.compound_document.is_none());
    assert_eq!(f.stream_position().unwrap(), 7);
    assert!(result.metrics.io_requests > 1);
    drop(f);
    std::fs::remove_file(path).unwrap();
}
struct Short<'a>(&'a [u8]);
impl InputView for Short<'_> {
    fn known_len(&self) -> Option<u64> {
        Some(100_000)
    }
    fn read_at_unaccounted(&mut self, offset: u64, out: &mut [u8]) -> rfile::Result<usize> {
        let input = self.0.get(offset as usize..).unwrap_or(&[]);
        let n = input.len().min(out.len());
        out[..n].copy_from_slice(&input[..n]);
        Ok(n)
    }
}
#[test]
fn short_random_reads_are_malformed_not_zero_padded_evidence() {
    let data = fixture("summary-v3-mini");
    let facts = cdf::inspect(
        &mut Short(&data[..600]),
        &budget(),
        SemanticProfile::Pinned548,
        &TimeContext::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(facts.failure_stage, Some("Can't read SAT"));
    assert!(facts.property_sets.is_empty());
}
#[test]
fn prefix_truncation_and_mutated_lengths_remain_bounded() {
    let e = engine(SemanticProfile::CorrectedV1);
    for name in ["summary-v3-mini", "all-properties", "catalog"] {
        let bytes = fixture(name);
        for cut in (0..bytes.len()).step_by(31) {
            let _ = e.classify(&bytes[..cut]);
        }
        for p in [24, 26, 28, 30, 32, 44, 48, 56, 60, 64, 68, 72, 76] {
            let mut b = bytes.clone();
            b[p..p + 4].fill(255);
            let _ = e.classify(&b);
        }
    }
}

#[test]
fn catalog_twelve_byte_header_is_retained_without_reading_past_it() {
    let c = engine(SemanticProfile::CorrectedV1)
        .classify(&fixture("catalog-header12"))
        .unwrap();
    assert_eq!(c.output, b"Microsoft Thumbs.db [first.jpg, second.png]");
    let facts = c.compound_document.unwrap();
    assert_eq!(facts.catalog[0].header_bytes.as_ref().unwrap().len(), 12);
}

#[test]
fn adjacent_unknown_property_offsets_cannot_underflow_opaque_range() {
    let mut bytes = vec![0u8; 80];
    for (at, n) in [
        (24, 1u32),
        (44, 48),
        (48, 32),
        (52, 2),
        (56, 1),
        (60, 24),
        (64, 2),
        (68, 25),
    ] {
        bytes[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    bytes[72..80].fill(255);
    assert!(cdf::parse_property_stream(&bytes, "test", &budget()).is_err());
}
