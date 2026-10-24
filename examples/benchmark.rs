// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
#![forbid(unsafe_code)]
use rfile::Engine;
use std::{io::Read, time::Instant};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let paths = std::fs::read_to_string(&args[1]).unwrap();
    let mut samples = Vec::new();
    for path in paths.lines() {
        let mut b = Vec::new();
        std::fs::File::open(path)
            .unwrap()
            .take(7 * 1024 * 1024)
            .read_to_end(&mut b)
            .unwrap();
        samples.push(b);
    }
    let start = Instant::now();
    let engine = Engine::builtin().unwrap();
    let cold = start.elapsed().as_secs_f64();
    for b in &samples {
        let _ = engine.classify(b).unwrap();
    }
    let mut durations = Vec::new();
    let mut checksum = 0u64;
    for _ in 0..10 {
        let start = Instant::now();
        for _ in 0..3 {
            for b in &samples {
                let result = engine.classify(b).unwrap();
                for c in result.output {
                    checksum = checksum.wrapping_add(c as u64);
                }
            }
        }
        durations.push(start.elapsed().as_secs_f64());
    }
    let peak = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|x| x.starts_with("VmHWM:"))
                .and_then(|s| s.split_whitespace().nth(1))
                .and_then(|s| s.parse::<usize>().ok())
        });
    println!(
        "{}",
        serde_json::json!({"engine":"rust","cold_load_seconds":cold,"warm_pass_seconds":durations,"calls_per_pass":samples.len()*3,"input_bytes":samples.iter().map(Vec::len).sum::<usize>(),"checksum":checksum,"peak_rss_kib":peak})
    );
}
