#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Independent regression tests for tools/verification_run.py.

Locates the module via RFILE_TEST_TOOLS, else this file's directory (so it
works when integrated into tools/). Every test uses a fresh temporary git
root and drives the real VerificationRun with small real subprocesses; no
production code is mocked. Failures exit nonzero via unittest.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest

TOOLS = Path(os.environ.get('RFILE_TEST_TOOLS') or Path(__file__).resolve().parent).resolve()
sys.path.insert(0, str(TOOLS))
import verification_run  # noqa: E402
from verification_run import VerificationRun  # noqa: E402

PY = sys.executable


def git(root, *args):
    subprocess.run(['git', *args], cwd=root, check=True, capture_output=True)


def make_root(base):
    root = Path(base) / 'checkout'
    (root / 'src').mkdir(parents=True)
    (root / 'src' / 'lib.rs').write_text('pub fn f() {}\n')
    (root / 'Cargo.toml').write_text('[package]\nname = "x"\n')
    (root / '.gitignore').write_text('reports/\n')
    git(root, 'init', '-q')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'add', '.')
    git(root, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-qm', 'init')
    return root


def load(path):
    return json.loads(Path(path).read_text())


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    # Zombies count as dead.
    try:
        return Path(f'/proc/{pid}/stat').read_text().split()[2] != 'Z'
    except FileNotFoundError:
        return False


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='vrun-test-')
        self.root = make_root(self.tmp.name)
        self.env = dict(os.environ)
        self.summary = self.root / 'reports' / 'verification.json'

    def tearDown(self):
        self.tmp.cleanup()

    def stale_pass(self):
        (self.root / 'reports').mkdir(exist_ok=True)
        self.summary.write_text(json.dumps(dict(status='passed', run_id='stale')) + '\n')

    def holder_script(self, body):
        """Child that owns a VerificationRun on self.root and runs body after identify."""
        lines = ['import sys, time', f'sys.path.insert(0, {str(TOOLS)!r})',
                 'from verification_run import VerificationRun',
                 f'with VerificationRun({str(self.root)!r}, "default", {{}}) as run:',
                 '    run.identify()']
        lines += ['    ' + line for line in textwrap.dedent(body).strip().splitlines()]
        return '\n'.join(lines) + '\n'


class StartupAndSuccess(Base):
    def test_startup_replaces_stale_pass_before_identify(self):
        self.stale_pass()
        with VerificationRun(self.root, 'default', {}) as run:
            during = load(self.summary)
            self.assertEqual(during['status'], 'running')
            self.assertEqual(during['run_id'], run.run_id)
            self.assertEqual(load(run.directory / 'previous-summary.json')['run_id'], 'stale')
            run.identify()  # required precondition for a normal completion
        self.assertEqual(load(self.summary)['status'], 'passed')

    def test_normal_completion_without_identify_fails(self):
        with self.assertRaises(RuntimeError):
            with VerificationRun(self.root, 'default', {}):
                pass
        self.assertEqual(load(self.summary)['status'], 'failed')

    def test_success_retains_logs_hashes_coverage(self):
        line = 'RFILE_COVERAGE ' + json.dumps(dict(component='c', status='executed'))
        with VerificationRun(self.root, 'all-features', dict(x=1)) as run:
            run.identify()
            run.run('one', [PY, '-c', f'print("hello"); print({line!r})'], self.env)
            run.run('two', [PY, '-c', 'import sys; sys.stderr.write("err\\n")'], self.env)
        summary = load(self.root / 'reports' / 'verification-extended.json')
        retained = load(run.directory / 'report.json')
        self.assertEqual(summary, retained)
        self.assertEqual(summary['status'], 'passed')
        self.assertEqual(summary['git_head'], subprocess.check_output(
            ['git', 'rev-parse', 'HEAD'], cwd=self.root, text=True).strip())
        self.assertEqual(len(summary['source_sha256']), 64)
        self.assertEqual([s['state'] for s in summary['steps']], ['passed', 'passed'])
        for step in summary['steps']:
            log = self.root / step['log']
            self.assertTrue(log.is_file())
            self.assertEqual(step['log_sha256'], verification_run.digest(log))
        self.assertIn(b'err', (self.root / summary['steps'][1]['log']).read_bytes())
        self.assertEqual(summary['coverage'], [dict(step='one', component='c', status='executed')])

    def test_distinct_run_directories(self):
        ids = []
        for _ in range(2):
            with VerificationRun(self.root, 'default', {}) as run:
                run.identify()
                run.run('s', [PY, '-c', 'print(1)'], self.env)
            ids.append(run.directory)
        self.assertNotEqual(ids[0], ids[1])
        self.assertTrue(all((d / 'report.json').is_file() for d in ids))


