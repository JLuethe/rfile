# Current compatibility profile and deliberate limits

This is the supported **experimental** behavior contract for the Rust engine,
CLI and optional C interface on the executed x86_64 Linux/glibc profile. The
reference is file 5.48 at `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`.
The published v0.2.0-rc.1 artifacts retain their own [release scope](release-scope.md);
later `main` changes do not change those bytes. “Exact” below means the named
comparisons only, with the same rules, locale, timezone, options and input.
An allowed call outside the compared profile may still return a result; that
result is **not** a claim of native byte, channel or state parity.

| Area | Current supported profile | Outside the parity claim | Why and next gate |
|---|---|---|---|
| Regex captures | GNU C-byte ERE subset, single-digit backreferences to closed groups, at most 32 groups, bounded VM histories and earliest-start/longest-end selection. Source and v21 routes share the same dialect. | In `Pinned548`, referenced captures under variable finite repetition with upper bound above one, or variable-width/reference-containing captures under positive-minimum unbounded repetition, return `Unsupported` at evaluation. `CorrectedV1` enumerates those histories but does not claim glibc's pruning quirks. | The host's quantified capture behavior is observably inconsistent with complete history enumeration. A bounded exact glibc emulation needs a separate differential contract; no silent nonmatch is permitted. See [SEM](remaining/04-semantics.md#r6a-delivered-regex-contract-and-remaining-fidelity-boundary) and `tests/regex_profiles.rs`. |
| Magic source grammar and collation | Checked source and v21 parsing, the byte-identical selected rule rebuild, GNU C-regex classes and single-byte collating/equivalence elements. Strict Rust source constructors fail on syntax/capability errors. | Multibyte collating elements are rejected; arbitrary libc collation and source forms that the native diagnostic parser accepts with warnings are outside the strict-source parity profile. Source-action partial recovery is covered only for its recorded cases. | Native grammar/warning acceptance can produce different records and state. Expanding it requires case-by-case parser, compiled roundtrip and diagnostic tests, never an unclassified accepted rule. See [source-action contract](remaining/07-c-abi.md#r8b-source-diagnostic-contract-and-remaining-actions). |
| Time and locale | Rust defaults to immutable UTC and C-byte display. The CLI snapshots its selected locale/TZ before inspection. Executed locale parity is C, C.UTF-8 and en_US.UTF-8 on Linux/glibc; the pinned date matrix covers its named TZif/POSIX zones and boundary values. Explicit POSIX rules and TZif data are checked and bounded. | Arbitrary installed locales, codesets other than ASCII/UTF-8, custom/extreme TZif footers, leap-second tables, host rounding-mode changes and native undefined FILETIME results have no general byte-parity claim. Unknown locale/unsupported codeset and invalid timezone settings return configuration errors; other valid untested zones may still render without a parity guarantee. | Locale tables and libc edge behavior are host-specific. The post-release 2,850-case date rerun found 30 native undefined FILETIME cases and required rfile's safe invalid marker; its earlier historical report counted raw byte matches in those cases. Add identical-zone-byte and host-specific vectors before widening the exact profile. See [SEM](remaining/04-semantics.md#sem-03-and-sem-04-dates-timezone-loading-and-calendar-behavior). |
| C ABI diagnostics, I/O and state | Experimental `librfile_magic` with 19 exports, valid caller-owned pointers, externally serialized cookies, explicit bounded regular-file or buffer database inputs, and the recorded classification/action sequences. The original 49 state cases and 440 expanded diagnostic cases were exact on their recorded candidate; CLI database actions had 243 exact explicit-path cases in each sandbox mode. | Default database discovery without an explicit installation-specific path, DEBUG tracing, untested accepted source warnings/conditional grammar, locale-specific I/O prose, special-file/failure races, and I/O failure after an earlier successful directory member are outside native parity. Source-tree defaults are development behavior. All valid C calls remain subject to documented pointer/ownership and finite safety limits. | A matching symbol and `magic_version()==548` do not establish drop-in behavior. Broader action/state equivalence requires named matrices and same-candidate isolated package, sanitizer and independent review before a conventional `libmagic` identity is offered. See [C ABI contract](remaining/07-c-abi.md) and [FFI README](../crates/rfile-ffi/README.md). |

## Decision rules

- `Pinned548` retains safe observed quirks within the named profile. It rejects
  the quantified-reference class above visibly; a generic `data` result cannot
  stand in for a recognized but unimplemented match. `CorrectedV1` is an
  explicitly different semantic profile, not evidence of pinned exactness.
- A parser or decoder error, budget limit, cancellation or unavailable feature
  is reported as such. None is treated as a successful native classification.
  Finite resource limits and memory-safe rejection of malformed native behavior
  are intentional differences.
- The C interface is a preview under its distinct library name. It is suitable
  for clients that accept the documented subset and limits; no supported-profile
  decision authorizes system `libmagic` replacement or a full ABI conformance
  claim.
- Remaining fidelity is qualified only by new direct comparisons on the final
  candidate. Passing earlier matrices, source safety, or an unsupported label
  does not close the sustained fuzzing or independent security gates in
  [issue #3](https://github.com/JLuethe/rfile/issues/3).
