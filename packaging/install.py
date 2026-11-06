#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Install or remove this rfile archive in an explicit, user-owned prefix."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat

RECEIPT = 'share/rfile/install-receipt.json'
DIR_FLAGS = os.O_RDONLY | getattr(os, 'O_DIRECTORY', 0) | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_CLOEXEC', 0)
READ_FLAGS = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_CLOEXEC', 0) | getattr(os, 'O_NONBLOCK', 0)
CREATE_FLAGS = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_CLOEXEC', 0)


def require_platform():
    if os.name != 'posix' or any(not hasattr(os, key) for key in
            ('O_DIRECTORY', 'O_NOFOLLOW', 'O_CLOEXEC', 'O_NONBLOCK')) or any(
            operation not in os.supports_dir_fd for operation in
            (os.open, os.mkdir, os.unlink, os.rmdir, os.stat)):
        raise ValueError('Secure descriptor-relative installation is unavailable on this platform')


def parts(name):
    if not isinstance(name, str):
        raise ValueError('Invalid installed path')
    path = PurePosixPath(name)
    if path.is_absolute() or not path.parts or any(p in ('', '.', '..') for p in name.split('/')):
        raise ValueError(f'Invalid installed path: {name}')
    return path.parts


def owned_directory(fd, name):
    meta = os.fstat(fd)
    if not stat.S_ISDIR(meta.st_mode) or meta.st_uid != os.geteuid() or meta.st_mode & 0o022:
        raise ValueError(f'Refusing untrusted writable directory: {name}')


def safe_ancestor(fd, name):
    meta = os.fstat(fd)
    if not stat.S_ISDIR(meta.st_mode):
        raise ValueError(f'Refusing non-directory ancestor: {name}')
    # An owner can change permissions and rename entries even when the current
    # mode is 0755, so elevated installs cannot traverse attacker-owned parents.
    if meta.st_uid not in (0, os.geteuid()):
        raise ValueError(f'Refusing unowned prefix ancestor: {name}')
    if meta.st_mode & 0o022:
        # A sticky shared directory (such as /tmp) protects entries owned by
        # this process; a plain shared writable parent permits rename attacks.
        if not meta.st_mode & stat.S_ISVTX or meta.st_uid not in (0, os.geteuid()):
            raise ValueError(f'Refusing writable prefix ancestor: {name}')


def descend(fd, name, create=False, trusted=False):
    try:
        child = os.open(name, DIR_FLAGS, dir_fd=fd)
    except FileNotFoundError:
        if not create:
            raise
        os.mkdir(name, 0o755, dir_fd=fd)
        child = os.open(name, DIR_FLAGS, dir_fd=fd)
    except (NotADirectoryError, OSError) as error:
        if isinstance(error, NotADirectoryError) or error.errno == 40:
            raise ValueError(f'Refusing symlink or non-directory component: {name}') from error
        raise
    try:
        if trusted:
            owned_directory(child, name)
        return child
    except BaseException:
        os.close(child)
        raise


def open_prefix(prefix, create):
    # Traverse the caller's exact absolute path. Resolving symlinks first would
    # let an attacker choose an unrelated owner-controlled destination before
    # the descriptor checks begin.
    if '..' in prefix.parts:
        raise ValueError('Refusing parent traversal in prefix')
    fd = os.open('/', DIR_FLAGS)
    try:
        safe_ancestor(fd, '/')
        for name in prefix.parts[1:]:
            safe_ancestor(fd, name)
            child = descend(fd, name, create)
            os.close(fd)
            fd = child
        owned_directory(fd, str(prefix))
        return fd, prefix
    except BaseException:
        os.close(fd)
        raise


def parent_fd(root_fd, name, create=False):
    names = parts(name)
    fd = os.dup(root_fd)
    try:
        for component in names[:-1]:
            child = descend(fd, component, create, trusted=True)
            os.close(fd)
            fd = child
        return fd, names[-1]
    except BaseException:
        os.close(fd)
        raise


