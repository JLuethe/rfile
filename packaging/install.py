#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Install or remove this rfile archive in an explicit, user-owned prefix."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def destination(prefix, name):
    relative = PurePosixPath(name)
    if relative.is_absolute() or '..' in relative.parts or not relative.parts:
        raise ValueError(f'Invalid installed path: {name}')
    path = prefix.joinpath(*relative.parts)
    for parent in [path, *path.parents]:
        if parent == prefix:
            break
        if parent.is_symlink():
            raise ValueError(f'Refusing symlink in installed path: {parent}')
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['install', 'uninstall'])
    parser.add_argument('--prefix', required=True, type=Path)
    args = parser.parse_args()
    prefix = args.prefix.expanduser()
    if not prefix.is_absolute():
        parser.error('Prefix must be absolute')
    # A deliberately chosen prefix may itself be reached through a symlink;
    # paths within the installation must not redirect writes or removals.
    prefix = prefix.resolve()
    receipt = destination(prefix, 'share/rfile/install-receipt.json')
    if args.action == 'install':
        root = Path(__file__).resolve().parent
        manifest = json.loads((root / 'MANIFEST.json').read_text())
        files = manifest['files']
        if receipt.exists():
            raise SystemExit('rfile is already installed at this prefix; uninstall it first')
        for name, info in files.items():
            path = destination(prefix, name)
            source = root / name
            if not source.is_file() or source.is_symlink() or sha256(source) != info['sha256']:
                raise SystemExit(f'Archive payload checksum mismatch: {name}')
            if path.exists():
                raise SystemExit(f'Refusing to overwrite existing file: {path}')
        created = []
        try:
            for name, info in files.items():
                path = destination(prefix, name)
                path.parent.mkdir(parents=True, exist_ok=True)
                # Exclusive creation avoids replacing a file created since preflight.
                with path.open('xb') as output, (root / name).open('rb') as source:
                    created.append(path)
                    shutil.copyfileobj(source, output)
                path.chmod(info['mode'])
            receipt.parent.mkdir(parents=True, exist_ok=True)
            with receipt.open('x') as output:
                created.append(receipt)
                json.dump(dict(source_commit=manifest['source_commit'], version=manifest['version'],
                               files=files), output, indent=2)
                output.write('\n')
        except BaseException:
            for path in reversed(created):
                path.unlink(missing_ok=True)
            raise
        print(f'Installed rfile {manifest["version"]} into {prefix}')
    else:
        if not receipt.is_file():
            raise SystemExit('No rfile installation receipt at this prefix')
        manifest = json.loads(receipt.read_text())
        paths = []
        for name, info in manifest['files'].items():
            path = destination(prefix, name)
            if not path.is_file() or sha256(path) != info['sha256']:
                raise SystemExit(f'Refusing to remove changed or missing installed file: {path}')
            paths.append(path)
        for path in paths:
            path.unlink()
        receipt.unlink()
        parents = {p for path in [*paths, receipt] for p in path.parents
                   if p != prefix and prefix in p.parents}
        for parent in sorted(parents, key=lambda p: len(p.parts), reverse=True):
            try:
                parent.rmdir()
            except OSError:
                pass  # Preserve any directory still used by other software.
        print(f'Removed rfile {manifest["version"]} from {prefix}')


if __name__ == '__main__':
    main()
