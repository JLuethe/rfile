// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
use rfile::{
    ExecutionControl, InspectionBudget, Limits, SemanticProfile, TimeContext, input::SliceView,
};
fuzz_target!(|bytes: &[u8]| {
    let limits = Limits {
        memory: 16 * 1024 * 1024,
        io_bytes: 4 * 1024 * 1024,
        io_reads: 1024,
        steps: 20_000,
        work_bytes: 4 * 1024 * 1024,
        ..Limits::default()
    };
    for profile in [SemanticProfile::Pinned548, SemanticProfile::CorrectedV1] {
        let budget = InspectionBudget::new(&limits, ExecutionControl::default()).unwrap();
        let _ = rfile::cdf::inspect(
            &mut SliceView::new(bytes),
            &budget,
            profile,
            &TimeContext::default(),
        );
    }
});
