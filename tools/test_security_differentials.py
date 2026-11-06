"""Security-only oracle exceptions must reject unrelated output drift."""
import unittest

from cli_database_compare import security_escape_only as database_escape_only
from locale_compare import security_escape_only as locale_escape_only


class SecurityDifferentialTests(unittest.TestCase):
    def test_locale_escape_requires_exact_output_and_status(self):
        reference = dict(status=1, stdout_hex=b"ERROR: missing-\x1b[31m\n".hex(), stderr_hex="")
        rust = dict(status=1, stdout_hex=b"ERROR: missing-\\u{1b}[31m\n".hex(), stderr_hex="")
        row = dict(reference=reference, rust=rust)
        self.assertTrue(locale_escape_only(row, [b"missing-\x1b"]))
        self.assertFalse(locale_escape_only(dict(reference=reference, rust={**rust, "status": 0}), [b"missing-\x1b"]))
        self.assertFalse(locale_escape_only(dict(reference=reference, rust={**rust, "stdout_hex": b"wrong\n".hex()}), [b"missing-\x1b"]))

    def test_database_escape_requires_exact_channels_and_artifacts(self):
        reference = dict(status=1, stdout_hex="", stderr_hex=b"bad\r' invalid\n".hex(), artifacts={"probe.mgc": "hash"})
        rust = {**reference, "stderr_hex": b"bad\\u{d}' invalid\n".hex()}
        self.assertFalse(database_escape_only("other", "c", reference, [rust]))
        reference["stderr_hex"] = b"bogus x bad\r' invalid\n".hex()
        rust["stderr_hex"] = b"bogus x bad\\u{d}' invalid\n".hex()
        self.assertTrue(database_escape_only("crlf", "c", reference, [rust]))
        self.assertFalse(database_escape_only("crlf", "c", reference, [{**rust, "artifacts": {}}]))


if __name__ == "__main__":
    unittest.main()