def regular_file(parent, leaf):
    fd = os.open(leaf, READ_FLAGS, dir_fd=parent)
    if not stat.S_ISREG(os.fstat(fd).st_mode):
        os.close(fd)
        raise ValueError(f'Refusing non-regular file: {leaf}')
    return fd


def digest(fd):
    hash_value = hashlib.sha256()
    os.lseek(fd, 0, os.SEEK_SET)
    while block := os.read(fd, 1024 * 1024):
        hash_value.update(block)
    os.lseek(fd, 0, os.SEEK_SET)
    return hash_value.hexdigest()


def write_all(fd, data):
    while data:
        written = os.write(fd, data)
        if written <= 0:
            raise OSError('Short installer write')
        data = data[written:]


def manifest_from_fd(fd):
    if os.fstat(fd).st_size > 4 * 1024 * 1024:
        raise ValueError('Manifest exceeds size limit')
    data = os.read(fd, 4 * 1024 * 1024 + 1)
    if len(data) > 4 * 1024 * 1024:
        raise ValueError('Manifest exceeds size limit')
    manifest = json.loads(data)
    if not isinstance(manifest, dict):
        raise ValueError('Manifest must be a JSON object')
    for key in ('version', 'source_commit'):
        if not isinstance(manifest.get(key), str) or not manifest[key]:
            raise ValueError(f'Manifest requires a nonempty {key}')
    files = manifest.get('files')
    if not isinstance(files, dict) or not files:
        raise ValueError('Manifest files must be a nonempty object')
    for name, info in files.items():
        parts(name)
        if not isinstance(info, dict):
            raise ValueError('Manifest file entries must map paths to objects')
        checksum = info.get('sha256')
        if not isinstance(checksum, str) or len(checksum) != 64 or any(
                c not in '0123456789abcdefABCDEF' for c in checksum):
            raise ValueError(f'Invalid manifest checksum: {name}')
        mode = info.get('mode')
        if type(mode) is not int or not 0 <= mode <= 0o777:
            raise ValueError(f'Invalid manifest permissions: {name}')
    return manifest


def source_fd(root_fd, name):
    parent, leaf = parent_fd(root_fd, name)
    try:
        return regular_file(parent, leaf)
    finally:
        os.close(parent)


def same_inode(parent, leaf, metadata):
    current = os.stat(leaf, dir_fd=parent, follow_symlinks=False)
    return (current.st_dev, current.st_ino) == (metadata.st_dev, metadata.st_ino)


