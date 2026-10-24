// SPDX-License-Identifier: GPL-3.0-only
#![no_main]
use libfuzzer_sys::fuzz_target;
const CODEC: rfile::codecs::CodecId = rfile::codecs::CodecId::Lzma;
include!("codec_driver.inc");
