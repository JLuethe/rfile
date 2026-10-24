// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
use rfile::{Database, Engine, Flags, Limits, SemanticProfile};
use std::sync::{Arc, LazyLock};
const PATTERNS: &[&str] = &[
    r"(ab)\1",
    r"(a|ab)\1",
    r"((a)|b)*\2",
    r"(a*)\1",
    r"(a?)*\1",
    r"(a*)*\1",
    r"(a|aa)*\1",
    r"(a|aa)+\1",
    r"(a){0,2}\1",
    r"(a)\1{2}",
    r"((a)+\2)*\1",
    r"([^x]*)\1",
    r"([[:lower:]])\1",
    r"(a|)\1",
    r"()\1",
    r"(a)(b)\2\1",
    r"\d+",
    r"[\d]+",
    r"[^]\]+",
    r"\b(a+)\1\b",
];
fn database(pattern: &[u8]) -> rfile::Result<Database> {
    let literal = pattern
        .iter()
        .map(|b| format!("\\x{b:02x}"))
        .collect::<String>();
    Database::from_source(&format!("0 regex/bc {literal} match=%s\n"))
}
static RULES: LazyLock<Vec<Arc<Database>>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .map(|p| Arc::new(database(p.as_bytes()).unwrap()))
        .collect()
});
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 || data.len() > 65536 {
        return;
    }
    let (db, input) = if data[0] as usize >= PATTERNS.len() {
        let n = (data[1] as usize).min(100).min(data.len() - 2);
        let Ok(db) = database(&data[2..2 + n]) else {
            return;
        };
        (Arc::new(db), &data[2 + n..])
    } else {
        (RULES[data[0] as usize].clone(), &data[1..])
    };
    for profile in [SemanticProfile::Pinned548, SemanticProfile::CorrectedV1] {
        let mut engine = Engine::new(db.clone());
        engine.profile = profile;
        engine.flags = Flags(0xffb000);
        engine.limits = Limits {
            bytes: 65536,
            regex: 4096,
            regex_steps: 200000,
            regex_states: 4096,
            steps: 250000,
            work_bytes: 16 * 1024 * 1024,
            ..Limits::default()
        };
        let _ = engine.classify(input);
    }
});
