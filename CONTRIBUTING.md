# Contributing

This is an independent Rust implementation. Production detection must not link
libmagic or delegate classification to the C file program. The pinned upstream
checkout is a test oracle only. Read docs/README.md and the implementation status
before changing compatibility behavior.

Current rfile contributions use GPL-3.0-only. By submitting a contribution you
agree to distribute it under that license. Preserve all original upstream and
third-party notices; do not relicense vendored rule fragments or edit generated
rule data without updating provenance and verification evidence.

## Development checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
python3 tools/rebuild_database.py
```

Rust stable is used by CI. The initial local run used Rust 1.99.1; other compiler
versions must be checked rather than assumed compatible. For full local oracle
checks, prepare the pinned C build using `bash tools/build_reference.sh`, then run
`python3 tools/verify.py --benchmark`. System corpus contents vary by host; retain
hash manifests and complete discrepancies. Do not copy arbitrary system files
into this repository.

Submit focused pull requests with the observable change, compatibility decision
and relevant verification. Preserve safe upstream quirks unless a documented
semantic change is intended. Add a minimal regression for a demonstrated bug;
do not alter expected output merely to conceal a mismatch. Never add unsafe
parsing or remove limits to make a test pass.

Report security issues privately using SECURITY.md. Ordinary bug reports should
include the commit, platform, flags, database identity and a small public or
synthetically generated reproducer. Additional format/platform coverage and
sustained fuzzing are welcome; see fuzz/README.md.
