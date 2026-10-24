# Independent compressed fixtures

Generated from repository-owned deterministic byte patterns by
`python3 tools/compression_fixtures.py`. The manifest records input/payload
hashes, encoder versions and the exact test-only ncompress commit. Python's
stdlib, xz, zstd, lz4 and ncompress encode the fixtures; no Rust product decoder
is used to generate expected bytes. Regeneration requires those tools and the
ignored `.reference/ncompress` checkout. Ordinary Rust tests use the committed
bytes and need no encoder, native codec library or C compiler.

`random` uses Python Random(548); `runs` exercises inverse-BWT RLE, dictionary
width growth, multiple blocks and expansion caps. Cases include empty/binary
payloads, concatenated members, XZ checks/filter chains/multiple blocks, LZ4
linked blocks/check modes, UNIX compress widths and first-entry ZIP64 data
descriptors. `compatibility-decisions.json` records individually reviewed
classification discrepancies; changing it is a compatibility policy review,
not an automatic test refresh. Fixture bytes are rfile contributions under
GPL-3.0-only; encoder programs retain their own independent licenses and are
not redistributed here.
