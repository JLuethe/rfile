#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Run bounded timezone and smoke regressions for one rfile binary in an OCI container.

Example:
  python3 timezone-container-regression.py --engine podman --image \
    docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11 \
    --binary ./target/x86_64-unknown-linux-gnu/release/rfile --report result.json
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--engine', required=True, choices=('podman', 'docker'))
    parser.add_argument('--image', required=True, help='Pinned OCI image reference containing @sha256:')
    parser.add_argument('--report', required=True, type=Path)
    parser.add_argument('--timeout', type=int, default=30)
    parser.add_argument('--scratch-root', type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    if '@sha256:' not in args.image:
        parser.error('--image must be pinned by digest (contain @sha256:)')
    if not os.access(binary, os.X_OK):
        parser.error('binary must be executable')
    scratch = tempfile.mkdtemp(prefix='rfile-timezone-container-', dir=args.scratch_root)
    root = Path(scratch)
    fixture = root / 'fixtures'
    fixture.mkdir()
    (fixture / 'plain.txt').write_bytes(b'hello artifact\n')
    (fixture / 'input.gz').write_bytes(gzip.compress(b'hello artifact\n', mtime=0))
    malformed = root / 'malformed-localtime'
    malformed.write_bytes(b'not a TZif file\n')
    unreadable = root / 'unreadable-localtime'
    unreadable.write_bytes(b'not readable')
    unreadable.chmod(0)
    host_utc = Path('/usr/share/zoneinfo/UTC')
    if not host_utc.is_file():
        shutil.rmtree(root, ignore_errors=True)
        raise RuntimeError(f'required valid TZif fixture is missing: {host_utc}')
    binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    results = []
    clean_engine_env = dict(os.environ)
    clean_engine_env.pop('TZ', None)
    clean_engine_env.pop('MAGIC', None)

    def invoke(name, argv, *, tz=None, localtime=None, expected=0, assert_json_zone=None,
               expected_stdout=None, expected_stderr_substring=None, input_data=None):
        container_name = 'rfile-tz-qa-' + uuid.uuid4().hex[:12]
        cmd = [args.engine, 'run', '--name', container_name, '--interactive', '--network=none', '--read-only', '--cap-drop=ALL',
               '--security-opt=no-new-privileges', '--user=65534:65534', '--memory=256m',
               '--pids-limit=32', '--cpus=1', '--volume', f'{binary}:/qa/rfile:ro',
               '--volume', f'{fixture}:/fixtures:ro']
        if tz is None:
            # Pass a deliberately absent host variable so engines do not inherit TZ.
            cmd += ['--env', 'TZ']
        else:
            cmd += ['--env', 'TZ=' + tz]
        if localtime is not None:
            cmd += ['--volume', f'{localtime}:/etc/localtime:ro']
        cmd += [args.image, '/qa/rfile', *argv]
        record = {'name': name, 'command': cmd, 'status': 'running'}
        results.append(record)
        timeout_error = None
        run_error = None
        proc = None
        try:
            proc = subprocess.run(cmd, env=clean_engine_env, input=input_data, capture_output=True, text=True,
                                  timeout=args.timeout, check=False)
            record.update(status=proc.returncode, stdout=proc.stdout, stderr=proc.stderr)
        except subprocess.TimeoutExpired as error:
            timeout_error = error
            record.update(status='timeout', stdout=(error.stdout or b'').decode(errors='replace'),
                          stderr=(error.stderr or b'').decode(errors='replace'))
        except BaseException as error:
            run_error = error
            record.update(status='launch_error', stdout='', stderr=f'{type(error).__name__}: {error}')
        try:
            cleanup = subprocess.run([args.engine, 'rm', '--force', container_name], env=clean_engine_env,
                                      capture_output=True, text=True, timeout=min(10, args.timeout), check=False)
            record['cleanup'] = {'status': cleanup.returncode, 'stdout': cleanup.stdout, 'stderr': cleanup.stderr}
        except BaseException as error:
            record['cleanup'] = {'status': 'error', 'stdout': '', 'stderr': f'{type(error).__name__}: {error}'}
        if record['cleanup']['status'] != 0:
            raise AssertionError(f'{name}: engine cleanup failed: {record["cleanup"]}')
        if timeout_error is not None:
            raise AssertionError(f'{name} timed out after {args.timeout}s')
        if run_error is not None:
            raise AssertionError(f'{name} failed to launch: {run_error}')
        if proc.returncode != expected:
            raise AssertionError(f'{name}: expected exit {expected}, got {proc.returncode}: {proc.stderr}')
        if expected_stdout is not None and proc.stdout != expected_stdout:
            raise AssertionError(f'{name}: unexpected stdout {proc.stdout!r}')
        if expected_stderr_substring and expected_stderr_substring not in proc.stderr:
            raise AssertionError(f'{name}: stderr lacks {expected_stderr_substring!r}: {proc.stderr!r}')
        if assert_json_zone is not None:
            payload = json.loads(proc.stdout)
            if payload.get('time_zone') != assert_json_zone:
                raise AssertionError(f'{name}: time_zone expected {assert_json_zone!r}; got {payload.get("time_zone")!r}')
        return proc

    status = 'failed'
    error = None
    try:
        # Runtime baseline, with the pinned Ubuntu image's absent /etc/localtime.
        version = invoke('version-without-localtime', ['--version'], expected_stdout=None)
        if 'rfile-' not in version.stdout:
            raise AssertionError(f'unexpected version output: {version.stdout!r}')
        invoke('implicit-missing-localtime-capabilities', ['--capabilities'], assert_json_zone='UTC')
        invoke('implicit-missing-localtime-classification', ['-b', '/fixtures/plain.txt'],
               expected_stdout='ASCII text\n')
        invoke('implicit-missing-localtime-stdin', ['-b', '-'], expected_stdout='ASCII text\n', input_data='hello artifact\n')
        invoke('implicit-missing-localtime-gzip', ['-b', '-Z', '/fixtures/input.gz'],
               expected_stdout='ASCII text\n')
        invoke('explicit-tz-utc', ['--capabilities'], tz='UTC', assert_json_zone='UTC')
        invoke('explicit-tz-utc0', ['--capabilities'], tz='UTC0', assert_json_zone='UTC0')
        invoke('explicit-empty-tz', ['--capabilities'], tz='', assert_json_zone='UTC')
        invoke('explicit-missing-tz-path', ['--capabilities'], tz=':/qa/no-such-zone', expected=1,
               expected_stderr_substring='No such file or directory')
        invoke('explicit-invalid-tz-name', ['--capabilities'], tz='Definitely/NotAZone', expected=1)
        invoke('malformed-existing-localtime', ['--capabilities'], localtime=malformed, expected=1)
        invoke('unreadable-existing-localtime', ['--capabilities'], localtime=unreadable, expected=1)
        invoke('valid-existing-localtime', ['--capabilities'], localtime=host_utc, expected=0)
        status = 'passed'
    except BaseException as exc:
        error = f'{type(exc).__name__}: {exc}'
        raise
    finally:
        report = {'status': status, 'error': error, 'source_binary': str(binary),
                  'source_binary_sha256': binary_sha256, 'engine': args.engine, 'image': args.image, 'checks': results,
                  'check_count': len(results)}
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + '\n')
        # Containers are explicitly removed after each command; remove all scratch fixtures.
        shutil.rmtree(root, ignore_errors=True)
    print(json.dumps({'status': status, 'checks': len(results), 'report': str(args.report)}))


if __name__ == '__main__':
    main()
