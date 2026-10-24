# file/libmagic: architecture and observable behavior

Baseline: **41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59**, upstream master, authored/committed 2026-10-22 19:11:42 UTC, version 5.48. All upstream references below are relative to `upstream/` at this exact commit. For an immutable browser reference use `https://github.com/file/file/blob/41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59/<path>#L<line>`.

Evidence labels: **D** documented contract; **V** verified by reading implementation; **O** observed in an executed experiment; **I** inference; **Q** unresolved. Documentation describes the target; it does not establish that the Rust implementation already satisfies it. See the separate implementation status and compatibility matrix.

## Baseline and inspection record

**O:** cloned the complete repository, read root `COPYING`, `README.md`, `README.DEVELOPER`, `INSTALL`, maintenance/release metadata, Autotools configuration, public headers, manuals, CLI, loader/compiler, evaluator, text/encoding, filesystem, compression, structured detectors, test driver, fuzz harness and sandbox implementation. Inspected representative database fragments (archive, compress, elf, commands, images, database, DER and related fixtures); scanned and hashed every tracked file and all 360 magic fragments. `reports/upstream-inventory.json` records 709 tracked files, lengths, line counts and SHA-256. This is a reproducible inventory, not a claim that every database description or platform fallback was manually audited.

**O:** `autoreconf -fiv`, `./configure --prefix=/home/john/Code/rfile/.reference`, `make -j4`, `make check` all succeeded on x86_64 Linux, glibc 2.44. The suite exercised 103 `.testfile` fixtures through the C library. Logs: `autoreconf.log`, `configure.log`, `upstream-build.log`, `upstream-check.log`. Built ELF support, zlib, bz2, liblzma, zstd, lz4, seccomp, Landlock; lzlib/lrzip not available. `reports/baseline.json` records features and database identity. Native database: 10,876,464 bytes; SHA-256 `8eab57da91fd9cc4c7af3b8a4369a64d32124aa55dc376207a81e69abd4a5ac8`; 17,627 ordinary and 7,549 named-set records (25,176 total). `reports/upstream-probes.json` contains raw CLI observations and buffer API observations.

**Not executed:** Windows, BSD, macOS, Amiga/OS2 builds, big-endian hardware, alternate libc regex implementations, external OSS-Fuzz service, long-duration fuzzing, external `file-tests` corpus, historical CVE reproductions other than fixtures present here. Their behavior remains source-derived or pending. No benchmark or compatibility percentage should be inferred from upstream's passing tests alone.

## Execution and ownership

```text
file.c main -> getopt_long -> flags/actions/parameters
  -> magic_open -> file_ms_alloc
  -> magic_load -> file_apprentice -> compiled map OR parse source
  -> process -> magic_file -> file_or_fd
     -> file_reset -> file_fsmagic -> open/fstat/read bounded prefix
     -> file_buffer -> encoding probe + detector pipeline
     -> file_getbuffer -> escaping -> filename/output framing -> flush/status

magic_buffer -> file_reset -> file_buffer (no fd/stat)
magic_descriptor -> file_or_fd (borrow fd; restore seek position if possible)
```

**V:** `src/file.c:197` (`main`), `:544` (`unwrap`), `:655` (`process`); `src/magic.c:448` (`file_or_fd`), `:567` (`magic_buffer`); `src/funcs.c:346` (`file_buffer`). Library output and errors live in the mutable `magic_set`, not a returned independent allocation. A result pointer expires on a later call or close. `magic_close(NULL)` is harmless (`magic.c:315`). `magic_buffer` does not read beyond the supplied slice; file/descriptor calls can seek separately for EOF magic and ELF/CDF details. A seekable borrowed descriptor's original position is restored, but pipes are consumed (`magic.c:448–563`). File paths and in-memory buffers need not classify identically: metadata and random access are additional evidence.

**V/I:** mutable output, search cursor, continuation stack, counters and lazily compiled regex caches make simultaneous use of one `magic_t` unsafe without external synchronization (`file.h:492`, `softmagic.c:2145`). Independent cookies are the intended concurrency unit. `magic_getpath` uses static mutable storage and legacy locale fallback can change process locale (`magic.c:184`, `funcs.c:746`); no blanket process-wide thread-safety guarantee follows from separate cookies.

