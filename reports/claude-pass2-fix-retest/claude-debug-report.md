# Focused retest of 2ef0bbc (evaluator DEBUG stderr best-effort)
Not exact-release-artifact acceptance. F07 inherited-closed-stdout criterion NOT closed (still documented/unresolved).
- Commit 2ef0bbcba7c6099a0f539b7a282388493d051f90, tree b1fa7771ca68816f1ed16d6da0ad6a13cf2e2a29 (checkout clean before/after)
- Toolchain rustc 1.99.1; build: `cargo build --release --locked -j1` (40s), CARGO_TARGET_DIR/TMPDIR on /home/john disk
- rfile sha256 0c55d587ea467c71e1ab360d99576e83c07989e8aff000ea43e4a911dc2d820b
- Diff vs d7d0eca: src/eval.rs (eprintln! -> `let _ = writeln!(stderr().lock(), ...)`, import io::Write) and tests/cli_output.rs (+regression). Nothing else.
- Repro (`rfile -d -b -m m.magic s.bin`, TZ=UTC0; log 2ef-repro.log): writable stderr -> 2 debug lines, stdout "matched second"; /dev/full, read-only fd2, closed fd2, early-closed pipe -> rc=0, no panic, stdout byte-identical to baseline. Also non -b form prints "s.bin: matched second" rc=0 under /dev/full.
- Tests (2ef-tests.log): cli_output 4/4 (incl. new debug_diagnostics_do_not_panic_on_unwritable_stderr) and semantics 27/27 (incl. compiled_roundtrip_and_builtin, source_* tests), in both default and --all-features.
- Skipped: full suite, d7 baseline build to show the old panic (other Claude owns d7 verification), sandbox feature runs, release-artifact checks.
- Verdict: PASS. Fix behaves as intended; no unintended diff.
