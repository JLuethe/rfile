#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path
base=Path('/home/john/Code/rfile/.reference/package-qa-e64')
binary=os.fsencode(base/'binary-extracted/rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu/bin/rfile')
fixtures=os.fsencode(base/'fixtures')
image=b'docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11'
cmd=[b'podman',b'run',b'--rm',b'--network=none',b'--read-only',b'--cap-drop=ALL',b'--security-opt=no-new-privileges',b'--user=65534:65534',b'--memory=256m',b'--pids-limit=32',b'--cpus=1',b'--env',b'TZ=UTC0',b'--volume',binary+b':/qa/rfile:ro',b'--volume',fixtures+b':/fixtures:ro',image,b'/qa/rfile',b'-b',b'/fixtures/name-\xff.txt']
p=subprocess.run(cmd,capture_output=True,timeout=30,check=False)
record={'status':p.returncode,'stdout':p.stdout.decode('utf-8','backslashreplace'),'stderr':p.stderr.decode('utf-8','backslashreplace'),'filename_bytes_hex':b'/fixtures/name-\xff.txt'.hex(),'command_argv_bytes_hex':[x.hex() for x in cmd]}
(base/'evidence/nonutf8-filename-check.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record))
sys.exit(0 if p.returncode==0 and p.stdout==b'ASCII text\n' else 1)
