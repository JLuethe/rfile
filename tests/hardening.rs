use rfile::{Database, Engine, Error, Flags};
use std::{io::Write, sync::Arc, time::Instant};
fn small_db() -> Database {
    Database::from_source("0 string TEST sample\n>4 lelong x \\b, %u\n").unwrap()
}
#[test]
fn database_byte_order_is_explicit() {
    let original = small_db().compile().unwrap();
    let mut big = original.clone();
    for p in (0..16).step_by(4) {
        big[p..p + 4].reverse();
    }
    for p in (432..big.len()).step_by(432) {
        big[p..p + 2].reverse();
        for offset in [12, 16, 20] {
            big[p + offset..p + offset + 4].reverse();
        }
        if big[p + 6] == 5 {
            big[p + 24..p + 28].reverse();
            big[p + 28..p + 32].reverse();
        } else {
            big[p + 24..p + 32].reverse();
            big[p + 32..p + 40].reverse();
        }
    }
    let db = Database::from_compiled(&big).unwrap();
    assert_eq!(db.compile().unwrap(), original);
    let e = Engine::new(Arc::new(db));
    assert_eq!(e.classify(b"TEST\x04\0\0\0").unwrap().output, b"sample, 4");
}
#[test]
fn parser_rejects_unsafe_formats_and_literal_overflow() {
    for s in [
        "0 byte x %n\n",
        "0 byte x %*d\n",
        "0 byte 256 nope\n",
        "0 ushort 65536 nope\n",
        "0 string A %p\n",
        "0 byte x %999999999999999999999u\n",
    ] {
        assert!(Database::from_source(s).is_err(), "{s}");
    }
}
#[test]
fn work_budget_covers_whitespace_search() {
    let mut e = Engine::new(Arc::new(
        Database::from_source("0 search/2000/W \\040X hit\n").unwrap(),
    ));
    e.limits.work_bytes = 1000;
    assert!(matches!(
        e.classify(&vec![b' '; 2000]),
        Err(Error::Limit(_))
    ));
}
#[test]
fn decompression_bomb_is_a_limit_error() {
    let mut compressed = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    compressed.write_all(&vec![b'a'; 100_000]).unwrap();
    let b = compressed.finish().unwrap();
    let mut e = Engine::new(Arc::new(small_db()));
    e.flags = Flags(Flags::COMPRESS);
    e.limits.bytes = 1024;
    assert!(matches!(
        e.classify(&b),
        Err(Error::Limit("decompression output"))
    ));
}
#[test]
fn compiled_description_cannot_call_c_printf() {
    let mut b = small_db().compile().unwrap();
    b[432 + 160..432 + 224].fill(0);
    b[432 + 160..432 + 162].copy_from_slice(b"%n");
    let e = Engine::new(Arc::new(Database::from_compiled(&b).unwrap()));
    assert!(matches!(
        e.classify(b"TEST\0\0\0\0"),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn byte_order_masks_and_signed_boundaries() {
    let e = Engine::new(Arc::new(
        Database::from_source(
            "0 byte 0xff negative-%d\n>1 byte -255 wrapped-%d\n>2 ubyte 0xff unsigned-%u\n",
        )
        .unwrap(),
    ));
    assert_eq!(
        e.classify(&[255, 1, 255]).unwrap().output,
        b"negative--1 wrapped-1 unsigned-255"
    );
}

#[test]
#[ignore = "deterministic mutation campaign; run explicitly with --release --ignored"]
fn deterministic_mutation_campaign() {
    let started = Instant::now();
    let mut state = 0x4f6c655275737421u64;
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }
    let original = small_db().compile().unwrap();
    let e = Engine::builtin().unwrap();
    let mut db_cases = 0;
    let mut source_cases = 0;
    let mut input_cases = 0;
    for _ in 0..10_000 {
        let mut bytes = original.clone();
        for _ in 0..1 + (next(&mut state) % 8) {
            let p = next(&mut state) as usize % bytes.len();
            bytes[p] = next(&mut state) as u8;
        }
        let length = if next(&mut state) % 4 == 0 {
            next(&mut state) as usize % bytes.len()
        } else {
            bytes.len()
        };
        if let Ok(db) = Database::from_compiled(&bytes[..length]) {
            let mut engine = Engine::new(Arc::new(db));
            engine.limits.steps = 10_000;
            let _ = engine.classify(b"TEST\0\0\0\0");
        }
        db_cases += 1;
    }
    const ALPHABET: &[u8] = b">0()&+-*/% \\0123456789abcdefuxstringregex!:=\n\t[]{}";
    for _ in 0..10_000 {
        let n = (next(&mut state) % 256) as usize;
        let bytes: Vec<_> = (0..n)
            .map(|_| ALPHABET[next(&mut state) as usize % ALPHABET.len()])
            .collect();
        let s = std::str::from_utf8(&bytes).unwrap();
        if let Ok(db) = Database::from_source(s) {
            let _ = Engine::new(Arc::new(db)).classify(b"TEST\0\0\0\0");
        }
        source_cases += 1;
    }
    for _ in 0..10_000 {
        let n = (next(&mut state) % 1024) as usize;
        let bytes: Vec<_> = (0..n).map(|_| next(&mut state) as u8).collect();
        let _ = e.classify(&bytes);
        input_cases += 1;
    }
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("upstream/tests");
    let mut fixture_cases = 0;
    if let Ok(entries) = std::fs::read_dir(fixtures) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "testfile") {
                let b = std::fs::read(entry.path()).unwrap();
                for n in [
                    0,
                    1,
                    2,
                    3,
                    8,
                    16,
                    64,
                    128,
                    256,
                    512,
                    b.len() / 2,
                    b.len().saturating_sub(1),
                    b.len(),
                ] {
                    let _ = e.classify(&b[..n.min(b.len())]);
                    fixture_cases += 1;
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"seed":"4f6c655275737421","database_cases":db_cases,"source_cases":source_cases,"random_input_cases":input_cases,"fixture_truncations":fixture_cases,"elapsed_seconds":started.elapsed().as_secs_f64()})
    );
}
#[test]
fn output_limit_includes_fallbacks_and_escaping() {
    let mut e = Engine::new(Arc::new(small_db()));
    e.limits.output = 1;
    assert!(matches!(e.classify(b""), Err(Error::Limit("output"))));
    let db = Database::from_source("0 string AB \\b\x01\x01\x01\n").unwrap();
    let mut e = Engine::new(Arc::new(db));
    e.limits.output = 4;
    assert!(matches!(e.classify(b"ABCD"), Err(Error::Limit("output"))));
}
#[test]
fn concurrent_regex_cache_is_owned_and_synchronized() {
    let db = Database::from_source("0 regex a|ab [%s]\n").unwrap();
    let e = Arc::new(Engine::new(Arc::new(db)));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let e = e.clone();
            std::thread::spawn(move || {
                for _ in 0..100 {
                    assert_eq!(e.classify(b"ab\n").unwrap().output, b"[ab], ASCII text");
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}
#[test]
fn source_directory_entry_limit_precedes_parsing() {
    let path = std::env::temp_dir().join(format!("rfile-dir-limit-{}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    for i in 0..4097 {
        std::fs::write(path.join(format!("rule-{i:04}")), b"# empty\n").unwrap();
    }
    let result = Database::load_source(&path);
    std::fs::remove_dir_all(&path).unwrap();
    assert!(matches!(result, Err(Error::Limit("magic source files"))));
}
