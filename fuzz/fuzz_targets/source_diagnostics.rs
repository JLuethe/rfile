// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if data.len() < 3 || data.len() > 65536 {
        return;
    }
    let source = rfile::SourceFragment {
        name: data[..3].to_vec(),
        bytes: data[3..].to_vec(),
    };
    let options = rfile::SourceActionOptions {
        check: data[0] & 1 != 0,
        warning_count: usize::from(data[1] % 65),
        warning_limit: usize::from(data[2] % 65),
        output_limit: 8192,
    };
    let report = rfile::Database::source_action(&[source], &rfile::TimeContext::default(), options);
    assert!(report.output.len() <= 8192);
    if let Ok(db) = report.database {
        let bytes = db.compile().expect("accepted bounded source compiles");
        let imported = rfile::Database::from_compiled(&bytes).expect("accepted source round trips");
        let mut engine = rfile::Engine::new(std::sync::Arc::new(imported));
        engine.limits.steps = 10000;
        engine.limits.names = 8;
        engine.limits.indirect = 8;
        engine.limits.output = 8192;
        let _ = engine.classify(data);
    }
});
