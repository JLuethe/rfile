// SPDX-License-Identifier: GPL-3.0-only
#![forbid(unsafe_code)]
use rfile::{
    ExecutionControl, InspectionBudget, Limits, SemanticProfile,
    codecs::{self, CodecId},
};
fn main() {
    let root = std::path::Path::new("tests/fixtures/compression");
    for (name, id) in [
        ("gzip", CodecId::Gzip),
        ("zlib", CodecId::Zlib),
        ("bzip2", CodecId::Bzip2),
        ("xz", CodecId::Xz),
        ("lzma", CodecId::Lzma),
        ("lzip", CodecId::Lzip),
        ("zstd", CodecId::Zstd),
        ("lz4", CodecId::Lz4),
        ("compress", CodecId::Compress),
        ("zip-deflate", CodecId::Zip),
    ] {
        for payload in ["random", "runs"] {
            let data = std::fs::read(root.join(format!("{name}-{payload}"))).unwrap();
            let expected = std::fs::read(root.join(payload)).unwrap();
            let mut timings = Vec::new();
            let mut memory = 0;
            let mut work = 0;
            for _ in 0..10 {
                let budget =
                    InspectionBudget::new(&Limits::default(), ExecutionControl::default()).unwrap();
                let start = std::time::Instant::now();
                let decoded =
                    codecs::decode(&data, id, &budget, SemanticProfile::Pinned548).unwrap();
                timings.push(start.elapsed().as_secs_f64());
                assert_eq!(&*decoded.bytes, expected);
                memory = memory.max(budget.metrics().tracked_memory_peak);
                work = work.max(budget.metrics().work_bytes);
            }
            timings.sort_by(f64::total_cmp);
            println!(
                "{}",
                serde_json::json!({"codec":id.name(),"payload":payload,"input_bytes":data.len(),"output_bytes":expected.len(),"iterations":10,"median_seconds":timings[5],"p95_seconds":timings[9],"tracked_memory_peak":memory,"work_bytes":work,"payload_equal":true})
            );
        }
    }
}
