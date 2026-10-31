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


def load_manifest(path):
    manifest = json.loads(path.read_text())
    if not isinstance(manifest, dict):
        raise ValueError('Manifest must be a JSON object')
    for key in ('version', 'source_commit'):
        if not isinstance(manifest.get(key), str) or not manifest[key]:
            raise ValueError(f'Manifest requires a nonempty {key}')
    files = manifest.get('files')
    if not isinstance(files, dict) or not files:
        raise ValueError('Manifest files must be a nonempty object')
    for name, info in files.items():
        if not isinstance(name, str) or not isinstance(info, dict):
            raise ValueError('Manifest file entries must map paths to objects')
        digest = info.get('sha256')
        if not isinstance(digest, str) or len(digest) != 64 or any(
                c not in '0123456789abcdefABCDEF' for c in digest):
            raise ValueError(f'Invalid manifest checksum: {name}')
        mode = info.get('mode')
        if type(mode) is not int or not 0 <= mode <= 0o777:
            raise ValueError(f'Invalid manifest permissions: {name}')
    return manifest


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
        manifest = load_manifest(root / 'MANIFEST.json')
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
        manifest = load_manifest(receipt)
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
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        raise SystemExit(f'rfile installer: {error}') from None
