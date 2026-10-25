// SPDX-License-Identifier: GPL-3.0-only
//! Research driver for the bounded source-check primitive, not the C ABI.
use std::io::{Read, Write};
fn main() {
    let mut input = Vec::new();
    std::io::stdin()
        .take(128 * 1024 * 1024 + 1)
        .read_to_end(&mut input)
        .unwrap();
    let name = std::env::args().nth(1).unwrap_or_else(|| "magic".into());
    let report = rfile::Database::check_source_dump(
        &input,
        &name,
        &rfile::TimeContext::from_environment().unwrap(),
        16 * 1024 * 1024,
    );
    std::io::stderr().write_all(&report.output).unwrap();
    if let Err(e) = report.database {
        eprintln!("rfile-source-check: {e}");
        std::process::exit(1);
    }
}
