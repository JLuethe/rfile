#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Replay exact research vectors; never hide changed errors behind a known-gap label."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
RESEARCH = json.loads((ROOT / 'reports/remaining-spec-observations.json').read_text())
ENV = dict(os.environ, LC_ALL='C', TZ='UTC')
ENV.pop('POSIXLY_CORRECT', None)
PIN = json.loads((ROOT / 'data/UPSTREAM.json').read_text())['commit']


def invoke(binary, database, path, flags):
    command = [str(binary), '-b', '-m', str(database), *flags, str(path)]
    result = subprocess.run(command, env=ENV, capture_output=True, timeout=10)
    return dict(status=result.returncode, stdout_hex=result.stdout.hex(),
                stderr_hex=result.stderr.replace(str(path.parent).encode(), b'$FIXTURE').hex())


def main():
    actual_pin = subprocess.check_output(
        ['git', '-C', str(ROOT / 'upstream'), 'rev-parse', 'HEAD'], text=True).strip()
    if actual_pin != PIN:
        raise SystemExit('Reference revision drift')
    rows = []
    with tempfile.TemporaryDirectory(prefix='rfile-components-') as directory:
        directory = Path(directory)
        cdf = RESEARCH['cdf_summary']
        fixture = bytes.fromhex(cdf['fixture_hex'])
        assert hashlib.sha256(fixture).hexdigest() == cdf['sha256']
        cases = []
        for index, flags in enumerate([[], ['-i']]):
            observations = [o for o in cdf['observations'] if o['flags'] == flags]
            cases.append((f'cdf-summary-{index}', fixture, None, flags, [
                dict(status=o['status'], stdout_hex=o['stdout'].encode().hex(),
                     stderr_hex=o['stderr'].encode().hex()) for o in observations]))
        for case in RESEARCH['regex']:
            cases.append(('regex-' + case['name'], bytes.fromhex(case['input_hex']),
                          bytes.fromhex(case['source_hex']), [], case['observations']))
        for name, data, source, flags, baseline in cases:
            path = directory / (name + '.input')
            path.write_bytes(data)
            database = ROOT / 'data/magic.mgc'
            if source is not None:
                database = directory / (name + '.magic')
                database.write_bytes(source)
            reference = invoke(ROOT / 'upstream/src/file', database, path, flags)
            rust = invoke(ROOT / 'target/release/rfile', database, path, flags)
            expected_reference = {k: baseline[0][k] for k in reference}
            expected_rust = {k: baseline[1][k] for k in rust}
            reference_valid = reference == expected_reference
            equal = reference == rust
            known = (not equal and reference_valid and rust == expected_rust
                     and name in {'cdf-summary-0', 'regex-backreference'})
            explicit_unsupported = (reference_valid and name == 'regex-backreference' and rust == dict(
                status=1, stdout_hex='', stderr_hex=(
                    'rfile: magic line 1: $FIXTURE/regex-backreference.magic: magic line 1: '
                    'regex backreference at byte 4 requires the bounded capture VM\n').encode().hex()))
            disposition = ('reference-drift'  if not reference_valid else
                           'exact' if equal else 'explicit-unsupported' if explicit_unsupported else 'known-open' if known else
                           'unclassified-disagreement')
            rows.append(dict(case=name, input_sha256=hashlib.sha256(data).hexdigest(),
                             flags=flags, reference=reference, rust=rust,
                             equal=equal, disposition=disposition))
    report = dict(schema_version=1, upstream_commit=PIN,
                  rust_commit=subprocess.check_output(
                      ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  environment={'LC_ALL': 'C', 'TZ': 'UTC'}, comparisons=rows,
                  total=len(rows), exact=sum(r['equal'] for r in rows),
                  known_open=sum(r['disposition'] == 'known-open' for r in rows),
                  explicit_unsupported=sum(r['disposition'] == 'explicit-unsupported' for r in rows),
                  unexpected=sum(r['disposition'] not in {'exact', 'known-open', 'explicit-unsupported'} for r in rows))
    (ROOT / 'reports/components.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'comparisons'}))
    if report['unexpected']:
        raise SystemExit('Component mismatch changed; inspect reports/components.json')


if __name__ == '__main__':
    main()
