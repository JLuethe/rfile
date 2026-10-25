// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, Engine, Flags};
use std::sync::Arc;
#[test]
fn complete_buffer_ignores_file_sample_without_mutating_the_engine() {
    let db = Database::from_source("20 string MARK late-marker\n").unwrap();
    let mut engine = Engine::new(Arc::new(db));
    engine.flags = Flags(Flags::NO_TEXT | Flags::NO_ENCODING);
    engine.limits.bytes = 8;
    let mut input = vec![0; 24];
    input[20..].copy_from_slice(b"MARK");
    assert_ne!(engine.classify(&input).unwrap().output, b"late-marker");
    assert_eq!(
        engine.classify_complete_buffer(&input).unwrap().output,
        b"late-marker"
    );
    assert_eq!(engine.limits.bytes, 8);
    engine.limits.bytes = 0;
    assert_eq!(
        engine.classify_complete_buffer(&input).unwrap().output,
        b"late-marker"
    );
}
#[test]
fn complete_buffer_still_obeys_execution_and_output_limits() {
    let mut engine = Engine::new(Arc::new(
        Database::from_source("0 string hello found\n").unwrap(),
    ));
    engine.limits.steps = 0;
    assert!(
        engine
            .classify_complete_buffer(b"hello")
            .unwrap_err()
            .is_limit()
    );
    engine.limits.steps = 1_000_000;
    engine.limits.output = 1;
    assert!(
        engine
            .classify_complete_buffer(b"hello")
            .unwrap_err()
            .is_limit()
    );
}
