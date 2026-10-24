#![no_main]
use libfuzzer_sys::fuzz_target;
static BASE: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
    rfile::Database::from_source("0 byte x value\n")
        .unwrap()
        .compile()
        .unwrap()
});
fuzz_target!(|data: &[u8]| {
    let mut b = BASE.clone();
    let n = data.len().min(63);
    b[432 + 160..432 + 224].fill(0);
    b[432 + 160..432 + 160 + n].copy_from_slice(&data[..n]);
    if let Ok(db) = rfile::Database::from_compiled(&b) {
        let _ = rfile::Engine::new(std::sync::Arc::new(db)).classify(b"ABCD");
    }
});
