#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Supervise sanitizer fuzz chunks and measure CPU time; smoke is never 24h.

Build the target with cargo +nightly fuzz build first. For a final-candidate
campaign use --cpu-hours 24. Resume requires exactly the same instrumented
binary, source tree, lockfiles and initial configuration. Run one supervisor
per target; each owns its child and wait4 accounting.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time
ROOT=Path(__file__).resolve().parents[1]

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def source_digest():
    paths=[*ROOT.glob('crates/**/*.rs'),*ROOT.glob('crates/**/Cargo.toml'),*ROOT.glob('src/**/*.rs'),*ROOT.glob('fuzz/fuzz_targets/*.rs'),*ROOT.glob('fuzz/fuzz_targets/*.inc'),ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'fuzz/Cargo.toml',ROOT/'fuzz/Cargo.lock']
    h=hashlib.sha256()
    for path in sorted(paths):
        h.update(str(path.relative_to(ROOT)).encode()+b'\0'+bytes.fromhex(digest(path)))
    return h.hexdigest()
def seeds(directory):
    paths=sorted(p for p in directory.iterdir()if p.is_file())
    h=hashlib.sha256()
    for path in paths:h.update(path.name.encode()+b'\0'+bytes.fromhex(digest(path)))
    return dict(files=len(paths),bytes=sum(p.stat().st_size for p in paths),sha256=h.hexdigest())
def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('target');ap.add_argument('--seconds',type=int,default=300)
    ap.add_argument('--cpu-hours',type=float,default=0)
    ap.add_argument('--seed',type=int,default=548)
    ap.add_argument('--max-len',type=int,default=65536)
    ap.add_argument('--rss-mib',type=int,default=512)
    ap.add_argument('--resume',action='store_true')
    args=ap.parse_args()
    if not re.fullmatch(r'[a-z_]+',args.target):raise SystemExit('Invalid target')
    if not 1<=args.seconds<=3600 or args.cpu_hours<0:raise SystemExit('Invalid campaign duration')
    binary=ROOT/'fuzz/target/x86_64-unknown-linux-gnu/release'/args.target
    corpus=ROOT/'fuzz/corpus'/args.target;corpus.mkdir(parents=True,exist_ok=True)
    artifacts=ROOT/'fuzz/artifacts'/args.target;artifacts.mkdir(parents=True,exist_ok=True)
    logs=ROOT/'fuzz/campaign-logs'/args.target;logs.mkdir(parents=True,exist_ok=True)
    record=ROOT/'reports'/f'fuzz-{args.target}.json'
    configuration=dict(target=args.target,binary_sha256=digest(binary),source_sha256=source_digest(),
                       seed=args.seed,max_len=args.max_len,rss_mib=args.rss_mib,sanitizer='address',
                       build='cargo +nightly fuzz build; default optimized debug assertions + instrumented std',
                       upstream_commit=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'])
    if args.resume:
        report=json.loads(record.read_text())
        if report['configuration']!=configuration:raise SystemExit('Candidate/config changed; start a fresh campaign')
    else:
        report=dict(schema_version=1,configuration=configuration,initial_corpus=seeds(corpus),chunks=[],
                    measured_cpu_seconds=0.0,wall_seconds=0.0,completed_gate=False)
    target_cpu=args.cpu_hours*3600
    while True:
        index=len(report['chunks'])
        log=logs/f'chunk-{index}.log'
        command=[str(binary),str(corpus),f'-seed={args.seed+index}',f'-max_total_time={args.seconds}',
                 f'-max_len={args.max_len}','-timeout=3',f'-rss_limit_mb={args.rss_mib}',
                 '-print_final_stats=1',f'-artifact_prefix={artifacts}/']
        started=time.monotonic();killed=False
        with log.open('wb')as output:
            child=subprocess.Popen(command,stdout=output,stderr=subprocess.STDOUT,cwd=ROOT,
                                   env=dict(os.environ,RUST_BACKTRACE='1'),start_new_session=True)
            while True:
                pid,status,usage=os.wait4(child.pid,os.WNOHANG)
                if pid:
                    child.returncode=os.waitstatus_to_exitcode(status);break
                elapsed=time.monotonic()-started
                if elapsed>args.seconds+30:
                    os.killpg(child.pid,signal.SIGKILL);killed=True
                time.sleep(0.2)
        wall=time.monotonic()-started;cpu=usage.ru_utime+usage.ru_stime
        data=log.read_text(errors='replace')
        stats={key:int(value)for key,value in re.findall(r'stat::([a-z_]+):\s*(\d+)',data)}
        chunk=dict(index=index,returncode=child.returncode,supervisor_kill=killed,cpu_seconds=cpu,
                   wall_seconds=wall,peak_rss_kib=usage.ru_maxrss,stats=stats,
                   log=str(log.relative_to(ROOT)),log_sha256=digest(log))
        report['chunks'].append(chunk);report['measured_cpu_seconds']+=cpu;report['wall_seconds']+=wall
        report['current_corpus']=seeds(corpus)
        report['requested_cpu_hours']=args.cpu_hours
        report['completed_gate']=bool(target_cpu>=86400 and report['measured_cpu_seconds']>=target_cpu and all(c['returncode']==0 for c in report['chunks']))
        report['status']='failed' if child.returncode else 'gate-passed' if report['completed_gate'] else 'smoke-only' if not target_cpu else 'running'
        temporary=record.with_suffix('.json.tmp');temporary.write_text(json.dumps(report,indent=2)+'\n');temporary.replace(record)
        print(json.dumps(dict(target=args.target,**chunk,accumulated_cpu_seconds=report['measured_cpu_seconds'],status=report['status'])),flush=True)
        if child.returncode:raise SystemExit('Fuzzer failed; preserve log/artifact and add a minimized regression')
        if not target_cpu or report['measured_cpu_seconds']>=target_cpu:break
        if source_digest()!=configuration['source_sha256'] or digest(binary)!=configuration['binary_sha256']:
            raise SystemExit('Candidate changed; campaign stopped before mixing revisions')
if __name__=='__main__':main()
