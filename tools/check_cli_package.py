#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Check the exact CLI/source archives in a disposable prefix and Cargo cache."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('--report', required=True, type=Path)
    args = parser.parse_args()
    package = args.package.resolve()
    release = json.loads((package / 'release-manifest.json').read_text())
    report = dict(source_commit=release['source_commit'], version=release['version'],
                  archives=release['archives'], checks=[], status='running')
    env = {key: value for key, value in os.environ.items()
           if key not in ('MAGIC', 'LD_PRELOAD', 'LD_LIBRARY_PATH', 'POSIXLY_CORRECT')
           and not key.startswith('RFILE_')}
    env.update(LC_ALL='C', TZ='UTC', CARGO_NET_OFFLINE='true')

    def run(name, command, *, cwd=None, expected=0, input=None, timeout=120, extra=None):
        result = subprocess.run(command, cwd=cwd, env=dict(env, **(extra or {})), input=input,
                                capture_output=True, timeout=timeout)
        report['checks'].append(dict(name=name, command=[str(p) for p in command], status=result.returncode,
                                    stdout=result.stdout.decode(errors='replace'), stderr=result.stderr.decode(errors='replace')))
        if (expected == 0 and result.returncode != 0) or (expected != 0 and result.returncode == 0):
            raise RuntimeError(f'{name}: unexpected exit {result.returncode}; {result.stderr.decode(errors="replace")}')
        return result

    try:
        expected_sums = ''.join(f'{value}  {name}\n' for name, value in sorted(release['archives'].items()))
        if (package / 'SHA256SUMS').read_text() != expected_sums:
            raise RuntimeError('SHA256SUMS does not match the release manifest')
        for name, value in release['archives'].items():
            if Path(name).name != name or digest(package / name) != value:
                raise RuntimeError(f'Archive checksum/name mismatch: {name}')
        with tempfile.TemporaryDirectory(prefix='rfile-artifact-check-') as temporary:
            temporary = Path(temporary)
            for name in release['archives']:
                with tarfile.open(package / name) as archive:
                    archive.extractall(temporary, filter='data')
            binary = temporary / f'rfile-{release["version"]}-{release["target"]}'
            source = temporary / f'rfile-{release["version"]}-source'
            manifest = json.loads((binary / 'MANIFEST.json').read_text())
            if any(manifest[key] != release[key] for key in manifest):
                raise RuntimeError('Binary manifest differs from release metadata')
            source_manifest = json.loads((source / 'RELEASE-SOURCE.json').read_text())
            if source_manifest['source_commit'] != release['source_commit'] or source_manifest['version'] != release['version']:
                raise RuntimeError('Source identity differs from binary')
            executable = str(binary / 'bin/rfile')
            version = run('version', [executable, '--version']).stdout.decode()
            if release['version'] not in version:
                raise RuntimeError('Executable reports the wrong version')
            run('help', [executable, '--help'])
            capabilities = json.loads(run('capabilities', [executable, '--capabilities']).stdout)
            if capabilities != json.loads((binary / 'share/rfile/capabilities.json').read_text()):
                raise RuntimeError('Packaged capability manifest differs from executable')
            text = temporary / 'plain input.txt'
            text.write_bytes(b'hello artifact\n')
            mime = run('file-mime', [executable, '-b', '-i', str(text)]).stdout
            if mime != b'text/plain; charset=us-ascii\n':
                raise RuntimeError(f'Unexpected MIME output: {mime!r}')
            stdin = run('stdin', [executable, '-b', '-'], input=b'hello artifact\n').stdout
            if stdin != b'ASCII text\n':
                raise RuntimeError(f'Unexpected stdin output: {stdin!r}')
            zipped = temporary / 'input.gz'
            zipped.write_bytes(gzip.compress(text.read_bytes(), mtime=0))
            if run('gzip-inner', [executable, '-b', '-Z', str(zipped)]).stdout != stdin:
                raise RuntimeError('Default gzip inspection differs from inner text')
            run('missing-file-error', [executable, '-E', str(temporary / 'missing')], expected=1)
            prefix = temporary / 'installed with spaces'
            installer = [sys.executable, str(binary / 'install.py')]
            run('install', [*installer, 'install', '--prefix', str(prefix)])
            run('installed-version', [str(prefix / 'bin/rfile'), '--version'])
            if not (prefix / 'share/man/man1/rfile.1').is_file():
                raise RuntimeError('Installed manual missing')
            run('refuse-existing-install', [*installer, 'install', '--prefix', str(prefix)], expected=1)
            sentinel = prefix / 'unrelated'
            sentinel.write_text('keep me')
            installed = prefix / 'bin/rfile'
            original = installed.read_bytes()
            installed.write_bytes(original + b'changed')
            run('refuse-modified-uninstall', [*installer, 'uninstall', '--prefix', str(prefix)], expected=1)
            if not (prefix / 'share/man/man1/rfile.1').is_file():
                raise RuntimeError('Rejected uninstall partially removed files')
            installed.write_bytes(original)
            run('uninstall', [*installer, 'uninstall', '--prefix', str(prefix)])
            if installed.exists() or not sentinel.is_file():
                raise RuntimeError('Uninstall removed unrelated files or left the executable')
            bad = temporary / 'symlink-prefix'
            outside = temporary / 'outside'
            bad.mkdir(); outside.mkdir()
            (bad / 'bin').symlink_to(outside, target_is_directory=True)
            run('refuse-symlink-prefix', [*installer, 'install', '--prefix', str(bad)], expected=1)
            if list(outside.iterdir()):
                raise RuntimeError('Installer wrote through a symlink')
            cargo = os.environ.get('CARGO', shutil.which('cargo') or str(Path.home() / '.cargo/bin/cargo'))
            # A fresh CARGO_HOME has no bin/ rustup proxies. Keep the explicitly
            # selected Cargo toolchain discoverable even when the caller's PATH
            # omits ~/.cargo/bin and we used the absolute-path fallback above.
            cargo_path = Path(shutil.which(cargo) or cargo).absolute()
            toolchain_path = str(cargo_path.parent) + os.pathsep + env.get('PATH', os.defpath)
            fresh = temporary / 'empty-cargo-home'
            fresh.mkdir()
            run('offline-source-rebuild', [cargo, 'check', '--workspace', '--all-targets', '--all-features', '--locked', '--offline'],
                cwd=source, timeout=300, extra=dict(CARGO_HOME=str(fresh), CARGO_TARGET_DIR=str(temporary / 'source-target'),
                    PATH=toolchain_path, CARGO_BUILD_JOBS=env.get('CARGO_BUILD_JOBS', '1')))
        report['status'] = 'passed'
    except BaseException as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(status=report['status'], checks=len(report['checks']), source_commit=report['source_commit'])))


if __name__ == '__main__':
    main()
