// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
use rfile::{ExecutionControl, InspectionBudget, Limits};
fuzz_target!(|bytes: &[u8]| {
    let limits = Limits {
        memory: 16 * 1024 * 1024,
        steps: 20_000,
        work_bytes: 4 * 1024 * 1024,
        ..Limits::default()
    };
    let budget = InspectionBudget::new(&limits, ExecutionControl::default()).unwrap();
    let _ = rfile::cdf::parse_property_stream(bytes, "fuzz", &budget);
});
