// SPDX-License-Identifier: GPL-3.0-only
use rfile::{
    ExecutionControl, InspectionBudget, Limits, SemanticProfile,
    codecs::{self, CodecId},
};
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/compression")
            .join(name),
    )
    .unwrap()
}
fn decode(name: &str, codec: CodecId, profile: SemanticProfile) -> rfile::Result<Vec<u8>> {
    let budget = InspectionBudget::new(&Limits::default(), ExecutionControl::default()).unwrap();
    Ok(codecs::decode(&fixture(name), codec, &budget, profile)?
        .bytes
        .to_vec())
}
#[test]
fn enabled_codecs_decode_independently_encoded_payload() {
    for (name, id) in [
        ("gzip", CodecId::Gzip),
        ("zlib", CodecId::Zlib),
        ("bzip2", CodecId::Bzip2),
        ("xz", CodecId::Xz),
        ("lzma", CodecId::Lzma),
        ("lzip", CodecId::Lzip),
        ("zstd", CodecId::Zstd),
        ("lz4", CodecId::Lz4),
        ("compress", CodecId::Compress),
        ("zip-stored", CodecId::Zip),
        ("zip-deflate", CodecId::Zip),
    ] {
        if !id.compiled() {
            continue;
        }
        for profile in [SemanticProfile::Pinned548, SemanticProfile::CorrectedV1] {
            assert_eq!(
                decode(name, id, profile).unwrap(),
                fixture("payload"),
                "{name} {profile:?}"
            );
        }
    }
}
#[test]
fn encoded_matrix_covers_members_filters_widths_and_empty_streams() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/compression/manifest.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let codec = match case["codec"].as_str().unwrap() {
            "gzip" => CodecId::Gzip,
            "zlib" => CodecId::Zlib,
            "bzip2" => CodecId::Bzip2,
            "xz" => CodecId::Xz,
            "lzma" => CodecId::Lzma,
            "lzip" => CodecId::Lzip,
            "zstd" => CodecId::Zstd,
            "lz4" => CodecId::Lz4,
            "compress" => CodecId::Compress,
            "zip" => CodecId::Zip,
            _ => panic!("unknown fixture codec"),
        };
        if !codec.compiled() {
            continue;
        }
        let name = case["name"].as_str().unwrap();
        for (profile, key) in [
            (SemanticProfile::Pinned548, "pinned_payload"),
            (SemanticProfile::CorrectedV1, "corrected_payload"),
        ] {
            let result =
                decode(name, codec, profile).unwrap_or_else(|e| panic!("{name} {profile:?}: {e}"));
            assert_eq!(
                result,
                fixture(case[key].as_str().unwrap()),
                "{name} {profile:?}"
            );
        }
    }
}

