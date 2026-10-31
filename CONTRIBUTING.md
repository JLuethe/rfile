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
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
python3 tools/rebuild_database.py
```

CI checks default and all-features builds plus the declared Rust 1.87 minimum. The initial local run used Rust 1.99.1; other compiler
versions must be checked rather than assumed compatible. For full local oracle
checks, prepare the pinned C build using `bash tools/build_reference.sh`, then run
`python3 tools/verify.py --benchmark` (default) and
`python3 tools/verify.py --all-features --benchmark` (extended codecs). System corpus contents vary by host; retain
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

## Independent release verification

Claude owns test execution and retesting; Codex implements fixes and performs
other validation. See [release scope and PR decisions](docs/release-scope.md)
for the current contract. Use separate checkouts and serialize feature builds.

Verification saves per-run reports/logs under `reports/verification-runs/` and
updates a latest-run summary at startup and on success, failure or interruption.
Full oracle runs require mutation fixtures from `RFILE_REFERENCE/tests`;
`--no-oracle` explicitly permits a recorded missing-fixture skip. A direct ignored
mutation run requires fixtures unless `RFILE_REQUIRE_FIXTURES=0` is set.
An explicit `RFILE_REFERENCE` must always be valid.

Use `python3 tools/verify.py --all-features --require-sandbox` on a capable host
to require actual CLI and kernel sandbox coverage in debug and release. Without
that flag, unsupported capability checks are recorded as skipped, not certified.
Archive the run directory before treating an ignored local log as release evidence.
