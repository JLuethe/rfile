// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
static BASE: std::sync::LazyLock<Vec<Vec<u8>>> = std::sync::LazyLock::new(|| {
    ["byte", "lefloat", "ledouble"]
        .iter()
        .map(|kind| {
            rfile::Database::from_source(&format!("0 {kind} x value\n"))
                .unwrap()
                .compile()
                .unwrap()
        })
        .collect()
});
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let mut b = BASE[data[0] as usize % BASE.len()].clone();
    let n = (data[1] as usize % 64).min(data.len() - 2);
    b[432 + 160..432 + 224].fill(0);
    b[432 + 160..432 + 160 + n].copy_from_slice(&data[2..2 + n]);
    if let Ok(db) = rfile::Database::from_compiled(&b) {
        let db = std::sync::Arc::new(db);
        for profile in [
            rfile::SemanticProfile::Pinned548,
            rfile::SemanticProfile::CorrectedV1,
        ] {
            let _ = rfile::Engine::new(db.clone())
                .with_profile(profile)
                .classify(&data[2 + n..]);
        }
    }
});
