// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| rfile_magic::fuzz_state_sequence(data));
