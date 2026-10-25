#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Supervise sanitizer fuzz chunks and measure CPU time; smoke is never 24h.

Build the target with cargo +nightly fuzz build first. For a final-candidate
campaign use --cpu-hours 24. Resume requires exactly the same instrumented
binary, source tree, lockfiles and initial configuration. Run one supervisor
per target; each owns its child and wait4 accounting.
"""
import argparse
import fcntl
import math
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time
import datetime
import platform
import shutil
import uuid
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
    if not 1<=args.seconds<=3600 or not math.isfinite(args.cpu_hours) or not 0<=args.cpu_hours<=168:raise SystemExit('Invalid campaign duration')
    if not 1<=args.max_len<=16*1024*1024 or not 128<=args.rss_mib<=4096:raise SystemExit('Invalid input/RSS limit')
    binary=ROOT/'fuzz/target/x86_64-unknown-linux-gnu/release'/args.target
    corpus=ROOT/'fuzz/corpus'/args.target;corpus.mkdir(parents=True,exist_ok=True)
    artifacts=ROOT/'fuzz/artifacts'/args.target;artifacts.mkdir(parents=True,exist_ok=True)
    logs_root=ROOT/'fuzz/campaign-logs'/args.target;logs_root.mkdir(parents=True,exist_ok=True)
    record=ROOT/'reports'/f'fuzz-{args.target}.json'
    # Keep this file descriptor open throughout the run, including final save.
    lock=(logs_root/'campaign.lock').open('a')
    try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError:raise SystemExit('Another campaign owns this target')
    configuration=dict(supervisor_sha256=digest(Path(__file__)),isolation='bubblewrap unshare-all; no host home/network; private tmp; readonly binary/system libraries',kernel=platform.platform(),toolchain=subprocess.check_output([str(Path.home()/'.cargo/bin/rustc'),'+nightly','--version'],text=True).strip(),target=args.target,binary_sha256=digest(binary),source_sha256=source_digest(),
                       seed=args.seed,max_len=args.max_len,rss_mib=args.rss_mib,sanitizer='address',
                       build='cargo +nightly fuzz build; default optimized debug assertions + instrumented std',
                       upstream_commit=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'])
    if args.resume:
        report=json.loads(record.read_text())
        if report['configuration']!=configuration:raise SystemExit('Candidate/config changed; start a fresh campaign')
    else:
        report=dict(schema_version=2,started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),supervisor_pid=os.getpid(),configuration=configuration,initial_corpus=seeds(corpus),chunks=[],
                    measured_cpu_seconds=0.0,wall_seconds=0.0,completed_gate=False)
    sandbox=shutil.which('bwrap')
    if not sandbox:raise SystemExit('bubblewrap is required; no unsandboxed campaign fallback')
    if args.resume:logs=ROOT/report['log_directory']
    else:
        logs=logs_root/str(uuid.uuid4());logs.mkdir();report['log_directory']=str(logs.relative_to(ROOT))
    target_cpu=args.cpu_hours*3600
    def save():
        temporary=record.with_suffix('.json.tmp');temporary.write_text(json.dumps(report,indent=2)+'\n');temporary.replace(record)
    report['status']='starting';save()
    while True:
        index=len(report['chunks'])
        log=logs/f'chunk-{index}.log'
        command=[str(binary),str(corpus),f'-seed={args.seed+index}',f'-max_total_time={args.seconds}',
                 f'-max_len={args.max_len}','-timeout=3',f'-rss_limit_mb={args.rss_mib}',
                 '-print_final_stats=1',f'-artifact_prefix={artifacts}/']
        command=[sandbox,'--unshare-all','--die-with-parent','--new-session',
            '--ro-bind','/usr','/usr','--symlink','usr/lib','/lib','--symlink','usr/lib','/lib64',
            '--proc','/proc','--dev','/dev','--tmpfs','/tmp',
            '--ro-bind',str(binary),str(binary),'--bind',str(corpus),str(corpus),
            '--bind',str(artifacts),str(artifacts),'--chdir','/tmp','--clearenv',
            '--setenv','PATH','/usr/bin','--setenv','HOME','/tmp','--setenv','LC_ALL','C',
            '--setenv','TZ','UTC','--setenv','RUST_BACKTRACE','1',
            '/usr/bin/python3','-I','-c',
            'import os,subprocess,sys; p=subprocess.Popen(sys.argv[1:]); _,s,u=os.wait4(p.pid,0); p.returncode=os.waitstatus_to_exitcode(s); print("RFILE_FUZZ_USAGE",u.ru_utime,u.ru_stime,u.ru_maxrss,p.returncode,file=sys.stderr,flush=True); sys.exit(p.returncode if p.returncode>=0 else 128-p.returncode)',*command]
        started=time.monotonic();killed=False
        with log.open('wb')as output:
            child=subprocess.Popen(command,stdout=output,stderr=subprocess.STDOUT,cwd=ROOT,
                                   env=dict(os.environ,RUST_BACKTRACE='1'),start_new_session=True,preexec_fn=lambda:os.nice(10))
            report['status']='running';report['active_child_pid']=child.pid;report['active_command']=command;save()
            while True:
                pid,status,usage=os.wait4(child.pid,os.WNOHANG)
                if pid:
                    child.returncode=os.waitstatus_to_exitcode(status);break
                elapsed=time.monotonic()-started
                if elapsed>args.seconds+30:
                    os.killpg(child.pid,signal.SIGKILL);killed=True
                time.sleep(0.2)
        wall=time.monotonic()-started
        data=log.read_text(errors='replace')
        measured=re.findall(r'^RFILE_FUZZ_USAGE ([0-9.]+) ([0-9.]+) ([0-9]+) (-?[0-9]+)$',data,re.M)
        cpu=sum(map(float,measured[0][:2]))if len(measured)==1 else 0.0
        peak_rss=int(measured[0][2])if len(measured)==1 else None
        accounting_valid=len(measured)==1
        stats={key:int(value)for key,value in re.findall(r'stat::([a-z_]+):\s*(\d+)',data)}
        chunk=dict(index=index,returncode=child.returncode,supervisor_kill=killed,cpu_seconds=cpu,
                   wall_seconds=wall,peak_rss_kib=peak_rss,stats=stats,accounting_valid=accounting_valid,fuzzer_exit_status=int(measured[0][3])if accounting_valid else None,supervisor_cpu_seconds=usage.ru_utime+usage.ru_stime,
                   log=str(log.relative_to(ROOT)),log_sha256=digest(log),command=command,finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
        report['chunks'].append(chunk);report['measured_cpu_seconds']+=cpu;report['wall_seconds']+=wall
        report['current_corpus']=seeds(corpus)
        report['requested_cpu_hours']=args.cpu_hours
        report['completed_gate']=bool(target_cpu>=86400 and report['measured_cpu_seconds']>=target_cpu and all(c['returncode']==0 and c['accounting_valid'] for c in report['chunks']))
        report['status']='failed' if child.returncode or not accounting_valid else 'gate-passed' if report['completed_gate'] else 'smoke-only' if not target_cpu else 'partial-campaign-complete' if report['measured_cpu_seconds']>=target_cpu else 'running'
        report.pop('active_child_pid',None);report.pop('active_command',None);save()
        print(json.dumps(dict(target=args.target,**chunk,accumulated_cpu_seconds=report['measured_cpu_seconds'],status=report['status'])),flush=True)
        if child.returncode or not accounting_valid:raise SystemExit('Fuzzer failed; preserve log/artifact and add a minimized regression')
        if not target_cpu or report['measured_cpu_seconds']>=target_cpu:break
        if source_digest()!=configuration['source_sha256'] or digest(binary)!=configuration['binary_sha256']:
            report['status']='stopped-candidate-changed';save()
            raise SystemExit('Candidate changed; campaign stopped before mixing revisions')
if __name__=='__main__':main()
