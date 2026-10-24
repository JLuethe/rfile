#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Optional second-reader check. Requires olefile in an isolated Python env."""
import json
from pathlib import Path
import olefile
ROOT=Path(__file__).resolve().parents[1]
rows=[]
for name in ['summary-v3-mini','summary-v3-regular','summary-v4-mini','summary-v4-regular','all-properties']:
    path=ROOT/'tests/fixtures/cdf'/f'{name}.cfb'
    try:
        with olefile.OleFileIO(path,raise_defects=olefile.DEFECT_INCORRECT) as reader:
            properties=reader.getproperties('\x05SummaryInformation')
            rows.append(dict(case=name,status='parsed',values=repr(properties),simple_expected=properties.get(1)==1252 and properties.get(18)==b'Microsoft Word'))
    except Exception as error:
        rows.append(dict(case=name,status='reader-error',error=type(error).__name__+': '+str(error)))
report=dict(reader='olefile',version=olefile.__version__,observations=rows,
            caveat='This reader fails its LPWSTR-vector path in the all-properties fixture; it is not independent proof for that vector. Microsoft MS-OLEPS wire layouts and typed vector tests are separate evidence.')
(ROOT/'reports/cdf-independent.json').write_text(json.dumps(report,indent=2)+'\n')
if not all(row.get('simple_expected') for row in rows[:4]):raise SystemExit('Independent scalar check failed')
print('Four scalar documents independently parsed; vector-reader limitation recorded')
