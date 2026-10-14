#!/usr/bin/env python3
"""Retain compact evidence; large raw authoring records remain explicit externals."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[4]
RAW = Path.home() / '.codex/incant-evidence/core-script-stress-001'
OUT = Path(__file__).resolve().parent

def sha(data): return hashlib.sha256(data).hexdigest()
def file_sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        while block := f.read(1024 * 1024): h.update(block)
    return h.hexdigest()

def main():
    source = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    paths = subprocess.check_output(['git','diff-tree','--no-commit-id','--name-only','-r',source],cwd=ROOT,text=True).splitlines()
    paths += ['samples/core/gameplay.ts','samples/core/bindings.py','samples/core/art/ids.py','tools/native_process.py','sdk/ts/src/native.d.ts']
    sources = [{'path': p, 'sha256': file_sha(ROOT / p)} for p in sorted(set(paths))]
    records=[]; payloads={}
    for path in sorted(RAW.rglob('*')):
        if not path.is_file(): continue
        rel=str(path.relative_to(RAW)); size=path.stat().st_size
        external = ('/authoring/' in '/' + rel and path.name not in ('core.json','stress.ts')) or path.name.endswith(('.rpc.jsonl.gz','.stdout.gz'))
        digest=file_sha(path)
        records.append({'path':rel,'bytes':size,'sha256':digest,'embedded':not external})
        if not external and digest not in payloads: payloads[digest]=base64.b64encode(path.read_bytes()).decode()
    data=(json.dumps({'version':1,'payloads':payloads},separators=(',',':'))+'\n').encode()
    archive=gzip.compress(data,compresslevel=9,mtime=0);(OUT/'artifacts.json.gz').write_bytes(archive)
    manifest={'format':'incant-core-script-stress-evidence','version':1,'source_commit':source,
        'raw_root':str(RAW),'sources':sources,'records':records,'archive_sha256':sha(archive),
        'claims':{'python_tests':10,'native_ticks_per_run':65,'native_cases':2,'native_checks':28,
                  'script_attachments':10000,'original_entities_equal':46,'fast_runtime_executed':False,'performance_measured':False},
        'historical_failures':{'author-det-001':'60s reopen timeout; own authoring292calls passed. Separate180s read-only recovery passed.',
            'author-fast-001':'Root-directed optimization cancelled redundant reauthoring; exact process interrupted/reaped. Not a passing run.'},
        'limits':['Retained deterministic compiler/binary2489325/7b75; not current shipping proof.',
                  'Final fast input is public-authored only; full shipping/performance belongs to a separate root run.',
                  'Raw large journals/RPC and reused assets are external hash-bound records, not embedded payloads.']}
    (OUT/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps({'records':len(records),'payloads':len(payloads),'external':sum(not r['embedded'] for r in records),'archive_bytes':len(archive),'manifest_sha256':file_sha(OUT/'manifest.json')}))

if __name__=='__main__': main()
