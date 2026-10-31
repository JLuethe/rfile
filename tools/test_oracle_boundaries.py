# SPDX-License-Identifier: GPL-3.0-only
import importlib.util
import json
import os
from pathlib import Path
import unittest

BOUNDARY_PATH = Path(os.environ.get('RFILE_BOUNDARY_MODULE') or Path(__file__).resolve().with_name('oracle_boundaries.py'))
REPORT_DIR = Path(os.environ['RFILE_ORACLE_REPORTS']) if 'RFILE_ORACLE_REPORTS' in os.environ else None
spec = importlib.util.spec_from_file_location('oracle_boundaries_under_test', BOUNDARY_PATH)
boundary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(boundary)

MIN = 2_648_708_640_000_000_000  # independently calculated from pinned C source
I64_MAX = (1 << 63) - 1
U64_MAX = (1 << 64) - 1


class OracleBoundaryTests(unittest.TestCase):
    def test_threshold_is_inclusive_and_signed_upper_bound_is_exclusive(self):
        check = boundary.filetime_has_uninitialized_seconds
        self.assertFalse(check(MIN - 1))
        self.assertTrue(check(MIN))
        self.assertTrue(check(MIN + 1))
        self.assertTrue(check(I64_MAX))
        self.assertFalse(check(1 << 63))
        self.assertFalse(check(U64_MAX))

    def test_negative_signed_patterns_and_defined_ordinary_dates(self):
        check = boundary.filetime_has_uninitialized_seconds
        for bits in (1 << 63, U64_MAX, U64_MAX - 1):
            self.assertFalse(check(bits), hex(bits))
        for ticks in (0, 1, 86400 * 10_000_000, 116444736000000000, MIN - 1):
            self.assertFalse(check(ticks), str(ticks))

    def test_invalid_unsigned_domain_is_rejected(self):
        check = boundary.filetime_has_uninitialized_seconds
        for value in (-1, 1 << 64):
            with self.assertRaises(ValueError):
                check(value)

    @unittest.skipUnless(REPORT_DIR is not None, 'set RFILE_ORACLE_REPORTS to run report-contract checks')
    def test_time_report_keeps_raw_undefined_evidence_and_checks_rust_marker(self):
        report = json.loads((REPORT_DIR / 'time.json').read_text())
        rows = [r for r in report['comparisons'] if r.get('oracle_defined') is False]
        self.assertGreaterEqual(len(rows), 30)
        for row in rows:
            self.assertIn('reference', row)
            self.assertIn('stdout_hex', row['reference'])
            self.assertFalse(row['equal'])
            self.assertEqual(row['disposition'], 'native-filetime-uninitialized-seconds')
            self.assertEqual(row['rust'], {
                'status': 0,
                'stdout_hex': b'*Invalid datetime*\n'.hex(),
                'stderr_hex': '',
            })
            self.assertTrue(row['safe_result_validated'])
        self.assertEqual(report['unexpected'], report['total'] - report['exact'] - report['oracle_undefined_validated'])
        self.assertEqual(report['unexpected'], 0)

    @unittest.skipUnless(REPORT_DIR is not None, 'set RFILE_ORACLE_REPORTS to run report-contract checks')
    def test_semantics_short_input_fast_path_and_zero_padded_native_width(self):
        report = json.loads((REPORT_DIR / 'semantics.json').read_text())
        rows = [r for r in report['cases'] if r.get('case') == 'beqwdate']
        one_byte = [r for r in rows if len(bytes.fromhex(r['input_hex'])) == 1]
        two_byte = [r for r in rows if r['input_hex'] == '4142']
        self.assertTrue(one_byte)
        self.assertTrue(two_byte)
        self.assertTrue(all(r.get('oracle_defined') is not False for r in one_byte))
        # file_buffer bypasses softmagic only for 0/1 bytes. For 2+ bytes,
        # mget zero-pads the native 8-byte value before FILETIME conversion.
        self.assertTrue(all(r.get('oracle_defined') is False for r in two_byte))
        self.assertTrue(all(r['safe_result_validated'] for r in two_byte))


if __name__ == '__main__':
    unittest.main(verbosity=2)
