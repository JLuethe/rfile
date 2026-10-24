# rfile

[![Rust checks](https://github.com/JLuethe/rfile/actions/workflows/check.yml/badge.svg)](https://github.com/JLuethe/rfile/actions/workflows/check.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

**Early development: compatibility, platform coverage, and security hardening are ongoing.**

An independent Rust implementation of `file` magic detection, with a CLI and library. Detection, magic parsing and compilation execute in Rust. The program does not link to libmagic or run the C utility. The pinned upstream rules are bundled, so ordinary builds and detection do not need the upstream checkout or a C compiler.

This is version **0.1.0**, tested on x86_64 Linux. [Implementation status and remaining gaps](docs/06-implementation-status.md) distinguish the working profile from full libmagic replacement.

```sh
cargo build --release --locked
./target/release/rfile example.pdf
./target/release/rfile -i example.pdf
printf 'hello\n' | ./target/release/rfile -b -
./target/release/rfile -m custom.magic example.bin
```

Rust 1.87 or later is required. `--help` lists CLI options. Gzip/zlib inspection uses `-z`; `-Z` omits the outer description. Experimental additional codecs are available with `cargo build --release --locked --features extended-codecs`: bzip2, XZ/LZMA/lzip, modern Zstd/LZ4, UNIX compress and the first stored/deflate ZIP entry. They remain off by default pending sustained security gates. `--capabilities` reports the actual compiled set. Unsupported codecs return an explicit inspection error; ordinary outer identification still uses the rules. See [codec contracts and gaps](docs/remaining/03-compression.md).

```rust
use rfile::{Engine, Flags};

let mut engine = Engine::builtin()?;
engine.flags = Flags(Flags::MIME);
let result = engine.classify(b"hello\n")?;
assert_eq!(result.as_bytes(), b"text/plain; charset=us-ascii");
# Ok::<(), rfile::Error>(())
```

`Engine` shares an immutable database through `Arc`; results are owned and concurrent calls use local evaluation state with synchronized, bounded regex caches. `Classification.output` is the authoritative result for the selected flags. Auxiliary fields reflect that pass; use MIME flags for authoritative MIME output. `classify_path`, `classify_reader`, and `classify_file` provide metadata, bounded streaming and borrowed-file entry points. The borrowed file's seek position is restored. ELF path/descriptor results retain checked note evidence; reader/stdin ELF metadata uses a separately capped positional spool. Reader/buffer sampling exposes `truncated`.

The detection core uses `#![forbid(unsafe_code)]`. A separate [platform adapter](crates/rfile-platform/README.md) contains the bounded libc locale boundary; its pointer/ownership invariants and unsafe inventory are documented. Limits cover input, database size/count/files, recursion, matching work, IO, tracked inspection memory, regex construction/cache, expanded data, formatting and final output. `ExecutionControl` supplies cancellation and deadlines. `TimeContext` provides immutable UTC, named TZif or POSIX timezone settings; CLI local-date rules snapshot `TZ`. `DisplayContext` defaults to C bytes for Rust callers; explicit Linux/glibc locale contexts provide host Unicode printability/width, and the CLI snapshots LC_ALL/LC_CTYPE/LANG for LC_CTYPE only. `--capabilities` reports current support; [delivery progress](docs/08-delivery-progress.md) explains accounting boundaries. This is not a completed security audit: the long fuzz campaign and remaining format/platform gates are tracked in the status document.

The [specification set](docs/README.md) was committed before Rust implementation (`fc385f1`). It includes architecture, source evidence, compatibility decisions, implementation semantics, verification milestones and open risks. The [remaining-work specification](docs/07-remaining-work-spec.md) defines the next CDF/ELF, codec, locale/date/regex, sandbox, security and C ABI milestones; the [delivery ledger](docs/08-delivery-progress.md) separates implemented milestones from remaining acceptance and security gates. CDF classification retains checked compound-document metadata, and ELF/core enrichment is merged. The bounded regex capture VM supports backreferences with an explicit pinned quantified-history restriction; `corrected-v1` selects exhaustive histories. Locale display now has C/C.UTF-8/en_US.UTF-8 evidence. See the delivery ledger for remaining date/float/native-regex-history, sandbox and C ABI work.

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
python3 tools/rebuild_database.py       # Rust-only rebuild; verify pinned hash
bash tools/build_reference.sh          # optional: pinned C oracle + upstream tests
python3 tools/verify.py --benchmark    # full local conformance/hardening reports
```

The test oracle is upstream commit `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59` (file 5.48). The Rust compiler produces byte-identical v21 database bytes from the same selected input directory. [Reports](reports/) contain fixture, differential, CLI, compiler, hardening, dependency and benchmark evidence. Tests using system files are reproducible through their manifests and scripts; those files are not redistributed.

rfile contributions are licensed under **GPL-3.0-only**. Upstream portions, rules and dependencies retain their original license terms and notices. See [licensing details](LICENSES/README.md).

Report vulnerabilities through [private security reporting](https://github.com/JLuethe/rfile/security/advisories/new); see [SECURITY.md](SECURITY.md). Contribution and verification instructions are in [CONTRIBUTING.md](CONTRIBUTING.md).

Rule data and source notices are preserved under [data](data/). `etf` remains separate because upstream excludes it from its default database build. [NOTICE](NOTICE), [LICENSE](LICENSE), [upstream license](data/UPSTREAM-COPYING), and [dependency licenses](LICENSES/dependencies/) accompany redistribution. No system utility is replaced by building this project. The GitHub release is experimental; no crates.io package is published.
