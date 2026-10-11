#!/usr/bin/env python3
"""Full typed native ECS physics streams in fresh native and actual browser processes.

Rapier's native default uses the calling/global Rayon pool; the harness sets and
verifies that pool before constructing the actual NativeWorld. WASM has no Rayon
solver feature. Original raw-solver serial/joint parity and timings stay separate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]

def command(args, capture=False):
    print(' '.join(map(str,args)),flush=True)
    return subprocess.run(list(map(str,args)),cwd=ROOT,check=True,text=True,capture_output=capture).stdout

def inspect(data, scenario='3d'):
    if len(data)<32:
        raise ValueError('missing header')
    if scenario not in ('3d','2d'): raise ValueError('unknown scenario')
    size=169 if scenario=='2d' else 121
    magic=0x4e503244 if scenario=='2d' else 0x4e504831
    header=struct.unpack_from('<8I',data)
    if header[:5]!=(magic,1,35,641,size) or header[5]<16 or header[6]<8 or header[7]==0:
        raise ValueError(f'missing native execution coverage: {header}')
    if len(data)!=32+35*641*size:
        raise ValueError('incomplete native stream')
    return dict(bodies=35,frames=641,record_bytes=size,max_sleeping=header[5],wake_transitions=header[6],events=header[7],
                bytes=len(data),sha256=hashlib.sha256(data).hexdigest())

def compare(reference,candidate,scenario='3d'):
    record_size=inspect(candidate,scenario)['record_bytes']
    inspect(reference,scenario)
    if reference==candidate:
        return
    for i,(a,b) in enumerate(zip(reference,candidate)):
        if a!=b:
            if i<32:
                where=f'header byte {i}'
            else:
                record,offset=divmod(i-32,record_size)
                frame,body=divmod(record,35)
                field='stable ID' if offset<16 else 'sleeping' if offset==record_size-1 else f'numeric field {(offset-16)//8}'
                where=f'frame {frame}, body {body+1}, {field}'
            raise ValueError(f'{where}: {a:02x} != {b:02x}')
    raise ValueError('stream length mismatch')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--trials',type=int,default=3)
    parser.add_argument('--browser',action='store_true')
    parser.add_argument('--wasm-bindgen',default='wasm-bindgen')
    args=parser.parse_args()
    if args.trials<2:
        parser.error('at least two fresh trials required')
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    cargo='cargo' if sys.platform=='win32' else ROOT/'tools/cargo'
    target=HERE/'target'
    build=[cargo,'build','-j2','--manifest-path',HERE/'Cargo.toml','--release','--locked','--target-dir',target]
    command(build+['--bin','incant-native-physics-parity'])
    exe=target/'release'/('incant-native-physics-parity'+('.exe' if os.name=='nt' else ''))
    records=[];references={}
    for scenario in ('3d','2d'):
        for trial in range(args.trials):
            for workers in ([1,4] if trial%2==0 else [4,1]):
                file=out/f'native-{scenario}-{workers}-{trial}.bin'
                output=command([exe,workers,file]+(['--2d'] if scenario=='2d' else []),True)
                if not output.startswith(f'workers={workers} '):
                    raise ValueError('pool was not configured')
                data=file.read_bytes()
                references.setdefault(scenario,data)
                compare(references[scenario],data,scenario)
                records.append(dict(inspect(data,scenario),file=file.name,scenario=scenario,trial=trial,workers=workers,mode='native-parallel'))
    if args.browser:
        command(build+['--lib','--target','wasm32-unknown-unknown'])
        command([args.wasm_bindgen,'--target','web','--out-dir',out/'pkg',target/'wasm32-unknown-unknown/release/incant_native_physics_parity.wasm'])
        command(['node',HERE/'browser.mjs',out,args.trials])
        for scenario in ('3d','2d'):
            for trial in range(args.trials):
                file=out/f'browser-{scenario}-{trial}.bin';data=file.read_bytes();compare(references[scenario],data,scenario)
                records.append(dict(inspect(data,scenario),file=file.name,scenario=scenario,trial=trial,workers=0,mode='browser-wasm-serial'))
    sources=list((ROOT/'crates/incant_runtime/src').glob('*.rs'))+list((ROOT/'crates/incant_physics_runtime/src').glob('*.rs'))+list((ROOT/'crates/incant_physics_2d_runtime/src').glob('*.rs'))+list((ROOT/'crates/incant_types/src').glob('physics_2d*.rs'))+[ROOT/'Cargo.lock',HERE/'Cargo.lock',HERE/'Cargo.toml',HERE/'src/lib.rs',HERE/'src/main.rs',HERE/'src/planar.rs',HERE/'run.py',HERE/'browser.mjs']
    report=dict(status='passed',host=dict(system=platform.system(),machine=platform.machine()),trials=args.trials,
                browser_executed=args.browser,browser=json.loads((out/'browser.json').read_text()) if args.browser else [],
                records=records,source_sha256={p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
                scope='native cooked ECS full public 3D/2D rigid-body state plus planar ECS transforms/velocity; original native serial/joint solver proof remains separate; no timings, phone, backend-adoption or hidden solver-state claim')
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(status='passed',streams=len(records),browser_executed=args.browser)))

if __name__=='__main__':main()
