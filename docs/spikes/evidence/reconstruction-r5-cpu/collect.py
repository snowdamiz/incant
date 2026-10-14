#!/usr/bin/env python3
"""Verify the scoped R5 CPU/cooker evidence; does not build or run a GPU."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[3]
EXTERNAL=Path('/Users/andreyyurlov/.codex/incant-evidence/reconstruction-r5-cpu-001')
BASE='825ff047284e84252d00f786a7b14cb6ecb44293'
CLAUDE='268f3b57228ab7bef47511103d0c97c2af057d56'
SOURCE='8c39fd6adbf95d470388867de4d305756b05913c'
ROOTS=['crates','third_party','tools','.cargo','editor/app','sdk/rust','Cargo.lock','Cargo.toml','rust-toolchain.toml']
FREEZE='d615ab15dfc3ae10ec0bd060b9df2d473854cdae093c98514b28ebee93e37407'
sha=lambda data:hashlib.sha256(data).hexdigest()

def git_files(ref):
    packed=subprocess.check_output(['git','archive',ref,*ROOTS],cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(packed)) as archive:
        return {p.name:archive.extractfile(p).read() for p in archive if p.isfile()}

def capture():
    rows=[];payloads={}
    for path in sorted(EXTERNAL.rglob('*')):
        relative=path.relative_to(EXTERNAL)
        if not path.is_file() or any(p in ('binaries','__pycache__','tmp') for p in relative.parts):continue
        data=path.read_bytes();digest=sha(data);payloads[digest]=base64.b64encode(data).decode()
        rows.append(dict(name=str(relative),path=str(path),sha256=digest,bytes=len(data)))
    return dict(records=rows,payloads=payloads)

def verify(store,originals=False,binaries=False):
    rows={r['name']:r for r in store['records']};assert len(rows)==len(store['records'])
    data={k:base64.b64decode(v,validate=True) for k,v in store['payloads'].items()}
    assert set(data)=={r['sha256'] for r in rows.values()}
    for r in rows.values():
        b=data[r['sha256']];assert sha(b)==r['sha256'] and len(b)==r['bytes']
        if originals:assert Path(r['path']).read_bytes()==b,r['name']
    def raw(n):return data[rows[n]['sha256']]
    def obj(n):return json.loads(raw(n))
    assert sha(raw('source-002/freeze.json'))==FREEZE
    source,claude,base=git_files(SOURCE),git_files(CLAUDE),git_files(BASE)
    assert set(source)==set(claude)==set(base)
    inv=obj('claude-inventory.json');assert len(inv['files'])==7
    paths={r['path'] for r in inv['files']};assert {p for p in source if source[p]!=base[p]}==paths
    oracle='crates/incant_render/src/upscaling/evidence_reference.rs'
    assert {p for p in source if source[p]!=claude[p]}=={oracle}
    fmt=obj('format-followup.json');assert fmt['exit']==0 and fmt['exact_expression_wrapper_change'] and fmt['unchanged_other_build_inputs']==2838
    assert sha(claude[oracle])==fmt['before_sha256'] and sha(source[oracle])==fmt['after_sha256']
    for r in inv['files']:assert sha(claude[r['path']])==r['candidate_sha256']
    for generation,expected in [(1,claude),(2,source)]:
        f=obj(f'source-{generation:03d}/freeze.json');assert len(f['files'])==2839 and len(f['source_copies'])==7
        for r in f['files']:assert sha(expected[r['path']])==r['sha256'] and len(expected[r['path']])==r['bytes']
        for r in f['source_copies']:assert raw(f'source-{generation:03d}/'+r['path'])==expected[r['path']]
    wgsl=[p for p in source if p.endswith('.wgsl')];assert len(wgsl)==21
    changed={'crates/incant_render/src/upscaling/resolve.wgsl','crates/incant_render/src/upscaling/resolve_owner.wgsl'}
    assert {p for p in wgsl if source[p]!=base[p]}==changed
    assert all(source[p]==claude[p] for p in wgsl)
    common=source['crates/incant_render/src/upscaling/common.wgsl']
    cooker=source['crates/incant_shader_cook/src/temporal/mod.rs'].decode()
    for kind,filename in [('Upscale','resolve.wgsl'),('UpscaleOwner','resolve_owner.wgsl')]:
        match=re.search(rf'Self::{kind} => \[([\d,\s]+)\]',cooker)
        pin=bytes(int(x) for x in match.group(1).split(',') if x.strip()).hex()
        assert pin==sha(common+b'\n'+source['crates/incant_render/src/upscaling/'+filename])==inv['trusted_pins'][kind]['candidate_pin']
    prior=obj('release.json');assert prior['released'] and prior['source_unchanged'] and prior['failure']=='fmt'
    assert len(prior['commands'])==1 and prior['commands'][0]['exit']==1 and prior['commands'][0]['reaped'] and prior['live_owned_groups']==[]
    for suffix in ['stdout','stderr']:assert sha(raw('fmt.'+suffix))==prior['commands'][0][suffix+'_sha256']
    final=obj('attempt-002/release.json');assert final['released'] and final['failure'] is None and final['exception'] is None and final['source_unchanged'] and not final['live_owned_groups'] and not final['gpu_executed']
    names=['fmt','diff','default','cooker','cooked-native','clippy-default','clippy-cooked-native','wasm']
    assert [r['name'] for r in final['commands']]==names
    counts={};arts=[]
    for r in final['commands']:
        name=r['name'];assert obj('attempt-002/'+name+'.json')==r
        assert r['exit']==0 and r['reaped'] and r['group_empty_after'] and not r['timed_out'] and not r['interrupted_by'] and r['source_unchanged']
        for suffix in ['stdout','stderr']:assert sha(raw('attempt-002/'+name+'.'+suffix))==r[suffix+'_sha256']
        out=raw('attempt-002/'+name+'.stdout').decode();summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',out,re.M)
        assert [list(x) for x in summaries]==r['test_summaries']
        if name in ['default','cooked-native']:
            want=['ok','167','0','43','0','0'] if name=='default' else ['ok','182','0','44','0','0'];assert r['test_summaries']==[want]
            counts[name]=[int(x) for x in want[1:]]
        if name=='cooker':assert sum(int(x[1]) for x in summaries)==34 and all(x[0]=='ok' and x[2:]==('0','0','0','0') for x in summaries);counts[name]=34
        for a in r['artifacts']:
            records=[]
            for line in out.splitlines():
                try:q=json.loads(line)
                except json.JSONDecodeError:continue
                if q.get('reason')=='compiler-artifact' and q.get('executable')==a['original']:records.append(q)
            assert len(records)==1
            q=records[0];assert q['target']==a['target'] and q['features']==a['features'] and q['profile']==a['compiler_profile'] and q['package_id']==a['package_id']
            assert q['profile']['test'] and a['mode']=='0555'
            if binaries:
                p=Path(a['path']);assert sha(p.read_bytes())==a['sha256'] and p.stat().st_size==a['bytes'] and p.stat().st_mode&0o777==0o555
            if name=='default':assert a['features']==['default','physics-deterministic']
            if name=='cooked-native':assert 'native-metalfx' in a['features'] and 'cooked-temporal' in a['features'] and 'physics-deterministic' in a['features'] and 'physics-fast-simd8' not in a['features']
            arts.append(a)
    assert len(arts)==10
    assert raw('attempt-002/wasm.stderr').count(b'warning: ')==6 # five existing diagnostics plus their summary
    return dict(records=len(rows),payloads=len(data),build_inputs=2839,changed_paths=7,unchanged_wgsl=19,changed_wgsl=2,
                checks=8,retained_failed_checks=1,cpu=counts,executable_copies=10,gpu_executed=False,appearance='R4 remains rejected; R5 unassessed',source=SOURCE)

def main():
    p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true');p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
    archive=HERE/'records.json.gz';manifest=HERE/'manifest.json'
    if a.verify_only:
        encoded=archive.read_bytes();m=json.loads(manifest.read_text());assert sha(encoded)==m['archive_sha256']
        result=verify(json.loads(gzip.decompress(encoded)),a.originals,a.local_binaries);assert result==m['result']
    else:
        store=capture();result=verify(store,True,True);encoded=gzip.compress(json.dumps(store,sort_keys=True,separators=(',',':')).encode(),mtime=0);archive.write_bytes(encoded)
        manifest.write_text(json.dumps(dict(source=SOURCE,claude_import=CLAUDE,claude_packet='51deb3848801f46c78ea85ca65fc330c5f60b758',base=BASE,source_freeze_sha256=FREEZE,archive_sha256=sha(encoded),result=result),indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
