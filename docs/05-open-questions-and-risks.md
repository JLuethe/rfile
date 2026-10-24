# Open questions and risks

Baseline: `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Reasonable working assumptions: Linux first; binary called `rfile`; GPL-3.0-only for current rfile contributions (initial local revisions used BSD-2-Clause); retain upstream v21 data as rules; no mandatory C ABI or external decompression processes in first release. Final user instruction supersedes the earlier “do not implement yet”: specification first, then build and verify.

| Priority | Question/risk | Evidence and resolution path |
|---|---|---|
| Critical | Full upstream replacement is much larger than signature matching | 25,176 rule records plus CDF/ELF/encoding/compression; each incomplete subsystem needs explicit scope/status. Do not call a fixture subset complete. |
| Critical | Bounded memory does not bound total CPU | source rules multiply input scanning, nested use/indirect and regex; enforce aggregate work/byte budgets across all child calls, not just recursion depth. |
| High | POSIX longest regex vs Rust regex | offsets/rendered spans depend on longest match; overall earliest/longest selection is now implemented and regression-tested. Backreferences/collation/libc syntax and platform cases still need closure. Unsupported backreferences need explicit diagnostics: the current octal-enabled backend can silently interpret a backreference-looking escape as octal. A minimal `(ab)\1`/`abab` oracle example differs and is outside the passing corpus. |
| High | Upstream regex discards search-region final byte | `softmagic.c:2387`; source-observed behavior needs minimal oracle test and versioned compatibility handling. Do not “fix” unnoticed. |
| High | Native sort ties and hidden provenance bytes | `apprentice_sort` compares complete record except line; empty desc contains debug filename. Source-compiler ordering may be architecture/path dependent; compiled-oracle comparison isolates evaluator. |
| High | Recently added types might parse but not evaluate | ID3, varints and certain indirect doubles have enum/parser entries but incomplete-looking `mconvert`/`magiccheck` switches. Treat as suspected defects, add custom magic repro before deciding parity. No unverified claim that every documented type works. |
| High | CDF sectors/summary metadata are a major safety surface | chain cycles, allocations, directory strings, FILETIME; build independent bounded readers and fuzz them before deployment. Header-only fallback isn't equivalent. |
| High | Data licensing/provenance | root grant and fragment-specific notices; retain full originals and build a notice manifest; no wholesale corpus redistribution assumed. |
| High | Limits API inconsistencies | `getmaxparam(ELF_SHSIZE_MAX)` vs default; magwarn width truncation; CLI parameter prefix+atoi. Record observations separately and decide compatibility mode behavior with tests. |
| High | Filesystem blocking/races | default special-file classification must happen before read; -s intentionally changes risk; path lstat/open races require fd verification and OS flags. |
| Medium | libc printf and local-time behavior | %g rounding, locale width/escaping, timezone beyond UTC and platform time_t range. Golden vectors and distinct portability gates. |
| Medium | `-f` option parse-order semantics | loads cookie during parsing; simple “parse everything then execute” implementation changes flags/database. Need event stream CLI if preserving exact behavior. |
| Medium | Sandbox difference | memory safety does not provide seccomp/Landlock. Kernel support and LD_PRELOAD bypass are feature-dependent; do not advertise sandbox until implemented and tested. |
| Medium | Compression output and error quirks | one layer vs unlimited recursion; native codec errors depend on library; safe Rust gzip-first profile differs from full C build. Use matched profile for performance/accuracy conclusions. |
| Medium | Rules updates introduce new syntax | pin explicit commit; scan grammar/types and run all rule compiler/evaluator regressions before promoting. Never auto-fetch mutable master in normal build. |
| Medium | Shared DB regex cache memory | hundreds of patterns, per-rule compilation and caches can exceed input-sized assumptions; cap aggregate cache or precompile with measured budget. |
| Medium | Platform certification | only Linux run here; byte order, inode descriptions, Windows path search and BSD flags require native testing. Source review is not executed coverage. |
| Low | Optional libmagic C ABI required by adopters? | independent Rust API first; ABI later only with panic containment, ownership and symbols tested. |

Decision rule: preserve safe, observed compatibility quirks; expose corrected behavior only as an explicit semantic profile. Fail safely on malformed/unsupported states instead of reproducing UB. Report the resulting deviation with reproducible evidence. No unresolved issue should be hidden by changing expected files to match the Rust output.

Resolved during implementation: exact selected-source compilation (including signed mask storage and final unterminated source line numbering); global keep-going separator state and database annotation boundaries; CDF FileHeader-based HWP detection; common Linux ELF enrichment; bounded synchronized regex caching; raw captured-string output. See the status document for evidence, remaining scope and release gates.
