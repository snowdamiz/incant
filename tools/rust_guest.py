"""Compile trusted repository guests with the generated no_std SDK.

Only pinned rustc and these two fixed library roots are used: no Cargo, project
build scripts, proc macros, external guest dependencies or native code loading.
The owning cook/probe supplies trusted source and deletes this build storage.
"""
from pathlib import Path
import hashlib
import os
import subprocess

ROOT = Path(__file__).resolve().parents[1]


FLAGS = ['--edition=2024','--target=wasm32-unknown-unknown','-C','panic=abort','-C','opt-level=2']


def prepare_sdk(rustc, storage, environment, *, execute=subprocess.run, compiler=None):
    """Only engine-owned sources; the untrusted service supplies its bounded owner.

    Keep the trusted helper's recipe/digest and argument ordering unchanged.
    Never pass project source here or let an untrusted child write this storage.
    """
    rustc, storage = Path(rustc), Path(storage)
    storage.mkdir(exist_ok=True)
    roots=[('incant_wasm_abi',ROOT/'crates/incant_wasm_abi/src/lib.rs'),('incant_module',ROOT/'sdk/rust/src/lib.rs')]
    flags=FLAGS
    if compiler is None:compiler=subprocess.check_output([str(rustc),'-vV'],env=environment)
    for name,root in roots:
        library=storage/f'lib{name}.rlib'
        files=sorted(root.parent.rglob('*.rs'))
        abi_files=sorted(roots[0][1].parent.rglob('*.rs'))
        digest=hashlib.sha256(b''.join(p.relative_to(root.parent).as_posix().encode()+b'\0'+p.read_bytes()+b'\0' for p in files)+b''.join(p.relative_to(roots[0][1].parent).as_posix().encode()+b'\0'+p.read_bytes()+b'\0' for p in abi_files)+compiler+repr(flags).encode()).hexdigest()
        stamp=storage/f'{name}.sha256'
        if not library.exists() or not stamp.exists() or stamp.read_text()!=digest:
            command=[rustc,root,'--crate-name',name,'--crate-type=rlib',*flags,'-o',library]
            if name=='incant_module':command+=['--extern',f'incant_wasm_abi={storage/"libincant_wasm_abi.rlib"}']
            built=execute([str(x) for x in command],env=environment,capture_output=True,text=True,encoding='utf-8')
            if built.returncode:raise RuntimeError(f'trusted SDK library failed: {built.stderr}')
            stamp.write_text(digest)
    return storage


def compile(rustc, source, output, *, env=None, cfg=()):
    rustc, source, output = map(Path, (rustc, source, output))
    environment = dict(os.environ if env is None else env)
    environment['PATH'] = str(rustc.parent)+os.pathsep+environment.get('PATH','')
    storage=prepare_sdk(rustc,output.parent/'rust-sdk-libraries',environment,execute=subprocess.run)
    command=[rustc,source,'--crate-type=cdylib',*FLAGS,'--extern',f'incant_module={storage/"libincant_module.rlib"}','-L',f'dependency={storage}','-o',output]
    for value in cfg:command+=['--cfg',value]
    result=subprocess.run([str(x) for x in command],env=environment,capture_output=True,text=True,encoding='utf-8')
    if result.returncode:raise RuntimeError(f'trusted Rust SDK guest failed: {result.stderr}')
    return output


def provenance():
    """Exact build-time SDK sources; these repository originals are retained."""
    paths=[ROOT/'sdk/abi/native-v1.json',Path(__file__).resolve(),*sorted((ROOT/'crates/incant_wasm_abi/src').rglob('*.rs')),*sorted((ROOT/'sdk/rust/src').rglob('*.rs'))]
    return {'target':'wasm32-unknown-unknown','edition':'2024','panic':'abort','opt_level':2,
            'source_sha256':{p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
            'repository_sdk_sources_retained':True,'build_scripts_or_proc_macros':False}


def main():
    import argparse
    import json
    import sys
    if '--untrusted' in sys.argv[1:]:
        from rust_guest_build import main as untrusted_main
        return untrusted_main(sys.argv[1:])
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path,help='Trusted Rust guest source')
    parser.add_argument('output',type=Path)
    parser.add_argument('--rustup',default='rustup')
    args=parser.parse_args()
    if args.output.exists():raise SystemExit('Output already exists')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    rustc=subprocess.check_output([args.rustup,'which','--toolchain','1.99.0','rustc'],text=True,encoding='utf-8').strip()
    compile(rustc,args.source,args.output)
    print(json.dumps({'wasm':str(args.output),'wasm_sha256':hashlib.sha256(args.output.read_bytes()).hexdigest(),'sdk':provenance()},indent=2))

if __name__=='__main__':main()
