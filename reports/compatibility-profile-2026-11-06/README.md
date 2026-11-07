# Compatibility profile review evidence

Executed 2026-11-06 (America/Los_Angeles) on Linux x86_64/glibc with the
clean pinned file 5.48 oracle at
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. The Rust shared library
SHA-256 recorded in every JSON report is
`9f14daa63290ddd9331da3dbd7ece0bebf092fc9f5cf4e76163f9e497c51cf96`.
Its production source and dependencies are the reviewed `283fd29` content
integrated into `main` by #31 (`727a701`). The documentation and new regex
rejection vectors were tested in the separate compatibility-profile branch.

Commands and results, with `RFILE_REFERENCE` set to the clean oracle:

- `python3 tools/abi_compare.py`: 49/49 exact state/action cases.
- `python3 tools/abi_diagnostic_compare.py`: 440/440 exact cases.
- `python3 tools/abi_clients.py`: 6/6 C/C++ client checks passed; 20 exports.
- `cargo test --test regex_profiles --test display_locale --test time_and_regex --test source_actions --all-features --locked`: all 30 focused tests passed, including visible rejection of multibyte collating/equivalence elements.

The three JSON reports here preserve the exact current-run inputs/results and
binary identity. The older top-level ABI reports retain their own historical
library hash. These cases do not cover the excluded default-discovery, DEBUG,
special-file and racing I/O behaviors named in
[the profile contract](../../docs/compatibility-profile.md).
