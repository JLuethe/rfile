#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build an experimental CLI and vendored source bundle from a clean Git commit.

This creates local archives only. It never installs, tags, pushes or publishes.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def archive_tree(tree, output, name, epoch):
    with output.open('wb') as stream:
        with gzip.GzipFile(filename='', mode='wb', fileobj=stream, mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode='w') as archive:
                for path in sorted(tree.rglob('*')):
                    info = archive.gettarinfo(str(path), arcname=name + '/' + str(path.relative_to(tree)))
                    info.uid = info.gid = 0
                    info.uname = info.gname = ''
                    info.mtime = epoch
                    if path.is_file():
                        with path.open('rb') as content:
                            archive.addfile(info, content)
                    else:
                        archive.addfile(info)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        parser.error('The initial CLI artifact supports x86_64 Linux/glibc only')
    if subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=all'], cwd=ROOT):
        raise SystemExit('Release packaging requires a clean committed checkout; use an isolated clone')
    output = args.output.resolve()
    if output.exists():
        raise SystemExit('Output directory already exists; choose a new one')
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    epoch = int(subprocess.check_output(['git', 'show', '-s', '--format=%ct', 'HEAD'], cwd=ROOT, text=True))
    cargo = os.environ.get('CARGO', shutil.which('cargo') or str(Path.home() / '.cargo/bin/cargo'))
    rustc = str(Path(cargo).with_name('rustc'))
    offline = ['--offline'] if args.offline else []
    with tempfile.TemporaryDirectory(prefix='rfile-cli-package-') as temporary:
        temporary = Path(temporary)
        source = temporary / 'source'
        source.mkdir()
        snapshot = temporary / 'git-source.tar'
        with snapshot.open('wb') as stream:
            subprocess.run(['git', 'archive', '--format=tar', commit], cwd=ROOT, stdout=stream, check=True)
        with tarfile.open(snapshot) as archive:
            archive.extractall(source, filter='data')
        version = tomllib.loads((source / 'Cargo.toml').read_text())['package']['version']
        target = 'x86_64-unknown-linux-gnu'
        target_directory = temporary / 'target'
        env = dict(os.environ, LC_ALL='C', TZ='UTC', SOURCE_DATE_EPOCH=str(epoch),
                   CARGO_TARGET_DIR=str(target_directory))
        # Do not inherit ad-hoc developer compiler flags or alternate build profiles.
        for key in list(env):
            if key.startswith('CARGO_PROFILE_') or key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS'):
                env.pop(key)
        cargo_home = Path(env.get('CARGO_HOME', str(Path.home() / '.cargo'))).resolve()
        env['CARGO_ENCODED_RUSTFLAGS'] = '\x1f'.join([
            '--remap-path-prefix=' + str(source) + '=/rfile-source',
            '--remap-path-prefix=' + str(cargo_home) + '=/cargo-home',
        ])
        command = [cargo, 'build', '-p', 'rfile', '--bin', 'rfile', '--release', '--locked',
                   '--no-default-features', '--target', target, *offline]
        subprocess.run(command, cwd=source, env=env, check=True)
        binary = target_directory / target / 'release/rfile'
        dynamic = subprocess.check_output(['readelf', '-d', str(binary)], text=True)
        if 'libmagic.so' in dynamic:
            raise SystemExit('Unexpected native libmagic linkage')
        symbols = subprocess.check_output(['readelf', '--version-info', str(binary)], text=True)
        glibc = sorted(set(re.findall(r'GLIBC_(\d+\.\d+(?:\.\d+)?)', symbols)),
                       key=lambda value: tuple(map(int, value.split('.'))))
        if not glibc:
            raise SystemExit('Could not determine the glibc symbol requirement')
        capabilities = json.loads(subprocess.check_output([str(binary), '--capabilities'], env=env))
        stage = temporary / 'binary'
        (stage / 'bin').mkdir(parents=True)
        (stage / 'share/man/man1').mkdir(parents=True)
        (stage / 'share/rfile').mkdir(parents=True)
        licenses = stage / 'share/licenses/rfile'
        licenses.mkdir(parents=True)
        shutil.copy2(binary, stage / 'bin/rfile')
        shutil.copy2(source / 'docs/rfile.1', stage / 'share/man/man1/rfile.1')
        for name in ('LICENSE', 'NOTICE'):
            shutil.copy2(source / name, licenses / name)
        shutil.copytree(source / 'LICENSES', licenses / 'LICENSES')
        for name in ('UPSTREAM-COPYING', 'UPSTREAM.json', 'SOURCES.json'):
            shutil.copy2(source / 'data' / name, licenses / name)
        shutil.copy2(source / 'README.md', stage / 'share/rfile/README.md')
        shutil.copy2(source / 'packaging/INSTALL.md', stage / 'INSTALL.md')
        shutil.copy2(source / 'packaging/install.py', stage / 'install.py')
        (stage / 'share/rfile/capabilities.json').write_text(json.dumps(capabilities, indent=2) + '\n')
        files = {str(path.relative_to(stage)): dict(sha256=digest(path), mode=0o755 if path == stage / 'bin/rfile' else 0o644)
                 for folder in ('bin', 'share') for path in sorted((stage / folder).rglob('*')) if path.is_file()}
        for name, info in files.items():
            (stage / name).chmod(info['mode'])
        manifest = dict(schema_version=1, version=version, source_commit=commit, target=target,
                        experimental=True, features=[], glibc_required=glibc[-1],
                        rustc=subprocess.check_output([rustc, '--version'], text=True).strip(),
                        cargo=subprocess.check_output([cargo, '--version'], text=True).strip(),
                        database_sha256=digest(source / 'data/magic.mgc'),
                        lock_sha256=digest(source / 'Cargo.lock'), files=files)
        (stage / 'MANIFEST.json').write_text(json.dumps(manifest, indent=2) + '\n')
        # The source archive starts with exactly the committed tree, then adds
        # locked vendored crates and their Cargo configuration for offline rebuilds.
        vendor = subprocess.run([cargo, 'vendor', '--locked', *offline, '--sync', 'fuzz/Cargo.toml', 'vendor'],
                                cwd=source, env=env, check=True, capture_output=True, text=True)
        (source / '.cargo').mkdir(exist_ok=True)
        (source / '.cargo/config.toml').write_text(vendor.stdout)
        (source / 'RELEASE-SOURCE.json').write_text(json.dumps(dict(source_commit=commit,
            version=version, git_archive_sha256=digest(snapshot), additions=['vendor/', '.cargo/config.toml', 'RELEASE-SOURCE.json']), indent=2) + '\n')
        # Normalize source modes too: temporary-directory umasks must not change archives.
        for path in source.rglob('*'):
            if path.is_dir():
                path.chmod(0o755)
            elif path.is_file() and not path.is_symlink():
                path.chmod(0o755 if path.stat().st_mode & 0o111 else 0o644)
        for path in stage.rglob('*'):
            if path.is_dir():
                path.chmod(0o755)
        for name in ('install.py', 'INSTALL.md', 'MANIFEST.json'):
            (stage / name).chmod(0o644)
        bundles = temporary / 'bundles'
        bundles.mkdir()
        binary_name = f'rfile-{version}-{target}'
        source_name = f'rfile-{version}-source'
        archive_tree(stage, bundles / (binary_name + '.tar.gz'), binary_name, epoch)
        archive_tree(source, bundles / (source_name + '.tar.gz'), source_name, epoch)
        (bundles / 'SHA256SUMS').write_text(''.join(f'{digest(path)}  {path.name}\n' for path in sorted(bundles.glob('*.tar.gz'))))
        (bundles / 'release-manifest.json').write_text(json.dumps(dict(**manifest,
            archives={path.name: digest(path) for path in sorted(bundles.glob('*.tar.gz'))}), indent=2) + '\n')
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(bundles, output)
    print(output)


if __name__ == '__main__':
    main()
