# rfile baseline test report: snapshot 7a3ea75

All six gates passed on this snapshot, and both smoke runs of the release binary worked.

## Identity
- **Executor model:** Claude Opus 5.5 (`claude-opus-5-5`)
- **Source:** `/tmp/rfile-review-7a3ea75/source`, a git archive snapshot. The orchestrator says it was made from `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76`. I did not run Git, so I have not checked this.
- **Toolchain observed:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)`
- **Date:** 2026-10-30

## Results (all observed in this session)

The Bash tool does not print exit codes directly; it reports an error when a command exits non-zero. No command reported one, so I'm recording each as exit 0.

| # | Command | Exit | Result |
|---|---|---|---|
| 1 | `/home/john/.cargo/bin/rustc --version` | 0 | `rustc 1.99.1` |
| 2 | `/home/john/.cargo/bin/cargo fmt --all -- --check` | 0 | No formatting differences |
| 3 | `timeout 150s /home/john/.cargo/bin/cargo test --workspace --locked --offline` | 0 | **143 passed, 0 failed, 1 ignored** (built in 7.80s) |
| 4 | `timeout 150s /home/john/.cargo/bin/cargo test --workspace --all-features --locked --offline` | 0 | **145 passed, 0 failed, 1 ignored** (built in 10.89s) |
| 5 | `timeout 150s /home/john/.cargo/bin/cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings` | 0 | No warnings or errors (5.56s) |
| 6 | `timeout 120s /home/john/.cargo/bin/cargo build --release --locked --offline` | 0 | Release build finished (27.94s) |
| 7 | `timeout 10s ./target/release/rfile --help` | 0 | Usage text printed. It reports "pinned magic v21" and lists `--profile=pinned-5.48\|corrected-v1`, `--sandbox=auto\|strict\|off`, and `--capabilities` |
| 8 | `timeout 10s ./target/release/rfile -i README.md` | 0 | `README.md: text/plain; charset=us-ascii` |

### Notes on the test runs
- **Ignored test (both runs):** `hardening::deterministic_mutation_campaign`. Its reason string says to run it with `--release --ignored`. I did not run it, because the brief ruled out long fuzzing.
- **Why all-features has 2 more tests:** `crates/rfile-platform/tests/sandbox.rs` has 0 tests in the default run. With all features it runs 2 (`policy_child` and `kernel_policy_children`), and both pass.
- **Timing:** with all features, `sandbox_cli::cli_policy_modes_order_and_identity` took 14.09s, compared with 0.31s in the default run. It still passed, well inside the timeout.
- **Dependencies:** every dependency resolved offline with `--locked`. Nothing was missing and the network was never needed. Bash denied no commands.

## What this means for release
- On this snapshot and toolchain, the basic checks pass: formatting, default and all-features tests, clippy with warnings as errors, and the release build. The binary also runs and gives a sensible MIME result.
- **What this does not cover:**
  - This is a small baseline run, not a security audit or a fuzzing campaign.
  - The ignored mutation campaign is unexercised.
  - The sandbox tests passed on this one kernel (`Linux 7.2.8-9-omarchy`). Landlock and seccomp behaviour on other kernels is untested.
  - I didn't compare the output against a reference `file`/libmagic beyond what the test suite already pins.
- **Earlier attempt:** a previous static review stopped after Git commands were denied. Everything in this report comes from my own runs in this session; nothing is carried over from that attempt.

I modified no source files, installed nothing and published nothing. The only new files are build artifacts under `target/`.
