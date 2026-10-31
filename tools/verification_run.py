#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Persist candidate-bound verification evidence, including failed runs."""
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time
import uuid


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def atomic_json(path, value):
    temporary = path.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


class VerificationRun:
    """One writer per checkout; each run retains its own logs and reports.

    SIGINT/SIGTERM checkpoint interruption and stop the active process group.
    SIGKILL/power loss can leave 'running', which is never a passing result.
    """

    def __init__(self, root, features, requirements):
        self.root = Path(root)
        self.reports = self.root / 'reports'
        self.reports.mkdir(exist_ok=True)
        self.lock = (self.reports / '.verification.lock').open('a')
        try:
            fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            self.lock.close()
            raise SystemExit('Another verification run owns this checkout')
        self.run_id = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ-') + uuid.uuid4().hex
        self.directory = self.reports / 'verification-runs' / self.run_id
        (self.directory / 'logs').mkdir(parents=True)
        (self.directory / 'artifacts').mkdir()
        self.summary = self.reports / ('verification-extended.json' if features == 'all-features' else 'verification.json')
        if self.summary.exists():
            shutil.copy2(self.summary, self.directory / 'previous-summary.json')
        self.started = time.monotonic()
        self.report = dict(schema_version=2, run_id=self.run_id, status='running',
                           started_utc=utc(), features=features, requirements=requirements,
                           run_directory=str(self.directory.relative_to(self.root)),
                           steps=[], coverage=[], artifacts={}, oracle=None)
        self.handlers = {}

    def save(self):
        self.report['elapsed_seconds'] = time.monotonic() - self.started
        atomic_json(self.directory / 'report.json', self.report)
        atomic_json(self.summary, self.report)

    def __enter__(self):
        # Invalidate the previous success before even inspecting the oracle.
        self.save()
        for sig in (signal.SIGINT, signal.SIGTERM):
            self.handlers[sig] = signal.getsignal(sig)
            signal.signal(sig, self.interrupt)
        return self

    def identify(self):
        head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=self.root, capture_output=True, text=True)
        status = subprocess.run(['git', 'status', '--porcelain', '--untracked-files=all'], cwd=self.root, capture_output=True, text=True)
        self.report['git_head'] = head.stdout.strip() if head.returncode == 0 else None
        self.report['git_status'] = status.stdout.splitlines() if status.returncode == 0 else None
        self.report['source_sha256'] = self.source_digest()
        self.save()

    def source_digest(self):
        paths = []
        for folder, pattern in [('src', '*.rs'), ('crates', '*.rs'), ('crates', 'Cargo.toml'),
                                ('tests', '*.rs'), ('tools', '*.py'), ('fuzz/fuzz_targets', '*.rs'),
                                ('fuzz/fuzz_targets', '*.inc')]:
            paths.extend((self.root / folder).rglob(pattern))
        paths.extend(self.root / name for name in ['Cargo.toml', 'Cargo.lock', 'fuzz/Cargo.toml',
                     'fuzz/Cargo.lock', 'data/magic.mgc', 'data/UPSTREAM.json'])
        for folder in ('data', 'tests/fixtures', '.github'):
            paths.extend(p for p in (self.root / folder).rglob('*') if p.is_file())
        value = hashlib.sha256()
        for path in sorted(set(paths)):
            if path.is_file():
                value.update(str(path.relative_to(self.root)).encode() + b'\0' + bytes.fromhex(digest(path)))
        return value.hexdigest()

    def interrupt(self, signum, _frame):
        self.report['signal'] = signum
        raise KeyboardInterrupt(f'verification interrupted by signal {signum}')

    def archive(self, path):
        destination = self.directory / 'artifacts' / path.name
        shutil.copy2(path, destination)
        self.report['artifacts'][path.name] = dict(path=str(destination.relative_to(self.root)), sha256=digest(destination))

    def report_stamps(self):
        return {p: (p.stat().st_mtime_ns, p.stat().st_size) for p in self.reports.iterdir()
                if p.is_file() and p.suffix in ('.json', '.txt')
                and p.name not in ('verification.json', 'verification-extended.json')}

    def run(self, name, command, env):
        print(f'Running {name}', flush=True)
        started = time.monotonic()
        log_path = self.directory / 'logs' / (name + '.log')
        row = dict(name=name, status=None, state='running', command=command,
                   log=str(log_path.relative_to(self.root)))
        self.report['steps'].append(row)
        self.save()
        before = self.report_stamps()
        child = None
        try:
            with log_path.open('wb') as log:
                child = subprocess.Popen(command, cwd=self.root, env=env, stdout=log,
                                         stderr=subprocess.STDOUT, start_new_session=True)
                row['status'] = child.wait()
                row['state'] = 'passed' if row['status'] == 0 else 'failed'
        except BaseException:
            row['state'] = 'interrupted' if self.report.get('signal') else 'failed'
            if child is not None:
                # Kill the group even if the leader exited before its children.
                try:
                    os.killpg(child.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                row['status'] = child.returncode
            raise
        finally:
            row['seconds'] = time.monotonic() - started
            if log_path.exists():
                row['log_sha256'] = digest(log_path)
                for line in log_path.read_text(errors='replace').splitlines():
                    if line.startswith('RFILE_COVERAGE '):
                        self.report['coverage'].append(dict(step=name, **json.loads(line.removeprefix('RFILE_COVERAGE '))))
            for path, stamp in self.report_stamps().items():
                if before.get(path) != stamp:
                    self.archive(path)
            self.save()
        if row['status'] != 0:
            raise SystemExit(f'{name} failed; see {log_path}')

    def __exit__(self, kind, error, _traceback):
        try:
            if error is None:
                if self.report.get('source_sha256') != self.source_digest():
                    error = RuntimeError('Source changed during verification; rerun a frozen candidate')
            self.report['status'] = ('interrupted' if kind is KeyboardInterrupt else 'failed') if error is not None else 'passed'
            if error is not None:
                self.report['error'] = str(error)
            self.report['finished_utc'] = utc()
            self.save()
            if kind is None and error is not None:
                raise error
        finally:
            for sig, handler in self.handlers.items():
                signal.signal(sig, handler)
            fcntl.flock(self.lock, fcntl.LOCK_UN)
            self.lock.close()
        return False
