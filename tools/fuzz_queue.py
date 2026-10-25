#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Bounded local fuzz queue, not an agent/restriction restart service.

Requires prebuilt instrumented targets. Stops dispatch on any failure, source or
binary change, signal or wall deadline. It does not retry failed inputs, modify
code, install packages, publish results, or claim short runs satisfy SEC-04.
"""
import argparse
import datetime
import fcntl
import json
import math
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import tomllib
import uuid
from fuzz_campaign import ROOT,source_digest,digest


def utc():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def save(path,value):
    temporary=path.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(value,indent=2)+'\n');temporary.replace(path)

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--targets',nargs='+')
    ap.add_argument('--workers',type=int,default=2)
    ap.add_argument('--cpu-hours-per-target',type=float,required=True)
    ap.add_argument('--max-wall-hours',type=float,required=True)
    ap.add_argument('--chunk-seconds',type=int,default=300)
    ap.add_argument('--rss-mib',type=int,default=512)
    args=ap.parse_args()
    if not 1<=args.workers<=4 or not 1<=args.chunk_seconds<=3600:ap.error('Invalid worker/chunk limits')
    if not all(math.isfinite(v)and 0<v<=168 for v in [args.cpu_hours_per_target,args.max_wall_hours]):ap.error('Hours must be finite and in (0,168]')
    if not 128<=args.rss_mib<=4096:ap.error('RSS must be 128..4096 MiB per worker')
    inventory=[v['name']for v in tomllib.loads((ROOT/'fuzz/Cargo.toml').read_text())['bin']]
    targets=args.targets or inventory
    if len(set(targets))!=len(targets)or any(t not in inventory for t in targets):ap.error('Unknown or duplicate target')
    if not shutil.which('bwrap'):raise SystemExit('bubblewrap is required')
    binary_root=ROOT/'fuzz/target/x86_64-unknown-linux-gnu/release'
    binaries={t:digest(binary_root/t)for t in targets}
    source=source_digest();supervisor=ROOT/'tools/fuzz_campaign.py';supervisor_hash=digest(supervisor)
    directory=ROOT/'.reference/fuzz-queues';directory.mkdir(parents=True,exist_ok=True)
    lock=(directory/'queue.lock').open('a')
    try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:raise SystemExit('A queue already owns this repository')
    run=directory/str(uuid.uuid4());run.mkdir()
    (run/'previous').mkdir();(run/'results').mkdir()
    # Preserve reports before supervisors begin fresh campaigns. Per-target locks
    # still prevent conflict with an independently launched campaign.
    for target in targets:
        old=ROOT/'reports'/f'fuzz-{target}.json'
        if old.exists():shutil.copy2(old,run/'previous'/old.name)
    rows={t:dict(status='pending',binary_sha256=binaries[t])for t in targets}
    report=dict(schema_version=1,status='starting',queue_pid=os.getpid(),started_utc=utc(),
        source_sha256=source,supervisor_sha256=supervisor_hash,queue_sha256=digest(Path(__file__)),
        git_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        git_status=subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).splitlines(),
        limits=vars(args),targets=rows,run_directory=str(run),qualified_release=False)
    report_path=run/'queue.json';save(report_path,report)
    save(directory/'latest.json',dict(run_directory=str(run),queue_pid=os.getpid(),started_utc=report['started_utc']))
    print(str(report_path),flush=True)
    stopping=None
    def stop(signum,_frame):
        nonlocal stopping
        stopping=f'signal-{signum}'
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    pending=list(targets);active={};start=time.monotonic();stop_at=None
    def harvest(target,process,output):
        status=process.poll()
        if status is None:return False
        output.close();row=rows[target];row.update(returncode=status,finished_utc=utc())
        record=ROOT/'reports'/f'fuzz-{target}.json'
        evidence=None
        if record.exists():
            try:evidence=json.loads(record.read_text())
            except (OSError,ValueError):pass
        # Never credit an old report when a process failed before writing one.
        owned=(evidence is not None and evidence.get('supervisor_pid')==process.pid
               and evidence.get('started_utc','')>=report['started_utc']
               and evidence.get('configuration',{}).get('source_sha256')==source
               and evidence.get('configuration',{}).get('binary_sha256')==binaries[target])
        if owned:
            shutil.copy2(record,run/'results'/record.name)
            row.update(measured_cpu_seconds=evidence.get('measured_cpu_seconds',0),
                       campaign_status=evidence.get('status'),completed_gate=evidence.get('completed_gate',False),
                       report_sha256=digest(record))
        row['status']='complete'if status==0 and owned and evidence.get('status')in ['partial-campaign-complete','gate-passed']else 'failed'
        return True
    try:
        while pending or active:
            for target,(process,output)in list(active.items()):
                if harvest(target,process,output):
                    del active[target]
                    if rows[target]['status']=='failed'and stopping is None:stopping='target-failed:'+target
            if time.monotonic()-start>=args.max_wall_hours*3600 and stopping is None:stopping='wall-deadline'
            if stopping is None and (source_digest()!=source or digest(supervisor)!=supervisor_hash):stopping='candidate-or-supervisor-changed'
            if stopping is not None:
                if stop_at is None:
                    stop_at=time.monotonic()
                    for process,_ in active.values():process.terminate()
                elif time.monotonic()-stop_at>10:
                    for process,_ in active.values():process.kill()
                for target in pending:rows[target]['status']='not-started'
                pending.clear()
                report['status']='stopping';report['stop_reason']=stopping
            else:
                while pending and len(active)<args.workers:
                    target=pending[0]
                    if digest(binary_root/target)!=binaries[target]:
                        stopping='binary-changed:'+target;break
                    pending.pop(0);output=(run/(target+'.log')).open('wb')
                    command=[sys.executable,str(supervisor),target,'--seconds',str(args.chunk_seconds),
                        '--cpu-hours',str(args.cpu_hours_per_target),'--rss-mib',str(args.rss_mib)]
                    process=subprocess.Popen(command,cwd=ROOT,stdin=subprocess.DEVNULL,stdout=output,stderr=subprocess.STDOUT,start_new_session=True)
                    active[target]=(process,output)
                    rows[target].update(status='running',pid=process.pid,started_utc=utc(),command=command)
                report['status']='running'
            report['heartbeat_utc']=utc();report['wall_seconds']=time.monotonic()-start;save(report_path,report)
            if pending or active:time.sleep(1)
    finally:
        # Handles unexpected supervisor exceptions too, leaving no orphan worker.
        for process,_ in active.values():
            if process.poll()is None:process.terminate()
        for target,(process,output)in active.items():
            try:process.wait(timeout=10)
            except subprocess.TimeoutExpired:process.kill();process.wait()
            harvest(target,process,output)
        report['status']='complete'if not stopping and all(r['status']=='complete'for r in rows.values())else 'stopped'
        report['finished_utc']=utc();report['wall_seconds']=time.monotonic()-start
        report['measured_cpu_seconds']=sum(r.get('measured_cpu_seconds',0)for r in rows.values())
        report['sustained_targets_passed']=[t for t,r in rows.items()if r.get('completed_gate')]
        save(report_path,report)
    if report['status']!='complete':raise SystemExit(1)
if __name__=='__main__':main()