## CLI contract

**D/V:** options originate in `src/file_opts.h:19`; some accepted options (`-E`) appear only in `file.c`. `main` initializes `LC_CTYPE` from the environment. `POSIXLY_CORRECT` enables symlink following by default; `-L`/`-h` override in order. Missing operands produce usage on stderr/status 1, rather than implicitly reading stdin. `-` means stdin and prints `/dev/stdin` as its label. `-f -` reads newline-delimited filenames from stdin, which is a different operation.

| Control | Observable contract |
|---|---|
| `-b`, `--brief` | Omit filename prefix. Undocumented `-bb` is brief only with <=1 operand; `-bbb` is always brief (`file.c:466`). |
| `-m LIST` | Colon-separated database paths on Unix; preserve list order. Explicit path overrides `MAGIC`. |
| `-i`, `--mime` | MIME type plus `; charset=...`. Type and encoding switches independently select the two halves. |
| `--apple`, `--extension` | Annotations instead of description; fallbacks `UNKNUNKN`, `???`. Metadata can produce empty annotation output. |
| `-k` | Continue after matches. Human descriptions use newline plus `- ` internally, then escaping normally turns newline into `\012`; raw mode leaves it literal. Builtins and text fallback also participate. |
| `-e TEST`, `--exclude-quiet TEST` | Set skip flags. Unknown test fails unless quiet. `ascii`/`text` synonyms; obsolete `tokens` accepted but ignored. |
| `-z`, `-Z` | Inspect compressed contents; `-Z` hides outer compression. `-z` human output encloses outer description in parentheses. |
| `-s` | Allow reading devices; named FIFO behavior differs from ordinary metadata-only handling. |
| `-p` | Restore atime/mtime after path inspection where available; can alter ctime and restoration errors are ignored. |
| `-F SEP`, `-N` | Replace colon; suppress padding to maximum displayed filename width. Space after separator remains. |
| `-0`, `-00` | First adds NUL after filename, retains separator/newline. Second removes separator/padding and NUL-terminates result too. Brief suppresses filename/NUL prefix. |
| `-n` | Flush each record; file lists are streamed without global-width collection. |
| `-r` | Preserve nonprintable output bytes; otherwise octal escapes, respecting locale display rules. |
| `-P name=value` | Set limit. Ten names in `file.c:141`. C accepts prefixes through `strncmp` and uses `atoi`; this is a compatibility quirk. |
| `-c`, `-C`, `-l` | Check/print source parse, compile, list rule strengths. Last selected action wins; `-f` and actions interact with parse order. |
| `-d`, `-S`, `-v`, `--help` | Debug+check; disable sandbox; print version/path/features; help. |

**V:** `-f` acts while arguments are being parsed: the first use loads a cookie using flags/database selected so far; later flags do not recreate that cookie (`file.c:280`). Preserve this in compatibility mode or label a deviation. File lists trim trailing line whitespace via `file_trimline` (`funcs.c`, `unwrap`); non-NUL line lists cannot represent arbitrary filenames. Filename alignment uses display width, including escaping, not byte length (`file.c:695`).

**V/O:** per-file failure normally prints `cannot open ...` on stdout and can return status 0; `-E` turns filesystem errors into `ERROR: ...` and status 1. Load/usage failures go to stderr/status 1; a library NULL result or failed output write makes final status nonzero (`file.c:655`). Type uncertainty is successful classification (`data`), never an error exit. Exact errno text is platform/libc dependent.

## Database discovery and APIs

**V:** `magic_getpath` (`magic.c:250`) returns explicit path, then `MAGIC` if set (including empty), then default. Unix load discovery prepends `$HOME/.magic.mgc`, `$HOME/.magic`, or `$HOME/.magic/magic.mgc` if appropriate, followed by compiled-in system path. Check/compile/list skip user home defaults. Windows separately searches application/profile/common directories and executable/DLL-relative files (`magic.c:91–247`). Loading tries `.mgc` naming and may fall back to source (`apprentice.c:463`, `:3332`, `:3535`); source directories ignore hidden/nonregular entries and sort filenames.