class Failures(Base):
    def test_nonzero_exit(self):
        self.stale_pass()
        with self.assertRaises(SystemExit):
            with VerificationRun(self.root, 'default', {}) as run:
                run.identify()
                run.run('bad', [PY, '-c', 'print("x"); raise SystemExit(3)'], self.env)
                run.run('never', [PY, '-c', 'pass'], self.env)
        s = load(self.summary)
        self.assertEqual(s['status'], 'failed')
        self.assertEqual([(x['name'], x['state'], x['status']) for x in s['steps']], [('bad', 'failed', 3)])
        self.assertEqual(s['steps'][0]['log_sha256'], verification_run.digest(self.root / s['steps'][0]['log']))

    def test_missing_executable(self):
        with self.assertRaises(FileNotFoundError):
            with VerificationRun(self.root, 'default', {}) as run:
                run.identify()
                run.run('missing', [str(self.root / 'no-such-binary')], self.env)
        s = load(self.summary)
        self.assertEqual(s['status'], 'failed')
        self.assertEqual(s['steps'][0]['state'], 'failed')
        self.assertNotEqual(s['steps'][0]['status'], 0)

    def test_early_failure_before_identify(self):
        self.stale_pass()
        with self.assertRaises(ValueError):
            with VerificationRun(self.root, 'default', {}):
                raise ValueError('oracle unavailable')
        s = load(self.summary)
        self.assertEqual(s['status'], 'failed')
        self.assertIn('oracle unavailable', s['error'])

    def test_source_mutation_invalidates(self):
        with self.assertRaises(RuntimeError):
            with VerificationRun(self.root, 'default', {}) as run:
                run.identify()
                run.run('mutate', [PY, '-c', 'open("src/lib.rs","a").write("//x\\n")'], self.env)
        s = load(self.summary)
        self.assertEqual(s['status'], 'failed')
        self.assertIn('Source changed', s['error'])


class Concurrency(Base):
    def test_lock_contention(self):
        holder = subprocess.Popen([PY, '-c', self.holder_script('''
            print('ready', flush=True)
            sys.stdin.readline()
        ''')],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(holder.stdout.readline().strip(), 'ready')
            owned = load(self.summary)
            with self.assertRaises(SystemExit) as caught:
                VerificationRun(self.root, 'default', {})
            self.assertIn('Another verification run', str(caught.exception))
            self.assertEqual(load(self.summary), owned)
            holder.stdin.write('\n')
            holder.stdin.flush()
            self.assertEqual(holder.wait(timeout=30), 0)
        finally:
            if holder.poll() is None:
                holder.kill()
                holder.wait()
        self.assertEqual(load(self.summary)['status'], 'passed')
        with VerificationRun(self.root, 'default', {}) as run:
            run.identify()

    def test_sigterm_cleans_group_including_term_ignoring_child(self):
        pids = Path(self.tmp.name) / 'pids'
        child_code = textwrap.dedent(f'''
            import os, signal, subprocess, sys, time
            grandchild = subprocess.Popen([sys.executable, '-c',
                'import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(300)'])
            open({str(pids)!r}, 'w').write(f'{{os.getpid()}} {{grandchild.pid}}\\n')
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            time.sleep(300)
        ''')
        script = textwrap.dedent(f'''
            import sys
            sys.path.insert(0, {str(TOOLS)!r})
            from verification_run import VerificationRun
            with VerificationRun({str(self.root)!r}, 'default', {{}}) as run:
                run.identify()
                run.run('hang', [sys.executable, '-c', {child_code!r}], None)
        ''')
        parent = subprocess.Popen([PY, '-c', script], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 30
            while not (pids.exists() and pids.read_text().endswith('\n')):
                self.assertLess(time.monotonic(), deadline, 'child never started')
                time.sleep(0.05)
            group = [int(p) for p in pids.read_text().split()]
            self.assertTrue(all(alive(p) for p in group))
            parent.send_signal(signal.SIGTERM)
            parent.communicate(timeout=30)
            self.assertNotEqual(parent.returncode, 0)
            deadline = time.monotonic() + 10
            while any(alive(p) for p in group) and time.monotonic() < deadline:
                time.sleep(0.05)
            survivors = [p for p in group if alive(p)]
            self.assertEqual(survivors, [], 'process group members survived SIGTERM cleanup')
        finally:
            if parent.poll() is None:
                parent.kill()
                parent.wait()
            if pids.exists():
                for p in pids.read_text().split():
                    try:
                        os.kill(int(p), signal.SIGKILL)
                    except (ProcessLookupError, ValueError):
                        pass
        s = load(self.summary)
        self.assertEqual(s['status'], 'interrupted')
        self.assertEqual(s['signal'], signal.SIGTERM)
        self.assertEqual(s['steps'][0]['state'], 'interrupted')
        self.assertIn('log_sha256', s['steps'][0])


if __name__ == '__main__':
    print(f'verification_run module: {verification_run.__file__}', flush=True)
    unittest.main(verbosity=2)
