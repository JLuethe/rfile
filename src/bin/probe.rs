// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
#![forbid(unsafe_code)]
use rfile::{Database, Engine, Flags};
use std::{
    io::{self, BufRead, Read},
    path::Path,
    sync::Arc,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut engine = Engine::new(Arc::new(
        if let Some(p) = args.get(1).filter(|x| x.as_str() != "-") {
            Database::merge(
                std::env::split_paths(p)
                    .map(|p| Database::load(&p).unwrap())
                    .collect(),
            )
            .unwrap()
        } else {
            Database::builtin().unwrap()
        },
    ));
    engine.flags = Flags(
        args.get(2)
            .map_or(0, |n| u32::from_str_radix(n, 16).unwrap()),
    );
    let mode = args.get(3).map_or("path", String::as_str);
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let path = Path::new(&line);
        let result = if mode == "buffer" {
            let mut b = Vec::new();
            std::fs::File::open(path)
                .and_then(|f| f.take(engine.limits.bytes as u64).read_to_end(&mut b))
                .map_err(rfile::Error::from)
                .and_then(|_| engine.classify(&b))
        } else {
            engine.classify_path(path)
        };
        match result {
            Ok(c) => println!(
                "{}",
                serde_json::json!({"path":line,"output":String::from_utf8_lossy(&c.output),"hex":c.output.iter().map(|x|format!("{x:02x}")).collect::<String>(),"mime":c.mime_type,"encoding":c.encoding,"truncated":c.truncated})
            ),
            Err(e) => println!("{}", serde_json::json!({"path":line,"error":e.to_string()})),
        }
    }
}
