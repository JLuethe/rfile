#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Warm decoder latency plus wait4 RSS; no unfair raw-codec/C-classifier ratio."""
import hashlib,json,os,platform,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    rustc=str(Path.home()/'.cargo/bin/rustc');binary=ROOT/'.reference/codec-bench'
    subprocess.run([rustc,'--edition=2024','-O','tools/codec_bench.rs','--extern','rfile=target/release/librfile.rlib','--extern',next('serde_json='+str(p)for p in (ROOT/'target/release/deps').glob('libserde_json-*.rlib')),'-L','dependency=target/release/deps','-o',str(binary)],cwd=ROOT,check=True)
    log=ROOT/'r5-codec-benchmark.log'
    with log.open('wb')as output:
        child=subprocess.Popen([str(binary)],cwd=ROOT,stdout=output)
        _,status,usage=os.wait4(child.pid,0);child.returncode=os.waitstatus_to_exitcode(status)
    if child.returncode:raise SystemExit('Decoder benchmark failed')
    report=dict(schema_version=1,scope='Warm Rust decoder smoke benchmark, no C ratio or adversarial-latency certification',features='extended-codecs',host=platform.platform(),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),peak_rss_kib=usage.ru_maxrss,measured_cpu_seconds=usage.ru_utime+usage.ru_stime,cases=[json.loads(line)for line in log.read_text().splitlines()])
    (ROOT/'reports/compression-benchmark.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items()if k!='cases'})
if __name__=='__main__':main()
