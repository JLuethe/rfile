#![no_main]
use libfuzzer_sys::fuzz_target;
static ENGINE: std::sync::LazyLock<rfile::Engine> = std::sync::LazyLock::new(|| {
    let mut e = rfile::Engine::builtin().unwrap();
    e.limits.bytes = 65536;
    e.limits.steps = 100_000;
    e.limits.work_bytes = 8 * 1024 * 1024;
    e.limits.output = 65536;
    e
});
fuzz_target!(|data: &[u8]| if data.len() <= 65536 {
    let _ = ENGINE.classify(data);
});
