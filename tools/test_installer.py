# SPDX-License-Identifier: GPL-3.0-only
"""Expected installer refusals must be concise and preserve existing files."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class InstallerDiagnostics(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='rfile-installer-')
        self.root = Path(self.tmp.name)
        self.archive = self.root / 'archive'
        (self.archive / 'bin').mkdir(parents=True)
        payload = b'test executable\n'
        (self.archive / 'bin/rfile').write_bytes(payload)
        shutil.copyfile(Path(__file__).resolve().parents[1] / 'packaging/install.py', self.archive / 'install.py')
        (self.archive / 'MANIFEST.json').write_text(json.dumps(dict(version='test', source_commit='test',
            files={'bin/rfile': {'sha256': hashlib.sha256(payload).hexdigest(), 'mode': 0o755}})))

    def tearDown(self):
        self.tmp.cleanup()

    def refuse(self, prefix):
        result = subprocess.run([sys.executable, str(self.archive / 'install.py'), 'install', '--prefix', str(prefix)],
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 1)
        self.assertFalse(result.stdout)
        self.assertIn(b'rfile installer:', result.stderr)
        self.assertNotIn(b'Traceback', result.stderr)
        return result

    def test_symlink_destination_refusal_preserves_target(self):
        prefix = self.root / 'prefix'
        (prefix / 'bin').mkdir(parents=True)
        precious = self.root / 'precious'
        precious.write_bytes(b'keep')
        (prefix / 'bin/rfile').symlink_to(precious)
        self.assertIn(b'Refusing symlink', self.refuse(prefix).stderr)
        self.assertEqual(precious.read_bytes(), b'keep')
        self.assertFalse((prefix / 'share/rfile/install-receipt.json').exists())

    def test_uncreatable_prefix_is_a_normal_error(self):
        blocker = self.root / 'regular-file'
        blocker.write_bytes(b'keep')
        self.refuse(blocker / 'prefix')
        self.assertEqual(blocker.read_bytes(), b'keep')

    def test_invalid_manifest_schema_fails_before_creating_prefix(self):
        path = self.archive / 'MANIFEST.json'
        valid = json.loads(path.read_text())
        mutations = [lambda m: m.pop('version'), lambda m: m.pop('source_commit'),
                     lambda m: m.__setitem__('files', []),
                     lambda m: m['files']['bin/rfile'].pop('mode'),
                     lambda m: m['files'].__setitem__('bin/rfile', 'bad'),
                     lambda m: m['files']['bin/rfile'].__setitem__('mode', '755'),
                     lambda m: m['files']['bin/rfile'].__setitem__('mode', 2**70)]
        for mutate in mutations:
            manifest = json.loads(json.dumps(valid))
            mutate(manifest)
            path.write_text(json.dumps(manifest))
            prefix = self.root / 'prefix'
            self.refuse(prefix)
            self.assertFalse(prefix.exists())
        path.write_text('[]')
        self.refuse(self.root / 'prefix')
        self.assertFalse((self.root / 'prefix').exists())

    def test_malformed_manifest_is_a_normal_error(self):
        (self.archive / 'MANIFEST.json').write_text('{')
        prefix = self.root / 'prefix'
        self.refuse(prefix)
        self.assertFalse(prefix.exists())
