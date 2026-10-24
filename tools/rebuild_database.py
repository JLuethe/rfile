#!/usr/bin/env python3
"""Rebuild shipped data using Rust alone, retaining the upstream compiler's path provenance."""
import argparse,hashlib,json,pathlib,shutil,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--write',action='store_true');args=p.parse_args()
with tempfile.TemporaryDirectory(prefix='rfile-data-')as d:
 d=pathlib.Path(d);shutil.copytree(ROOT/'data/magic-source',d/'magic')
 subprocess.run([str(ROOT/'target/release/rfile'),'-C','-m','magic'],cwd=d,check=True)
 compiled=(d/'magic.mgc').read_bytes();expected=json.loads((ROOT/'data/UPSTREAM.json').read_text())['database_sha256'];digest=hashlib.sha256(compiled).hexdigest()
 if digest!=expected:raise SystemExit(f'database hash mismatch: {digest} != {expected}; review source/compiler before updating provenance')
 if args.write:(ROOT/'data/magic.mgc').write_bytes(compiled)
 elif compiled!=(ROOT/'data/magic.mgc').read_bytes():raise SystemExit('shipped data differs from independent Rust compilation')
 print(f'Rust-only rebuild verified: {digest}')
