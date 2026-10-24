// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 || data.len() > 65536 {
        return;
    }
    let split = u16::from_le_bytes([data[0], data[1]]) as usize % (data.len() - 1) + 2;
    if let Ok(text) = std::str::from_utf8(&data[2..split]) {
        if let Ok(db) = rfile::Database::from_source(text) {
            let mut e = rfile::Engine::new(std::sync::Arc::new(db));
            e.limits.steps = 10_000;
            e.limits.names = 16;
            e.limits.indirect = 16;
            e.limits.work_bytes = 1024 * 1024;
            e.limits.output = 8192;
            let _ = e.classify(&data[split..]);
        }
    }
});
