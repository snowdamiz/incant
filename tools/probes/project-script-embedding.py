#!/usr/bin/env python3
"""Fresh build-produced bound-project bytecode, then source-deleted execution.

CPU-only native host==target proof. This uses an engine-owned initial test project,
not a project mutation API. Only the disposable engine snapshot gains a probe
workspace member. Runtime dependencies and actual target compiler flags are
checked separately from the authoring/compiler build graph. No phone/export gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

TEMPLATES = Path(__file__).with_suffix('')
NAME = 'incant_project_script_probe'


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(command, *, cwd=None, env=None, timeout=60):
    result = subprocess.run(list(map(str, command)), cwd=cwd, env=env,
                            capture_output=True, text=True, encoding='utf-8', timeout=timeout)
    require(result.returncode == 0, f'{command[0]} failed: {result.stderr[-6000:]}')
    return result


def snapshot(root, destination):
    """Copy exact tracked/untracked source, never caches, user credentials or .git."""
    listing = execute(['git', '-C', root, 'ls-files', '-c', '-o', '--exclude-standard', '-z']).stdout
    allowed = ('crates/', 'third_party/', 'sdk/', 'tools/', 'editor/app/', '.cargo/')
    hashes = {}
    for name in sorted(set(listing.split('\0')) - {''}):
        source = root/name
        if name not in ('Cargo.toml','Cargo.lock','rust-toolchain.toml') and not name.startswith(allowed):
            continue
        if not source.is_file():
            continue
        target = destination/name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        hashes[name] = digest(target)
    require(all(digest(root/name) == value for name,value in hashes.items()), 'Engine source changed during snapshot')
    return hashes


def normal_graph(metadata):
    names = {p['id']:p['name'] for p in metadata['packages']}
    nodes = {n['id']:n for n in metadata['resolve']['nodes']}
    pending = [i for i,name in names.items() if name == NAME]
    require(len(pending) == 1, 'Missing unique probe package')
    seen = set()
    forbidden = {'incant_doc','incant_cmd','incant_agent','incant_core','incant_cook','incant_play_legacy','loro','automerge'}
    while pending:
        ident = pending.pop()
        if ident in seen:
            continue
        seen.add(ident)
        require(names[ident] not in forbidden and not names[ident].startswith(('wasmtime','cranelift')),
                f'Authoring/JIT normal dependency: {names[ident]}')
        pending.extend(d['pkg'] for d in nodes[ident]['deps'] if any(k['kind'] is None for k in d['dep_kinds']))
    return sorted(names[i] for i in seen)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--engine-root', type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    root, out = args.engine_root.resolve(), args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    sys.path.insert(0, str(root/'tools'))
    from shipping import codegen
    from shipping_symbols import backend, package
    rustc = Path(execute(['rustup','which','rustc'], cwd=root).stdout.strip())
    compiler = execute([rustc,'-vV']).stdout
    host = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(host)
    split = 'packed' if kind in ('dsym','pdb') else 'off'
    suffix = '.exe' if os.name == 'nt' else ''
    llvm_bin = Path(execute([rustc,'--print','sysroot']).stdout.strip())/'lib/rustlib'/host/'bin'
    node = Path(shutil.which('node') or '').resolve()
    require(node.is_file(), 'Pinned development Node installation is required')
    for key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key}')
    environment = dict(os.environ, PATH=str(rustc.parent)+os.pathsep+os.environ.get('PATH',''))
    for key in list(environment):
        if key.startswith('INCANT_NATIVE_SCRIPT_'):
            del environment[key]
    template_hashes = {p.relative_to(TEMPLATES).as_posix():digest(p) for p in TEMPLATES.rglob('*') if p.is_file()}
    with tempfile.TemporaryDirectory(prefix='private-build-', dir=out) as temporary:
        storage = Path(temporary)
        engine = storage/'engine'
        print('Snapshotting frozen engine and trusted probe templates...', flush=True)
        source_hashes = snapshot(root, engine)
        crate = engine/'crates'/NAME
        shutil.copytree(TEMPLATES, crate)
        # Only build-time compiler packages resolve here. The symlink itself is
        # removed with the snapshot; no package data is included in the player.
        (engine/'node_modules').symlink_to((root/'node_modules').resolve(), target_is_directory=True)
        fixtures = crate/'fixtures'
        catalog = [{'id':'0000000000000000000000000A','path':'a.ts'},
                   {'id':'0000000000000000000000000B','path':'b.ts'}]
        node_env = {'PATH':str(node.parent),'TMPDIR':str(storage),'TEMP':str(storage),'TMP':str(storage),
                    'SWC_NATIVE_BINDING_CACHE':str(storage.resolve()/'swc')}
        node_env.update({key:os.environ[key] for key in ('SystemRoot','SystemDrive','WINDIR') if key in os.environ})
        catalog_file = storage/'catalog.json'
        catalog_file.write_text(json.dumps(catalog), encoding='utf-8')
        with catalog_file.open('rb') as stdin:
            transformed = subprocess.run([str(node),'--max-old-space-size=512','--disable-proto=throw',
                str(engine/'tools/build_native_script.mjs'),'--project',str(fixtures),str(storage/'ts')],
                stdin=stdin,cwd=engine,env=node_env,capture_output=True,text=True,encoding='utf-8',timeout=30)
        (out/'node.log').write_text(transformed.stderr,encoding='utf-8')
        require(transformed.returncode == 0, f'Fixed TS cook failed: {transformed.stderr}')
        manifest = json.loads((storage/'ts/manifest.json').read_text())
        require(manifest['sources'] == catalog, 'Unexpected source identity/order')
        for name,key in [('behavior.cjs','bundleSha256'),('behavior.cjs.map','mapSha256')]:
            require(digest(storage/'ts'/name) == manifest[key], 'Fresh TS output identity mismatch')
        for name in ('behavior.cjs','manifest.json'):
            shutil.copy2(storage/'ts'/name,fixtures/name)
        (out/'typescript-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
        cargo = engine/'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe')
        before_lock = tomllib.loads((engine/'Cargo.lock').read_text())['package']
        # Add only the engine-owned test package, preserving every pinned package.
        metadata = json.loads(execute([cargo,'metadata','--offline','--format-version=1'],cwd=engine,env=environment).stdout)
        after_lock = tomllib.loads((engine/'Cargo.lock').read_text())['package']
        require([p for p in after_lock if p['name'] != NAME] == before_lock, 'Probe changed existing dependency resolutions')
        dependencies = normal_graph(metadata)
        (out/'normal-dependencies.json').write_text(json.dumps(dependencies,indent=2)+'\n')
        command = [cargo,'build','-j2','-p',NAME,'--profile','shipping','--locked','--offline',
                   '--target',host,'--target-dir',storage/'target','--config',f'profile.shipping.split-debuginfo="{split}"',
                   '--message-format=json','-vv']
        print('Building actual generated Rust with isolated shipping target...', flush=True)
        build = subprocess.run(list(map(str,command)),cwd=engine,env=environment,capture_output=True,text=True,encoding='utf-8',timeout=1800)
        (out/'build.log').write_text(build.stderr,encoding='utf-8')
        require(build.returncode == 0, f'Embedding build failed; see {out/"build.log"}')
        flags = {}
        for name in ('incant_runtime','incant_script',NAME):
            commands = [line for line in build.stderr.splitlines() if 'Running `' in line
                        and re.search(r'--crate-name '+name+r'\b',line) and f'--target {host}' in line]
            require(len(commands) == 1, f'Missing fresh target invocation for {name}')
            require('feature="compiler"' not in commands[0], 'Runtime compiler feature leaked')
            actual = codegen(commands[0])
            expected = {'opt-level':'3','codegen-units':'1','panic':'abort','debuginfo':'2','split-debuginfo':split}
            if name == NAME:
                expected['lto'] = 'fat'
            require(all(actual.get(k)==v for k,v in expected.items()) and actual.get('target-cpu') != 'native', f'Incorrect codegen {actual}')
            flags[name] = expected
        target_lines = [line for line in build.stderr.splitlines() if 'Running `' in line and f'--target {host}' in line]
        require(not any(re.search(r'--crate-name (?:incant_doc|incant_cmd|incant_agent|incant_cook|incant_core|loro|wasmtime)\b',line) for line in target_lines), 'Authoring dependency compiled into target graph')
        artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
        executable = [Path(a['executable']) for a in artifacts if a.get('reason')=='compiler-artifact' and a['target']['name']==NAME and a.get('executable')]
        require(len(executable)==1,'Missing executable')
        symbols = execute([llvm_bin/f'llvm-nm{suffix}','--defined-only','--demangle',executable[0]]).stdout
        require('JS_ReadObject' in symbols and 'JS_EvalFunction' in symbols, 'Missing bytecode runtime')
        forbidden = ('JS_AddIntrinsicEval','__JS_EvalInternal','js_parse_program','js_parse_function_decl','incant_script::compiler')
        require(not any(name in symbols for name in forbidden), 'Source compiler linked')
        require(b'INCANT_EMBEDDING_TS_SOURCE_SENTINEL_67d2c3' not in executable[0].read_bytes(),'Source comment embedded')
        player, identity = package(executable[0],out,kind,llvm_bin)
        outputs = list((storage/'target'/host/'shipping/build').glob(f'{NAME}-*/out/behavior.rs'))
        require(len(outputs)==1,'Expected one actual generated Rust source')
        generated_hash = digest(outputs[0])
        require('ScriptBindings::new' in outputs[0].read_text(),'Bound Rust emitter was not exercised')
        shutil.copy2(outputs[0].with_name('world.scene'),out/'player/world.scene')
        (out/'bindings.json').write_bytes(outputs[0].with_name('bindings.json').read_bytes())
        authoring_hash = digest(outputs[0].with_name('project.incant.json'))
    require(not storage.exists(),'Build/source storage retained')
    print('Authoring, TS, Node outputs, generated Rust and target removed; running copied player...',flush=True)
    player_dir = out/'player'
    runtime_env = {'PATH':str(player_dir),'HOME':str(player_dir/'absent-home'),'TMPDIR':str(player_dir),'TEMP':str(player_dir),'TMP':str(player_dir)}
    runtime_env.update({key:os.environ[key] for key in ('SystemRoot','SystemDrive','WINDIR') if key in os.environ})
    before = {p.name:digest(p) for p in player_dir.iterdir() if p.is_file()}
    observations = []
    for ticks in (0,1,4,16):
        results = []
        for _ in range(3):
            result = json.loads(execute([player,'run','world.scene',ticks],cwd=player_dir,env=runtime_env).stdout)
            require(result['tick']==ticks and result['panic_abort'] and not result['source_parsing'],'Incorrect execution mode')
            require(result['translations']==[[speed*ticks/4,0,0] for speed in (2,3,4)],'Grouped movement mismatch')
            logs=result['logs']['entries']
            require([entry['message'] for entry in logs]==[f'group {group} tick {tick} 🚀' for tick in range(1,ticks+1) for group in (1,2)],'Source order/shared module state mismatch')
            results.append(result)
        require(results[0]==results[1]==results[2],'Fresh-process results differ')
        observations.append(results[0])
    failures = []
    for mode in ('eval','function','throw'):
        result=json.loads(execute([player,'rollback','world.scene',mode],cwd=player_dir,env=runtime_env).stdout)
        require(result['rollback'] and result['terminal'],'Failed callback prefix escaped')
        failures.append(result)
    pairing=json.loads(execute([player,'pairing','world.scene','unused'],cwd=player_dir,env=runtime_env).stdout)
    require(pairing['pairing_refused'],'Wrong scene accepted')
    require(before=={p.name:digest(p) for p in player_dir.iterdir() if p.is_file()},'Player mutated its package')
    result={'passed':True,'fresh_processes':16,'target':host,'compiler':compiler,'codegen':flags,'normal_dependencies':dependencies,
            'symbols':identity,'player_bytes':player.stat().st_size,'player_sha256':digest(player),
            'source_parser_symbols_absent':True,'node_on_runtime_path':False,'inputs_and_build_storage_removed':True,
            'generated_rust_sha256':generated_hash,'authoring_fixture_sha256':authoring_hash,'template_sha256':template_hashes,
            'engine_source_sha256':source_hashes,'observations':observations,'failure_controls':failures,'pairing':pairing,
            'limits':['host==target desktop only','typed initial authoring fixture; no new project mutation API','Node and source parsing are build-time only',
                      'repository and pinned tool installation retained outside private build snapshot','no GPU, device, export pipeline, save or performance gate']}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':True,'fresh_processes':16,'player_sha256':result['player_sha256']}))


if __name__ == '__main__':
    main()
