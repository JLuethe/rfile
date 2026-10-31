# Independent review evidence

Source snapshot: `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76`, exported by Codex
using `git archive` into an isolated temporary directory. Claude received that
identity; the archive itself has no Git metadata. Production source was not
changed by either agent during this planning review.

`claude-static.md` is Claude's verbatim initial review. Its initial shell calls
were denied by the command allowlist; it ran no tests. This was a harness
configuration limitation, not a Rfile failure. A second test-only pass was
requested with explicit allowed commands. Do not count the initial report as
fresh build or test evidence.

`claude-tests.md` is Claude's verbatim second report: 143 default tests and 145
all-feature tests passed, with one ignored mutation test in each run. Formatting,
all-feature Clippy, a default release build and two release-binary smoke commands
also passed. `claude-tests-metadata.json` records model/run identity and the empty
permission-denial list. These were executed by Claude, not Codex.

Normal Rust test capture can hide a successful test's `SKIP` message. The second
report's passing sandbox tests do not independently establish complete kernel
enforcement coverage; the release lane must record explicit enforcement and
skip evidence. No fresh MSRV run, oracle matrix, artifact install or sustained
security campaign was performed in this bounded pass.

Codex checked the static findings against repository and live GitHub evidence:

- Sandbox tests can skip unsupported kernels without failing the suite. The
  mutation test is ignored by normal CI and silently omits missing fixtures;
  its fixture path also does not honor `RFILE_REFERENCE`. These are actionable
  coverage-reporting gaps, included in the release plan.
- CI **does** run C/C++ clients through `tools/abi_clients.py --rust-only`,
  including the C client with ASan/UBSan. What is missing there is the native
  comparison and Rust sanitizer qualification, not all C/C++ client execution.
- A public experimental release exists. README's reference to it and HANDOFF's
  statement that no **new** release occurred are not contradictory. The active
  branch named in HANDOFF also matches the reviewed branch. Other status text
  is stale; concrete examples are identified in the release plan.
- The ordinary FFI build defaults to a development source path. The existing
  package builder sets `RFILE_MAGIC_PATH` to the intended installation prefix.
  Retest that artifact's discovery contract; this is not evidence that every
  packaged library has a broken path. Arbitrary relocation is not an established
  promise of that package.
- A warm-cache offline build does not prove vendored-source completeness.
  Artifact validation must separately test a source archive in a clean setup.

The [release plan](../../docs/11-github-release-plan.md) contains the reviewed
priorities and acceptance criteria. Neither this bounded review nor ordinary
test success constitutes an independent security audit.
