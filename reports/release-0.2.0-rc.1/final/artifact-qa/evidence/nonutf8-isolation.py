#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path
base=Path('/home/john/Code/rfile/.reference/package-qa-e64')
bin_path=base/'binary-extracted/rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu/bin/rfile'
fixture_dir=base/'fixtures'
raw_name=b'name-\xff.txt'
raw_path=os.fsencode(fixture_dir)+b'/'+raw_name
if not os.path.isfile(raw_path):
    fd=os.open(raw_path,os.O_CREAT|os.O_WRONLY|os.O_TRUNC,0o644); os.write(fd,b'hello artifact\n'); os.close(fd)
# Direct raw byte argv to exact packaged executable, without OCI transport.
direct_cmd=[os.fsencode(bin_path),b'-b',raw_path]
direct=subprocess.run(direct_cmd,capture_output=True,timeout=30,check=False)
# Pass only ASCII command text through OCI argv; shell glob obtains raw bytes from directory entries.
container_cmd=['podman','run','--rm','--network=none','--read-only','--cap-drop=ALL','--security-opt=no-new-privileges','--user=65534:65534','--memory=256m','--pids-limit=32','--cpus=1','--env','TZ=UTC0','--volume',f'{bin_path}:/qa/rfile:ro','--volume',f'{fixture_dir}:/fixtures:ro','docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11','/bin/sh','-c','for path in /fixtures/name-*.txt; do exec /qa/rfile -b "$path"; done; exit 2']
container=subprocess.run(container_cmd,capture_output=True,timeout=30,text=True,check=False)
result={'raw_filename_hex':raw_name.hex(),'direct_host':{'status':direct.returncode,'stdout':direct.stdout.decode('utf-8','backslashreplace'),'stderr':direct.stderr.decode('utf-8','backslashreplace'),'argv_hex':[a.hex() for a in direct_cmd]},'container_shell_glob':{'status':container.returncode,'stdout':container.stdout,'stderr':container.stderr,'command':container_cmd},'expected_stdout':'ASCII text\\n','status':'passed' if direct.returncode==0 and direct.stdout==b'ASCII text\n' and container.returncode==0 and container.stdout=='ASCII text\n' else 'failed'}
(base/'evidence/nonutf8-isolation.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
sys.exit(result['status']!='passed')
