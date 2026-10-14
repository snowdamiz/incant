#!/usr/bin/env python3
"""Verify retained script qualification without running or timing native code."""
import argparse
import base64
import gzip
import hashlib
import json
import re
import subprocess
from pathlib import Path


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args=parser.parse_args()
    directory=Path(__file__).resolve().parent
    root=directory.parents[3]
    sha=lambda b: hashlib.sha256(b).hexdigest()
    manifest=json.loads((directory/'manifest.json').read_text())
    archive=(directory/'records.json.gz').read_bytes()
    assert sha(archive)==manifest['archive_sha256']
    payloads=json.loads(gzip.decompress(archive)); records={}
    for row in manifest['records']:
        if row['external']:
            if not (args.external or args.originals): continue
            data=Path(row['original']).read_bytes()
        else:
            data=base64.b64decode(payloads[row['sha256']],validate=True)
        assert sha(data)==row['sha256'] and len(data)==row['bytes'], row
        if args.originals: assert Path(row['original']).read_bytes()==data, row
        records[row['name']]=data
    for row in manifest['current_sources']:
        assert sha((root/row['path']).read_bytes())==row['sha256'], row
    read=lambda name: json.loads(records[name])
    result=read('checks-001/result.json')
    assert result['passed'] and result['source_unchanged']
    assert result['source_commit']==manifest['source_commit']
    assert sha(records['checks-001/inputs.json'])==result['inputs_sha256']
    assert [r['name'] for r in result['commands']]==[
        'toolchain','focused','deterministic','fast8','clippy-deterministic',
        'clippy-fast8','fmt','conventions','diff']
    copies=[]
    for row in result['commands']:
        assert row['exit_code']==0
        for stream in ('stdout','stderr'):
            assert sha(records['checks-001/'+row['name']+'.'+stream])==row[stream]['sha256']
        for binary in row['binaries']:
            match=next(v for v in manifest['records'] if v['original']==binary['path'])
            assert match['sha256']==binary['sha256'] and match['bytes']==binary['bytes']
            assert binary['mode']=='0o755'
            assert binary['original'] in records['checks-001/'+row['name']+'.stderr'].decode()
            copies.append(binary)
    assert len(copies)==19
    for name,expected in [('focused',[9]),('deterministic',[148,26,4,6,10,4,0,17,2]),
                          ('fast8',[148,26,4,6,10,4,0,17,2])]:
        summaries=re.findall(rb'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored',
                             records['checks-001/'+name+'.stdout'])
        assert [int(v[1]) for v in summaries]==expected, (name,summaries)
        assert all(v[0]==b'ok' and v[2:]==(b'0',b'0') for v in summaries)
    assert read('checks-001/processes.json')['all_absent']
    freeze=read('source-001/freeze.json')
    assert freeze['commit']==manifest['source_commit']
    for row in freeze['inputs']:
        assert sha(records['source-001/files/'+row['path']])==row['sha256']
    inputs=read('checks-001/inputs.json')
    if args.git_source:
        query=''.join(manifest['source_commit']+':'+row['path']+'\n' for row in inputs).encode()
        blobs=subprocess.check_output(['git','cat-file','--batch'],cwd=root,input=query)
        position=0
        for row in inputs:
            end=blobs.index(b'\n',position)
            header=blobs[position:end].split(); assert header[1]==b'blob', row
            size=int(header[2]); position=end+1
            data=blobs[position:position+size]; position+=size+1
            assert size==row['bytes'] and sha(data)==row['sha256'], row
        assert position==len(blobs)
    print(json.dumps(dict(passed=True,records=len(manifest['records']),payloads=len(payloads),
                          inputs=len(inputs),commands=9,copied_binaries=19,focused=9,
                          deterministic=217,fast8=217,scope='CPU qualification, no shipping performance gate')))


if __name__=='__main__':
    main()