fn inspect(
    data: &[u8],
    id: CodecId,
    limits: Limits,
    profile: SemanticProfile,
) -> rfile::Result<codecs::DecodeOutcome> {
    let budget = InspectionBudget::new(&limits, ExecutionControl::default()).unwrap();
    let result = codecs::decode(data, id, &budget, profile);
    if result.is_err() {
        assert_eq!(budget.metrics().tracked_memory_live, 0);
    }
    result
}
fn ids() -> [(CodecId, &'static str); 10] {
    [
        (CodecId::Gzip, "gzip"),
        (CodecId::Zlib, "zlib"),
        (CodecId::Bzip2, "bzip2"),
        (CodecId::Xz, "xz"),
        (CodecId::Lzma, "lzma"),
        (CodecId::Lzip, "lzip"),
        (CodecId::Zstd, "zstd"),
        (CodecId::Lz4, "lz4"),
        (CodecId::Compress, "compress"),
        (CodecId::Zip, "zip-deflate"),
    ]
}
#[test]
fn capability_registry_and_feature_off_errors_agree() {
    for (id, name) in ids() {
        assert_eq!(id.compiled(), codecs::COMPILED.contains(&id.name()));
        if !id.compiled() {
            assert!(matches!(
                decode(name, id, SemanticProfile::Pinned548),
                Err(rfile::Error::Unsupported(_))
            ));
        }
        assert_eq!(codecs::probe(&fixture(name)), Some(id));
    }
}
#[test]
fn output_window_memory_work_and_member_caps_release_reservations() {
    for (id, name) in ids().into_iter().filter(|(id, _)| id.compiled()) {
        for limits in [
            Limits {
                bytes: 1,
                ..Limits::default()
            },
            Limits {
                codec_window: 1,
                ..Limits::default()
            },
            Limits {
                memory: 1,
                ..Limits::default()
            },
            Limits {
                work_bytes: 1,
                ..Limits::default()
            },
            Limits {
                codec_members: 0,
                ..Limits::default()
            },
        ] {
            let r = inspect(&fixture(name), id, limits, SemanticProfile::CorrectedV1);
            assert!(r.as_ref().is_err_and(|e| e.is_limit()), "{name}: {r:?}");
        }
    }
    for (id, name) in [
        (CodecId::Gzip, "gzip"),
        (CodecId::Zlib, "zlib"),
        (CodecId::Bzip2, "bzip2"),
        (CodecId::Xz, "xz"),
        (CodecId::Lzip, "lzip"),
        (CodecId::Zstd, "zstd"),
        (CodecId::Lz4, "lz4"),
    ] {
        if !id.compiled() {
            continue;
        }
        let data = fixture(&format!("{name}-concat"));
        let r = inspect(
            &data,
            id,
            Limits {
                codec_members: 1,
                ..Limits::default()
            },
            SemanticProfile::CorrectedV1,
        );
        assert!(r.as_ref().is_err_and(|e| e.is_limit()), "{name}: {r:?}");
    }
}
#[test]
fn cancellation_and_expired_deadlines_precede_decoding() {
    use rfile::{CancellationToken, Error};
    let token = CancellationToken::default();
    let budget = InspectionBudget::new(
        &Limits::default(),
        ExecutionControl {
            cancellation: Some(token.clone()),
            deadline: None,
        },
    )
    .unwrap();
    token.cancel();
    assert!(matches!(
        codecs::decode(
            &fixture("gzip"),
            CodecId::Gzip,
            &budget,
            SemanticProfile::Pinned548
        ),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        InspectionBudget::new(
            &Limits::default(),
            ExecutionControl {
                cancellation: None,
                deadline: Some(std::time::Instant::now())
            }
        ),
        Err(Error::DeadlineExceeded)
    ));
}
#[test]
fn every_incomplete_first_member_is_rejected() {
    for (id, name) in ids()
        .into_iter()
        .filter(|(id, _)| id.compiled() && *id != CodecId::Compress)
    {
        let data = fixture(name);
        let complete = inspect(&data, id, Limits::default(), SemanticProfile::Pinned548).unwrap();
        for n in 0..complete.facts.input_used {
            let r = inspect(
                &data[..n],
                id,
                Limits::default(),
                SemanticProfile::Pinned548,
            );
            assert!(
                r.is_err(),
                "{name} accepted incomplete prefix {n}/{}",
                complete.facts.input_used
            );
        }
    }
}
#[test]
fn payload_integrity_checks_and_absent_checks_are_distinct() {
    for (id, name) in ids().into_iter().filter(|(id, _)| id.compiled()) {
        let result = inspect(
            &fixture(name),
            id,
            Limits::default(),
            SemanticProfile::Pinned548,
        )
        .unwrap();
        let expected = if matches!(id, CodecId::Lzma | CodecId::Compress) {
            codecs::ChecksumState::Absent
        } else {
            codecs::ChecksumState::Verified
        };
        assert_eq!(result.facts.checksum, expected, "{name}");
        if expected == codecs::ChecksumState::Absent {
            continue;
        }
        let mut data = fixture(name);
        let n = result.facts.input_used;
        // For ZIP the CRC is in its local header, other streams end in their
        // checksum/footer (bzip2's final CRC is not necessarily byte aligned).
        let at = if id == CodecId::Zip { 14 } else { n - 2 };
        data[at] ^= 0x80;
        assert!(
            inspect(&data, id, Limits::default(), SemanticProfile::Pinned548).is_err(),
            "{name} damaged integrity accepted"
        );
    }
    if CodecId::Xz.compiled() {
        let r = inspect(
            &fixture("xz-check-0"),
            CodecId::Xz,
            Limits::default(),
            SemanticProfile::Pinned548,
        )
        .unwrap();
        assert_eq!(r.facts.checksum, codecs::ChecksumState::Absent);
    }
}
#[test]
fn trailing_bytes_and_skippable_frames_follow_explicit_profiles() {
    for (id, name) in ids().into_iter().filter(|(id, _)| {
        id.compiled() && !matches!(id, CodecId::Compress | CodecId::Zip | CodecId::Lzma)
    }) {
        let mut data = fixture(name);
        data.extend(b"garbage");
        assert!(inspect(&data, id, Limits::default(), SemanticProfile::Pinned548).is_ok());
        assert!(inspect(&data, id, Limits::default(), SemanticProfile::CorrectedV1).is_err());
    }
    for (id, name) in [(CodecId::Lz4, "lz4"), (CodecId::Zstd, "zstd")] {
        if !id.compiled() {
            continue;
        }
        let mut skip = b"\x50\x2a\x4d\x18\x03\0\0\0abc".to_vec();
        skip.extend(fixture(name));
        assert_eq!(
            codecs::probe_with_profile(&skip, SemanticProfile::Pinned548),
            None
        );
        assert_eq!(
            codecs::probe_with_profile(&skip, SemanticProfile::CorrectedV1),
            Some(id)
        );
        let r = inspect(&skip, id, Limits::default(), SemanticProfile::CorrectedV1).unwrap();
        assert_eq!(&*r.bytes, fixture("payload"));
        assert_eq!(r.facts.members, 2);
        let r = inspect(
            &skip,
            id,
            Limits {
                codec_members: 1,
                ..Limits::default()
            },
            SemanticProfile::CorrectedV1,
        );
        assert!(r.unwrap_err().is_limit());
    }
}
