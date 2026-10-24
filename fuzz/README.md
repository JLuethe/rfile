# Fuzzing

Seven buildable libFuzzer targets cover compiled databases, source, classification, combined rules/data, formatting, encodings and decompression. Production detection remains independent of the fuzz engine and any C library.

```sh
cargo check --manifest-path fuzz/Cargo.toml --locked --bins
cargo install cargo-fuzz
cargo +nightly fuzz run classify -- -max_len=65536
cargo +nightly fuzz run compiled_database -- -max_len=1048576
cargo +nightly fuzz run magic_source -- -max_len=65536
cargo +nightly fuzz run rules_and_data -- -max_len=65536
cargo +nightly fuzz run formatter -- -max_len=63
cargo +nightly fuzz run encoding -- -max_len=65536
cargo +nightly fuzz run compression -- -max_len=65536
```

Run from the repository root. Seed content targets with public upstream fixtures and the database target with small valid compiled custom rules. Keep minimized failures and record seed, toolchain, limits and feature manifest. No 24-hour fuzz campaign has been completed. Stable deterministic mutation smoke testing is separate:

```sh
cargo test --release --locked --test hardening -- --ignored --nocapture
```