Public API (`src/magic.h.in:157–210`): open/close; getpath; file/descriptor/buffer; error/errno; flags get/set; version; load/load_buffers; compile/check/list; parameter get/set/getmax. `magic_open` does not load a database. `magic_load` replaces loaded lists. Error state resets on next classification. `load_buffers` accepts compiled buffers (ownership and byte swapping need care; see `apprentice.c:683`, `:3309`). The Rust API must copy/validate borrowed external database bytes rather than expose C's mapped ownership model.

Defaults (`file.h:542–559`): input 7 MiB, encoding 64 KiB, indirect 50, named calls 150, regex 8192, ELF notes 256, program headers 2048, sections 32768, section size 128 MiB, magic warnings 64. **V:** `magic_getmaxparam` reports 65535 for ELF section size despite its 128 MiB default, whereas warning maximum is reported as signed-32-bit but stored as 16-bit (`magic.c:609`, `file.h:530`). Treat these as observed quirks, not desirable Rust type design.

## Classification precedence

**V:** `funcs.c:346–556` is authoritative, rather than the simplified filesystem/magic/text manual summary:

1. Path metadata in `file_fsmagic` (directories, links, devices, sockets, FIFO, regular empty files); else read.
2. Buffer length zero -> `empty`; length one -> `very short file (no magic)` **before** encoding/magic. MIME `application/x-empty` or `application/octet-stream`, encoding `binary`.
3. Preliminary encoding probe unless excluded. It determines `looks_text` and eventual MIME charset; it is not normally a terminal match.
4. OS2 application type where compiled; compression if enabled and not excluded.
5. tar checksum, JSON/NDJSON, CSV, SIMH, CDF, in that order.
6. ELF details collected separately only for fd-backed input, to append after a softmagic match.
7. Binary softmagic pass (`BINTEST`), then text/encoding pass (`ascmagic`) which converts characters to UTF-8 and runs text softmagic (`TEXTTEST`).
8. Fallback `data`, MIME octet-stream, Apple/extension defaults. Append charset once, unless compression stage has already formatted it.

`checkdone` and separators govern `-k`: a successful builtin normally terminates; keep-going records it and tries later stages. Annotation-only matching may keep searching until a printable annotation is found. A matching top-level condition with no description can still control children, and does not automatically create a useful terminal classification. MIME annotation encountered in a successful child can terminate before later human-detail children. Flags are independently bitwise; unsupported combinations must be probed, not guessed.

## Subsystem inventory

