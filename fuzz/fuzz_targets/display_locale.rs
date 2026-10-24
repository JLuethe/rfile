// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::sync::LazyLock;
static LOCALES: LazyLock<Vec<rfile_platform::Locale>> = LazyLock::new(|| {
    ["C", "C.UTF-8"]
        .into_iter()
        .map(|name| rfile_platform::Locale::new(name).unwrap())
        .collect()
});
fuzz_target!(|data: &[u8]| {
    if data.len() > 65536 {
        return;
    }
    for locale in &*LOCALES {
        let _ = locale.filename_width(data, false);
        let _ = locale.filename_width(data, true);
        for raw in [false, true] {
            let result = locale.filename(data, raw, 262144).unwrap();
            assert!(result.len() <= data.len() * 4);
            assert!(!result.contains(&0));
        }
        let result = locale.result(data, 262144).unwrap();
        assert!(result.len() <= data.len() * 4);
        assert!(!result.contains(&0));
        // Cap errors and malformed/incomplete conversions must still unwind
        // the guard; another conversion immediately follows each failure.
        let _ = locale.result(data, 1);
        assert_eq!(locale.result(b"ASCII", 5).unwrap(), b"ASCII");
    }
});
