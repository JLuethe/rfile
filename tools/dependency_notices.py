#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Refresh resolved dependency metadata and retain upstream license/notice files."""
import json
import os
from pathlib import Path
import shutil
import subprocess
ROOT = Path(__file__).resolve().parents[1]
CARGO = os.environ.get('CARGO', shutil.which('cargo') or str(Path.home()/'.cargo/bin/cargo'))
metadata = json.loads(subprocess.check_output([CARGO, 'metadata', '--locked', '--all-features', '--format-version', '1'], cwd=ROOT))
packages = [p for p in metadata['packages'] if p['source'] is not None]
rows = []
for p in sorted(packages, key=lambda p: p['name']):
    rows.append({key: p.get(key) for key in ['name', 'version', 'license', 'license_file', 'repository', 'source']})
    source = Path(p['manifest_path']).parent
    destination = ROOT/'LICENSES/dependencies'/f"{p['name']}-{p['version']}"
    files = [f for f in source.iterdir() if f.is_file() and f.name.upper().startswith(('LICENSE', 'LICENCE', 'UNLICENSE', 'COPYING', 'NOTICE'))]
    if p['license_file']:
        files.append(source/p['license_file'])
    if not files:
        raise SystemExit(f"No license text found for {p['name']}")
    destination.mkdir(exist_ok=True)
    for f in files:
        shutil.copyfile(f, destination/f.name)
(ROOT/'reports/dependencies.json').write_text(json.dumps(rows, indent=2)+'\n')
print(f'Retained license texts for {len(rows)} resolved dependencies')
