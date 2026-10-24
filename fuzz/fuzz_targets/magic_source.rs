#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if data.len() > 65536 {
        return;
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(db) = rfile::Database::from_source(text) {
            let compiled = db.compile().unwrap();
            assert!(rfile::Database::from_compiled(&compiled).is_ok());
        }
    }
});
