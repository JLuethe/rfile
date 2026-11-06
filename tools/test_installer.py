# SPDX-License-Identifier: GPL-3.0-only
"""Expected installer refusals must be concise and preserve existing files."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


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

    def test_install_uninstall_preserves_unrelated_files(self):
        prefix = self.root / 'prefix'
        prefix.mkdir()
        unrelated = prefix / 'keep'
        unrelated.write_bytes(b'untouched')
        install = subprocess.run([sys.executable, str(self.archive / 'install.py'), 'install',
                                  '--prefix', str(prefix)], capture_output=True, timeout=10)
        self.assertEqual(install.returncode, 0, install.stderr)
        self.assertEqual((prefix / 'bin/rfile').read_bytes(), b'test executable\n')
        receipt = prefix / 'share/rfile/install-receipt.json'
        self.assertTrue(receipt.is_file())
        uninstall = subprocess.run([sys.executable, str(self.archive / 'install.py'), 'uninstall',
                                    '--prefix', str(prefix)], capture_output=True, timeout=10)
        self.assertEqual(uninstall.returncode, 0, uninstall.stderr)
        self.assertFalse((prefix / 'bin/rfile').exists())
        self.assertFalse(receipt.exists())
        self.assertEqual(unrelated.read_bytes(), b'untouched')

    def test_component_swap_cannot_redirect_install(self):
        spec = importlib.util.spec_from_file_location('rfile_installer_test', self.archive / 'install.py')
        installer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(installer)
        prefix = self.root / 'prefix'
        (prefix / 'bin').mkdir(parents=True)
        outside = self.root / 'outside'
        outside.mkdir()
        manifest = json.loads((self.archive / 'MANIFEST.json').read_text())
        archive_fd = os.open(self.archive, installer.DIR_FLAGS)
        prefix_fd = os.open(prefix, installer.DIR_FLAGS)
        original_open = os.open
        swapped = False

        def swap_before_open(path, flags, *args, **kwargs):
            nonlocal swapped
            if path == 'bin' and kwargs.get('dir_fd') is not None and not swapped:
                swapped = True
                (prefix / 'bin').rename(prefix / 'bin-old')
                (prefix / 'bin').symlink_to(outside, target_is_directory=True)
            return original_open(path, flags, *args, **kwargs)

        try:
            with mock.patch.object(installer.os, 'open', side_effect=swap_before_open):
                with self.assertRaises((OSError, ValueError)):
                    installer.install(archive_fd, prefix_fd, manifest)
        finally:
            os.close(archive_fd)
            os.close(prefix_fd)
        self.assertTrue(swapped)
        self.assertFalse((outside / 'rfile').exists())
        self.assertFalse((prefix / 'share/rfile/install-receipt.json').exists())

    def test_malicious_receipt_path_is_rejected_before_unlink(self):
        prefix = self.root / 'prefix'
        (prefix / 'share/rfile').mkdir(parents=True)
        outside = self.root / 'outside'
        outside.write_bytes(b'keep')
        receipt = prefix / 'share/rfile/install-receipt.json'
        receipt.write_text(json.dumps(dict(version='test', source_commit='test', files={
            '../outside': {'sha256': hashlib.sha256(b'keep').hexdigest(), 'mode': 0o644}})))
        result = subprocess.run([sys.executable, str(self.archive / 'install.py'), 'uninstall',
                                 '--prefix', str(prefix)], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(outside.read_bytes(), b'keep')
        self.assertTrue(receipt.exists())

    def test_component_swap_cannot_redirect_uninstall(self):
        spec = importlib.util.spec_from_file_location('rfile_installer_uninstall_test', self.archive / 'install.py')
        installer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(installer)
        prefix = self.root / 'prefix'
        result = subprocess.run([sys.executable, str(self.archive / 'install.py'), 'install',
                                 '--prefix', str(prefix)], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        outside = self.root / 'outside'
        outside.mkdir()
        (outside / 'rfile').write_bytes(b'test executable\n')
        prefix_fd = os.open(prefix, installer.DIR_FLAGS)
        original_open = os.open
        swapped = False

        def swap_before_open(path, flags, *args, **kwargs):
            nonlocal swapped
            if path == 'bin' and kwargs.get('dir_fd') is not None and not swapped:
                swapped = True
                (prefix / 'bin').rename(prefix / 'bin-old')
                (prefix / 'bin').symlink_to(outside, target_is_directory=True)
            return original_open(path, flags, *args, **kwargs)

        try:
            with mock.patch.object(installer.os, 'open', side_effect=swap_before_open):
                with self.assertRaises((OSError, ValueError)):
                    installer.uninstall(prefix_fd)
        finally:
            os.close(prefix_fd)
        self.assertTrue(swapped)
        self.assertEqual((outside / 'rfile').read_bytes(), b'test executable\n')
        self.assertTrue((prefix / 'bin-old/rfile').exists())

    def test_unsupported_platform_fails_closed(self):
        spec = importlib.util.spec_from_file_location('rfile_installer_platform_test', self.archive / 'install.py')
        installer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(installer)
        with mock.patch.object(installer.os, 'name', 'nt'):
            with self.assertRaisesRegex(ValueError, 'unavailable'):
                installer.require_platform()

    def test_partial_failure_rolls_back_created_files(self):
        other = self.archive / 'share/man/rfile.1'
        other.parent.mkdir(parents=True)
        other.write_bytes(b'manual\n')
        manifest_path = self.archive / 'MANIFEST.json'
        manifest = json.loads(manifest_path.read_text())
        manifest['files']['share/man/rfile.1'] = {
            'sha256': hashlib.sha256(b'manual\n').hexdigest(), 'mode': 0o644}
        manifest_path.write_text(json.dumps(manifest))
        prefix = self.root / 'prefix'
        blocker = prefix / 'share/man/rfile.1'
        blocker.parent.mkdir(parents=True)
        blocker.write_bytes(b'keep')
        self.refuse(prefix)
        self.assertFalse((prefix / 'bin/rfile').exists())
        self.assertEqual(blocker.read_bytes(), b'keep')
        self.assertFalse((prefix / 'share/rfile/install-receipt.json').exists())

    def test_shared_writable_prefix_ancestor_is_rejected(self):
        shared = self.root / 'shared'
        shared.mkdir(mode=0o777)
        shared.chmod(0o777)
        prefix = shared / 'prefix'
        result = self.refuse(prefix)
        self.assertIn(b'writable prefix ancestor', result.stderr)
        self.assertFalse(prefix.exists())

    def test_prefix_symlink_alias_is_rejected(self):
        outside = self.root / 'outside'
        outside.mkdir()
        alias = self.root / 'prefix'
        alias.symlink_to(outside, target_is_directory=True)
        self.refuse(alias)
        self.assertFalse((outside / 'bin/rfile').exists())
        self.assertFalse((outside / 'share/rfile/install-receipt.json').exists())

    def test_elevated_install_rejects_other_owner_ancestor(self):
        if os.geteuid() == 0:
            self.skipTest('requires an unprivileged owner to simulate elevated traversal')
        spec = importlib.util.spec_from_file_location('rfile_installer_ancestor_test', self.archive / 'install.py')
        installer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(installer)
        ancestor = self.root / 'owned-by-unprivileged-user'
        ancestor.mkdir(mode=0o755)
        ancestor.chmod(0o755)
        fd = os.open(ancestor, installer.DIR_FLAGS)
        try:
            with mock.patch.object(installer.os, 'geteuid', return_value=0):
                with self.assertRaisesRegex(ValueError, 'unowned prefix ancestor'):
                    installer.safe_ancestor(fd, str(ancestor))
        finally:
            os.close(fd)
