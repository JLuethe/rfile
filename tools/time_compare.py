#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Isolated-process C/Rust date comparisons with identical TZif/configuration."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[1]
ZONES = ['UTC', 'PST8PDT,M3.2.0,M11.1.0', 'America/Los_Angeles',
         'Europe/Berlin', 'Asia/Kolkata', 'Asia/Kathmandu', 'Australia/Lord_Howe',
         'EST5EDT,0/0,J365/25', '<STD>-5:45<DST>-6:15,0/0,J365/25',
         'UTC0', '<FIX>-14', 'AAA12BBB11,M10.1.0,M3.1.0',
         'AAA0BBB,J60/1,J300/3', 'AAA0BBB,60/1,300/3',
         'AAA0BBB,M3.5.0/-2,M10.5.0/26']
KINDS = ['ledate', 'bedate', 'medate', 'leldate', 'beldate', 'meldate',
         'leqdate', 'beqdate', 'leqldate', 'beqldate', 'leqwdate', 'beqwdate']
VALUES = [0, 1, 86399, 86400, 951782400, 1710064799, 1710064800,
          1730624399, 1730624400, 2147483647, 2147483648, 4294967295]
WIDE = [n & ((1 << 64)-1) for n in [-2208988800, -15811200, -86400, -1,
        17999, 18000, 31535999, 31536000, 31553999, 31554000,
        4107542400, 253402300799, 253402300800]]
WINDOWS = [0, 1, 864000000000, 116444736000000000, 116470656000000000,
           125963424000000000, 133643520000000000, 133643519999999999, (1 << 63)-1]

def invoke(binary, source, path, zone):
    result = subprocess.run([str(binary), '-b', '-m', str(source), str(path)],
                            env=dict(os.environ, LC_ALL='C', TZ=zone),
                            capture_output=True, timeout=10)
    return dict(status=result.returncode, stdout_hex=result.stdout.hex(), stderr_hex=result.stderr.hex())

def main():
    pin = json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit']
    if subprocess.check_output(['git', '-C', str(ROOT/'upstream'), 'rev-parse', 'HEAD'], text=True).strip() != pin:
        raise SystemExit('Reference revision drift')
    rows = []
    with tempfile.TemporaryDirectory(prefix='rfile-time-') as directory:
        directory = Path(directory)
        source, path = directory/'magic', directory/'input'
        for kind in KINDS:
            source.write_text(f'0 {kind} x %s\n')
            values = WINDOWS if 'qwdate' in kind else VALUES + (WIDE if 'q' in kind else [])
            for value in values:
                width = 8 if 'q' in kind else 4
                order = 'big' if kind.startswith('be') else 'little'
                data = value.to_bytes(width, order)
                if kind.startswith('me'):
                    data = bytes([data[1], data[0], data[3], data[2]])
                path.write_bytes(data)
                for zone in ZONES:
                    c = invoke(ROOT/'upstream/src/file', source, path, zone)
                    rust = invoke(ROOT/'target/release/rfile', source, path, zone)
                    rows.append(dict(kind=kind, value=value, zone=zone, reference=c, rust=rust, equal=c == rust))
    hashes = {}
    for zone in ZONES:
        path = Path('/usr/share/zoneinfo')/zone
        if path.is_file(): hashes[zone] = hashlib.sha256(path.read_bytes()).hexdigest()
    report = dict(schema_version=1, upstream_commit=pin, environment=dict(LC_ALL='C', libc=platform.libc_ver(), zone_sha256=hashes),
                  total=len(rows), exact=sum(r['equal'] for r in rows), comparisons=rows)
    (ROOT/'reports/time.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'comparisons'}))
    if report['exact'] != report['total']: raise SystemExit('Date mismatch: inspect reports/time.json')
if __name__ == '__main__': main()
