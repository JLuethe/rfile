# Claude independent retest of ec687fc03817ca5716f40a06a9dece0db9d68098 (tree f703f5f144385a511b08305ba50feda083a781e0)
Partial retest, stopped early because a real F01 product mismatch remains. Not a complete audit, not a final-CI-artifact claim.

## Identities
- Archive: rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz sha256 8341a0a8eae6bd3eeb90fa420cc80913b43986c602da5082c1c002c573a0f2c0 (source tar 4a597695bca110410d3bf5937083ae677b85ce658414a18227cbab655c2174bd); SHA256SUMS verified OK; manifest source_commit = ec687fc…
- Packaged binary sha256 7fcd24730573dda6b1b19b76b670ba83249a5cb6e2ca4a76dd9c2bfda366bcb4e (binary.sha256)
- Separate release build from git-archive of the same tree (used only for elf_compare): rfile 925b7eeb…, rfile-probe a0c9ebf9… (src-build.sha256)
- Oracle: .reference/file-clean 41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59; magic data/magic.mgc
- Checkout stayed clean (git status empty).

## Verdicts
- F09: PASS (this scope). 8 of 9 prior failures pass in the original harness; the 9th ("uninstall receipt-files-list") now refuses rc1 with a concise message and unchanged state; my old harness expectation (rc0) was stale/wrong, not a product defect. Added 26 install + 12 receipt malformed variants: all rc1, no traceback, no prefix residue, preexisting prefix/precious data unchanged. Cosmetic: some refusals lack the "rfile installer:" prefix (archive payload checksum mismatch) and raw JSON/UTF-8 decoder text is shown.
- F01: NOT CLOSED. Core: 0 diffs across the core subset (60 files x 14 runs = 840 runs: LE/BE, 10 offsets, 3 sizes, -b/-i/--mime-type/-E/-k, path+stdin). elf_compare.py 5461/5461 exact. No rfile ERROR or nonzero exit in 5880 runs (e64 baseline had 564). BUT exec/dyn (ET_EXEC/ET_DYN with PT_NOTE/PT_DYNAMIC/PT_INTERP) still differ in 1800/3840 runs:
  1. 1440: src/elf.rs:924 emits " can't read section at N" without leading comma; oracle (readelf.c:1813) emits ", can't read section at N". Trivial fix.
  2. 360: p_offset 2^63-1 or 2^63-2 with filesz making off+len overflow signed off_t: oracle reports "can't read section at N" (pread fails); rfile prints a normal description (dynamically/statically linked, interpreter *empty*). 
- F02: PASS as documented-difference (no sampling equivalence claimed). gzip/zlib cap-1 and exact succeed; cap+1 with -z/-Z => rc1 "ERROR: decompression output limit exceeded", no partial/wrapper text; -P bytes=16777216 succeeds; 1 GiB bombs still error with raised cap; CRC/ISIZE/adler/truncation all rc1; oracle -S differs (samples), as documented. Docs mention 7 MiB in manual, compatibility, release notes.

## Untested (not run because product failure found)
cargo test (both profiles), tools/test_installer.py, Clippy, workspace tests; negative-position IO not directly observed (no strace/perf available) - inferred only from absence of IO errors; BE-specific per-class breakdown only via aggregate counts.
