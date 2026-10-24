#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if data.len() > 1024 * 1024 {
        return;
    }
    if let Ok(db) = rfile::Database::from_compiled(data) {
        let compiled = db.compile().unwrap();
        assert!(rfile::Database::from_compiled(&compiled).is_ok());
        let mut e = rfile::Engine::new(std::sync::Arc::new(db));
        e.limits.steps = 10_000;
        e.limits.names = 16;
        e.limits.work_bytes = 1024 * 1024;
        let _ = e.classify(b"TEST\0\0\0\0");
    }
});
