# Experimental rfile CLI

This archive contains the default-feature x86_64 Linux/glibc CLI, with embedded
rules and gzip/zlib inspection. It does not replace `file` or libmagic. Additional
codecs, the optional sandbox and C interface are available in source and are not
enabled in this binary. Read `MANIFEST.json` for the exact version, source commit,
required glibc symbol version, compiler and checksums. Other platforms are not
certified by this archive.

Verify `SHA256SUMS` against the downloaded archives before extracting them:

```sh
sha256sum --check SHA256SUMS
```

Run without installing:

```sh
./bin/rfile --version
./bin/rfile --capabilities
./bin/rfile -i example.pdf
printf 'hello\n' | ./bin/rfile -b -
```

For an installation in your own prefix, use Python 3.11 or later. Existing files
are never overwritten; uninstall the earlier installation before upgrading.
Keep this extracted directory for later uninstallation.
The installer requires POSIX descriptor-relative filesystem operations and an
owner-controlled prefix: the prefix and installed subdirectories must be owned
by the process and not writable by group or others. Other platforms fail with
an explicit error. It does not support installing into a shared writable tree.
The selected prefix path and its ancestors must not contain symlinks.

```sh
python3 install.py install --prefix "$HOME/.local"
"$HOME/.local/bin/rfile" --version
python3 install.py uninstall --prefix "$HOME/.local"
```

The installer records the files it owns. Uninstallation refuses to remove any
file whose contents changed after installation, and preserves unrelated files.
The executable itself has no Python runtime dependency.

The receipt tracks files, not directory ownership. Uninstallation also attempts
to remove empty parent directories of installed files below the prefix, including
directories that existed before installation. It keeps the prefix itself and
non-empty directories; it does not restore the original empty-directory layout.
This directory cleanup is intentional.

On a handled installation failure, rollback attempts to remove the files created
by that attempt but leaves any directories it created. Empty directories such as `bin/`
and `share/`, or a newly created prefix, may remain. This is also intentional;
check which directories were newly created before removing leftover empty ones.

The paired source archive contains the complete committed source tree, notices
and locked vendored dependencies. Rust 1.87 or later and a native linker are
required to rebuild the default CLI. From its extracted root:

```sh
cargo build --release --locked --offline -p rfile --bin rfile
./target/release/rfile --version
```

Dependency vendoring permits offline source rebuilding; it does not by itself
establish byte-identical binaries across different compilers or system linkers.
The project remains experimental. Classification does not certify that a file
is safe, and sustained security qualification remains open.
