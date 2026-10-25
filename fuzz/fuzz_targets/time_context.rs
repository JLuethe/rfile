// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
use rfile::{Database, Engine, Flags, SemanticProfile, TimeContext};
use std::sync::{Arc, LazyLock};
static RULES: LazyLock<Arc<Database>> = LazyLock::new(|| {
    Arc::new(Database::from_source("0 leqldate x unix=%s\n0 leqwdate x windows=%s\n").unwrap())
});
fuzz_target!(|data: &[u8]| {
    if data.len() < 9 {
        return;
    }
    let context = if data[0] & 1 == 0 {
        let Ok(expression) = std::str::from_utf8(&data[9..]) else {
            return;
        };
        TimeContext::posix(expression)
    } else {
        TimeContext::from_tzif("fuzz", &data[9..])
    };
    if let Ok(context) = context {
        for profile in [SemanticProfile::Pinned548, SemanticProfile::CorrectedV1] {
            let mut engine = Engine::new(RULES.clone())
                .with_time_context(context.clone())
                .with_profile(profile);
            engine.flags = Flags(0xffb020);
            let _ = engine.classify(&data[1..9]);
        }
    }
});
