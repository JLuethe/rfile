// SPDX-License-Identifier: GPL-3.0-only
fuzz_target!(|data: &[u8]| {
    if data.len() > 262144 { return; }
    let limits = rfile::Limits { bytes: 262144, codec_window: 8 * 1024 * 1024,
        memory: 64 * 1024 * 1024, steps: 100000, work_bytes: 8 * 1024 * 1024,
        codec_members: 16, ..rfile::Limits::default() };
    for profile in [rfile::SemanticProfile::Pinned548,rfile::SemanticProfile::CorrectedV1] {
        let budget = rfile::InspectionBudget::new(&limits,rfile::ExecutionControl::default()).unwrap();
        let result=rfile::codecs::decode(data,CODEC,&budget,profile);
        if let Ok(decoded)=&result { assert!(decoded.bytes.len()<=limits.bytes); assert!(decoded.facts.input_used<=data.len()); }
        drop(result);
        assert_eq!(budget.metrics().tracked_memory_live,0);
    }
});
