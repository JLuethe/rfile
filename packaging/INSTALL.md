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

```sh
python3 install.py install --prefix "$HOME/.local"
"$HOME/.local/bin/rfile" --version
python3 install.py uninstall --prefix "$HOME/.local"
```

The installer records the files it owns. Uninstallation refuses to remove any
file whose contents changed after installation, and preserves unrelated files.
The executable itself has no Python runtime dependency.

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
