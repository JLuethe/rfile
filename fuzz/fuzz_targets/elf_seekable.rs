// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
use rfile::{ExecutionControl, InspectionBudget, Limits, SemanticProfile, elf, input::SliceView};
fuzz_target!(|bytes: &[u8]| {
    let limits = Limits {
        memory: 16 * 1024 * 1024,
        io_bytes: 8 * 1024 * 1024,
        io_reads: 4096,
        steps: 100_000,
        work_bytes: 8 * 1024 * 1024,
        elf_phnum: 256,
        elf_shnum: 1024,
        elf_notes: 64,
        elf_shsize: 2 * 1024 * 1024,
        output: 256 * 1024,
        ..Limits::default()
    };
    for profile in [SemanticProfile::Pinned548, SemanticProfile::CorrectedV1] {
        let budget = InspectionBudget::new(&limits, ExecutionControl::default()).unwrap();
        let _ = elf::inspect(
            &mut SliceView::new(bytes),
            &bytes[..bytes.len().min(512)],
            elf::Options {
                profile,
                ..elf::Options::default()
            },
            &budget,
        );
    }
});
