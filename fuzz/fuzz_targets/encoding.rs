// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
static ENGINE: std::sync::LazyLock<rfile::Engine> = std::sync::LazyLock::new(|| {
    let mut e = rfile::Engine::new(std::sync::Arc::new(
        rfile::Database::from_source("").unwrap(),
    ));
    e.flags = rfile::Flags(rfile::Flags::NO_SOFT);
    e.limits.bytes = 65536;
    e
});
fuzz_target!(|data: &[u8]| if data.len() <= 65536 {
    let _ = ENGINE.classify(data);
});