def install(root_fd, prefix_fd, manifest):
    sources = []
    created = []
    try:
        for name, info in manifest['files'].items():
            fd = source_fd(root_fd, name)
            sources.append((name, info, fd))
            if digest(fd) != info['sha256']:
                raise ValueError(f'Archive payload checksum mismatch: {name}')
        try:
            parent, leaf = parent_fd(prefix_fd, RECEIPT)
        except FileNotFoundError:
            pass
        else:
            try:
                os.stat(leaf, dir_fd=parent, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                raise ValueError('rfile is already installed at this prefix; uninstall it first')
            finally:
                os.close(parent)
        for name, info, source in sources:
            parent, leaf = parent_fd(prefix_fd, name, create=True)
            try:
                output = os.open(leaf, CREATE_FLAGS, 0o600, dir_fd=parent)
            except FileExistsError as error:
                os.close(parent)
                raise ValueError(f'Refusing symlink or existing file: {name}') from error
            except BaseException:
                os.close(parent)
                raise
            created.append((parent, leaf, os.fstat(output)))
            try:
                copied = hashlib.sha256()
                while block := os.read(source, 1024 * 1024):
                    copied.update(block)
                    write_all(output, block)
                if copied.hexdigest() != info['sha256']:
                    raise ValueError(f'Archive payload changed during installation: {name}')
                os.fchmod(output, info['mode'])
            finally:
                os.close(output)
        parent, leaf = parent_fd(prefix_fd, RECEIPT, create=True)
        try:
            output = os.open(leaf, CREATE_FLAGS, 0o600, dir_fd=parent)
        except BaseException:
            os.close(parent)
            raise
        created.append((parent, leaf, os.fstat(output)))
        try:
            data = (json.dumps(dict(source_commit=manifest['source_commit'],
                                    version=manifest['version'], files=manifest['files']), indent=2) + '\n').encode()
            write_all(output, data)
        finally:
            os.close(output)
    except BaseException:
        for parent, leaf, identity in reversed(created):
            try:
                if same_inode(parent, leaf, identity):
                    os.unlink(leaf, dir_fd=parent)
            except FileNotFoundError:
                pass
        raise
    finally:
        for _, _, fd in sources:
            os.close(fd)
        for parent, _, _ in created:
            os.close(parent)


def uninstall(prefix_fd):
    parent, leaf = parent_fd(prefix_fd, RECEIPT)
    try:
        receipt = regular_file(parent, leaf)
        try:
            manifest = manifest_from_fd(receipt)
            receipt_identity = os.fstat(receipt)
        finally:
            os.close(receipt)
    finally:
        os.close(parent)
    verified = []
    try:
        for name, info in manifest['files'].items():
            parent, leaf = parent_fd(prefix_fd, name)
            try:
                fd = regular_file(parent, leaf)
                try:
                    if digest(fd) != info['sha256']:
                        raise ValueError(f'Refusing to remove changed installed file: {name}')
                    verified.append((parent, leaf, os.fstat(fd)))
                    parent = None
                finally:
                    os.close(fd)
            finally:
                if parent is not None:
                    os.close(parent)
        receipt_parent, receipt_leaf = parent_fd(prefix_fd, RECEIPT)
        try:
            if not same_inode(receipt_parent, receipt_leaf, receipt_identity):
                raise ValueError('Refusing changed installation receipt')
            for parent, leaf, identity in verified:
                if not same_inode(parent, leaf, identity):
                    raise ValueError(f'Refusing changed installed file: {leaf}')
                os.unlink(leaf, dir_fd=parent)
            os.unlink(receipt_leaf, dir_fd=receipt_parent)
        finally:
            os.close(receipt_parent)
        parents = {tuple(parts(name)[:depth]) for name in [*manifest['files'], RECEIPT]
                   for depth in range(1, len(parts(name)))}
        for components in sorted(parents, key=len, reverse=True):
            try:
                parent, leaf = parent_fd(prefix_fd, '/'.join(components))
                try:
                    os.rmdir(leaf, dir_fd=parent)
                finally:
                    os.close(parent)
            except OSError:
                pass
        return manifest
    finally:
        for parent, _, _ in verified:
            os.close(parent)


def main():
    require_platform()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['install', 'uninstall'])
    parser.add_argument('--prefix', required=True, type=Path)
    args = parser.parse_args()
    prefix = args.prefix.expanduser()
    if not prefix.is_absolute():
        parser.error('Prefix must be absolute')
    root = Path(__file__).resolve().parent
    root_fd = os.open(root, DIR_FLAGS)
    try:
        if args.action == 'install':
            archive_manifest_fd = regular_file(root_fd, 'MANIFEST.json')
            try:
                archive_manifest = manifest_from_fd(archive_manifest_fd)
            finally:
                os.close(archive_manifest_fd)
        prefix_fd, prefix = open_prefix(prefix, args.action == 'install')
        try:
            if args.action == 'install':
                install(root_fd, prefix_fd, archive_manifest)
                print(f'Installed rfile {archive_manifest["version"]} into {prefix}')
            else:
                receipt_manifest = uninstall(prefix_fd)
                print(f'Removed rfile {receipt_manifest["version"]} from {prefix}')
        finally:
            os.close(prefix_fd)
    finally:
        os.close(root_fd)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        raise SystemExit(f'rfile installer: {error}') from None
