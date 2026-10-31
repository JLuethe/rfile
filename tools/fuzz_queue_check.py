#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise real queue locking, partial completion and bounded interruption.
Requires built source_diagnostics, abi_state and encoding sanitizer targets.
Existing manifests are archived by the queue; this does not run sustained gates.
"""
import json
from pathlib import Path
import subprocess
import sys
import time
from fuzz_campaign import ROOT

def wait_for(check,seconds=15):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        try:
            value=check()
            if value:return value
        except (FileNotFoundError,KeyError,json.JSONDecodeError):pass
        time.sleep(.1)
    raise AssertionError('Expected queue state did not arrive')

def main():
    command=[sys.executable,str(ROOT/'tools/fuzz_queue.py'),'--targets','source_diagnostics','abi_state',
        '--workers','2','--cpu-hours-per-target','0.0001','--max-wall-hours','0.02','--chunk-seconds','10']
    latest=ROOT/'.reference/fuzz-queues/latest.json'
    with(ROOT/'reports/r9-queue-selftest.log').open('wb')as log:
        child=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,cwd=ROOT)
        try:
            index=wait_for(lambda:(r if (r:=json.loads(latest.read_text()))['queue_pid']==child.pid else None))
            path=Path(index['run_directory'])/'queue.json'
            def running():
                value=json.loads(path.read_text())
                return value if all(v['status']=='running'for v in value['targets'].values())else None
            report=wait_for(running)
            expected=report['targets']['source_diagnostics']['pid']
            wait_for(lambda:(r if (r:=json.loads((ROOT/'reports/fuzz-source_diagnostics.json').read_text())).get('supervisor_pid')==expected and r['status']=='running'else None))
            duplicate=subprocess.run(command,cwd=ROOT,capture_output=True,text=True,timeout=15)
            target_duplicate=subprocess.run([sys.executable,str(ROOT/'tools/fuzz_campaign.py'),'source_diagnostics','--seconds','1'],cwd=ROOT,capture_output=True,text=True,timeout=10)
            status=child.wait(timeout=90)
        finally:
            if child.poll()is None:
                child.terminate();child.wait(timeout=15)
    report=json.loads(path.read_text())
    assert status==0 and report['status']=='complete',report
    assert duplicate.returncode!=0 and 'queue already owns'in duplicate.stderr,duplicate
    assert target_duplicate.returncode!=0 and 'Another campaign owns'in target_duplicate.stderr,target_duplicate
    assert all(v['campaign_status']=='partial-campaign-complete'and not v['completed_gate']for v in report['targets'].values()),report
    result=dict(schema_version=1,normal_queue=str(path),queue_status=report['status'],duplicate_queue_rejected=True,
        duplicate_target_rejected=True,partial_not_qualified=True,measured_cpu_seconds=report['measured_cpu_seconds'])
    deadline=subprocess.run([sys.executable,str(ROOT/'tools/fuzz_queue.py'),'--targets','encoding','--workers','1',
        '--cpu-hours-per-target','1','--max-wall-hours','0.0003','--chunk-seconds','10'],cwd=ROOT,capture_output=True,text=True,timeout=30)
    run=Path(json.loads(latest.read_text())['run_directory']);report=json.loads((run/'queue.json').read_text())
    assert deadline.returncode==1 and report['status']=='stopped'and report['stop_reason']=='wall-deadline',report
    for row in report['targets'].values():
        if 'pid'in row:assert not Path('/proc',str(row['pid'])).exists(),row
    result.update(deadline_queue=str(run/'queue.json'),deadline_stopped=True,supervisor_process_exited=True)
    (ROOT/'reports/fuzz-queue-selftest.json').write_text(json.dumps(result,indent=2)+'\n');print(result)
if __name__=='__main__':main()