| Responsibility, algorithm/contract | Source anchors | Dependencies | Existing verification |
|---|---|---|---|
| CLI order, framing, exit aggregation | `file.c:197`, `:544`, `:655`; `file_opts.h:19` | getopt, locale, stdio | no dedicated CLI suite; new probes required |
| Cookie/API, borrowed descriptor handling | `magic.c:250`, `:315`, `:448`, `:567` | POSIX/Win32 IO | `tests/test.c:85`; Python bindings are consumers |
| Prefix + lazy bounded tail access | `buffer.c:46`, `:77` | fstat, pread | EOF rules in database; sparse/pipe tests needed |
| Source parse, grouping, strength sorting | `apprentice.c:1331`, `:1530`, `:2050`, `:2481`, `:3023` | libc regex syntax validation | `multiple*.magic`, `regex-eol.magic`, `searchbug.magic` |
| Binary database validation/swap/compile | `file.h:200`, `:245`; `apprentice.c:3408`, `:3478`; `swap.c` | optional mmap | compiler used by every build; adversarial loader tests needed |
| Tree evaluator, indirection, named recursion | `softmagic.c:217`, `:863`, `:1569`, `:1649`, `:2167` | checked byte access, regex, dates | fixture suite plus magic corpus; `CVE-2014-1943` |
| Formatting, annotation and escaping | `softmagic.c:602`, `:2560`; `funcs.c:563`; `print.c:310` | printf/fmtcheck, locale/timezone | all `.result`; custom format boundary tests needed |
| Metadata, symlinks, unreadable/special files | `fsmagic.c:177`; `magic.c:275` | stat/lstat/readlink/access | absent dedicated metadata suite |
| Encoding and text description | `encoding.c:75`, `:246`, `:356`; `ascmagic.c:71`, `:105` | UTF state machine, EBCDIC tables | ascii escape, utf16xmlsvg, language fixtures |
| tar | `is_tar.c:74`, `:112` | 512-byte checksum incl signed/unsigned variants | magic archive rules; generated tar tests needed |
| JSON/NDJSON | `is_json.c:127`, `:346`, `:422` | bounded recursive recognizer | json1–8, jsonlines1 |
| CSV | `is_csv.c:76`, `:135` | quote skipping and equal comma counts on lines | no dedicated fixture; generated quoted CSV tests |
| SIMH | `is_simh.c:112` | little-endian framed records and marks | needs generated tapes |
| ELF metadata/core notes | `readelf.c:1895`, `:1428`, `elfclass.h` | seekable fd, ELF header layouts | malicious regression; OS binaries in differential corpus |
| Compound documents | `cdf.c:395`, `:472`, `:564`, `:1009`; `readcdf.c:562`, `cdf_time.c` | sector/SAT/mini-SAT chains, summary properties | HWP2016.hwp; issue311/359 are ZIP rule tests |
| DER | `der.c:149`, `:182`, `:264`, `:303` | ASN.1 TLV/tag/length | `magic/Magdir/der`; new truncated-length corpus |
| Decompression | `compress.c:201`, `:305`, `:605`, `:1189` | native libs and external helpers depending feature/sandbox | compressed fixtures; bombs/short headers needed |
| Platform isolation | `seccomp.c:78`, `landlock.c:97`, `configure.ac:10–84` | libseccomp, kernel Landlock, process creation | build-time detection; no sandbox tests in regression loop |
| Fuzz | `fuzz/magic_fuzzer.c:44`, `fuzz/build.sh:24`, `project.yaml` | libFuzzer/OSS-Fuzz, ASan/MSan/UBSan | one initialized cookie, nonempty `magic_buffer` input |

## Structured input and threat model

**V:** encoding order is ASCII (including UTF7 signature), UTF8 BOM, UTF8, UTF32 BOM, UTF16 BOM, ISO8859, extended ASCII, EBCDIC. The byte table admits BEL, BS, HT, LF, VT, FF, CR, SO, SI, SUB, ESC, NEL; generic Unicode validity alone is insufficient (`encoding.c:246`). Text descriptions add long-line length (>300), CRLF/CR/LF/NEL mix, no terminators, escapes and overstriking (`ascmagic.c:105–337`). Recognizers operate on bounded prefixes; valid-looking prefixes do not imply full validation.

**V:** tar computes checksum with checksum field treated as spaces and recognizes V7/POSIX/GNU. CSV is deliberately heuristic: stable comma count outside quotes, >=2 terminated lines, ignores unfinished last line, stops after ten lines. JSON rejects standalone scalar-only input as JSON classification despite valid JSON syntax. CDF traverses attacker-selected sector chains and properties; cycle/size/overflow protection is fundamental. ELF notes and section counts/lengths have separate limits; file buffers lack ELF enrichment. Decompression clears `MAGIC_COMPRESS` during recursive classification, hence one nested decompression layer per call (`compress.c:371`). Helpers may be forked when native support is absent and sandbox allows it; no helper should be introduced implicitly into Rust.

**V:** CLI sandbox is optional and does not wrap library calls. Linux CLI initializes Landlock then seccomp; seccomp is disabled with nonempty `LD_PRELOAD`, and sandbox implies `MAGIC_NO_COMPRESS_FORK` (`file.c:380–414`). Landlock build modes skip restrictions, input pipe allows temporary writes under `$TMPDIR` or `/tmp`, and unsupported ABI is tolerated (`landlock.c:97`). At this pin `-S` is conditionally compiled around libseccomp assignment, so Landlock-only builds deserve a specific test. These controls reduce effects of a bug; they do not substitute for bounded safe parsing.
