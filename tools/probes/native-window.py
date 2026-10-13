#!/usr/bin/env python3
"""Fresh source-free interactive desktop player proof; no pixel capture/review.

Consumes cooked fixtures from a completed native-game.py --gpu proof. That
fixture ledger stays separate; this proof does not replace its 28 launches.
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

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from shipping import cargo_output, codegen, player_units
from shipping_symbols import backend, package, require, run


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('fixtures', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    source = args.fixtures.resolve()
    baseline = json.loads((source / 'result.json').read_text())
    require(baseline['gpu_submitted'] and baseline['authoring_and_build_storage_deleted'], 'Need completed source-free GPU fixture ledger')
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target)
    require(kind != 'pdb', 'Executable source-compiler symbol inspection currently supports Mach-O/ELF only')
    split = 'packed' if kind == 'dsym' else 'off'
    llvm = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key} for shipping audit')
    env = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
    env.pop('INCANT_NATIVE_SCRIPT_MAP', None)
    sentinel = 'INCANT_WINDOW_SOURCE_COMMENT_3c9db101'
    with tempfile.TemporaryDirectory(prefix='source-', dir=out) as src, tempfile.TemporaryDirectory(prefix='build-', dir=out) as build_dir:
        script = Path(src) / 'window.js'
        # Six dynamic parsing routes must fail inside the actual embedded VM.
        script.write_text('// ' + sentinel + '\n' + '''
for (const compile of [() => eval('1'), () => Function('return 1'),
 () => new Function('return 1'), () => (function(){}).constructor('return 1'),
 () => (async function(){}).constructor('return 1'), () => (function*(){}).constructor('yield 1')]) {
 let denied=false;try{compile();}catch(_){denied=true;}if(!denied)throw Error('runtime source compiler active');
}
''' + (ROOT / 'crates/incant_game_player/fixtures/game.js').read_text())
        env.update(INCANT_NATIVE_SCRIPT_SOURCE=str(script), INCANT_NATIVE_SCRIPT_NAME='embedded-window.js')
        command = [str(ROOT/'tools/cargo'), 'build', '-j2', '-p', 'incant_game_player', '--features', 'native-window', '--profile', 'shipping', '--locked', '--target', target, '--target-dir', build_dir, '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
        build = cargo_output(command, cwd=ROOT, env=env)
        (out/'build.log').write_text(build.stderr)
        require(build.returncode == 0, f'Build failed: {out}/build.log')
        lines = [line for line in build.stderr.splitlines() if 'Running `' in line and f'--target {target}' in line]
        forbidden = ('incant_doc','incant_cmd','incant_agent','incant_editor','incant_assets','incant_audio','incant_core','incant_cook','incant_play_legacy','incant_render_authoring','loro')
        require(not any(re.search(r'--crate-name ('+'|'.join(forbidden)+r')\b', line) for line in lines), 'Authoring target dependency leaked')
        flags = {}
        for crate in ('incant_game_runtime','incant_render_game','incant_render_gpu_host','incant_render_native','incant_render','incant_render_data','incant_runtime','incant_script','incant_input','incant_audio_runtime','incant_assets_runtime','incant_physics_runtime','rapier3d','wgpu','tao','kira','cpal'):
            found = [line for line in lines if re.search(r'--crate-name '+crate+r'\b', line)]
            require(len(found) == 1 and 'feature="compiler"' not in found[0], f'Missing/duplicate/compiling source in target {crate}')
            values = codegen(found[0]); expected = {'opt-level':'3','codegen-units':'1','panic':'abort'}
            require(all(values.get(k)==v for k,v in expected.items()) and values.get('target-cpu')!='native', f'Nonshipping flags: {crate}')
            flags[crate] = {k:values[k] for k in expected}
        flags.update(player_units(build.stderr, target, split,
            {'default', 'physics-deterministic', 'native-window', 'render-gpu', 'render-input'}))
        artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
        executables = [a['executable'] for a in artifacts if a.get('reason')=='compiler-artifact' and a['target']['name']=='incant_game_player' and a['target']['kind']==['bin'] and a.get('executable')]
        require(len(executables)==1, 'Missing unique player')
        executable = Path(executables[0])
        symbols = run([llvm/'llvm-nm','--defined-only','--demangle',executable])
        require('JS_ReadObject' in symbols and 'JS_EvalFunction' in symbols, 'Bytecode runtime missing')
        for name in ('JS_AddIntrinsicEval','__JS_EvalInternal','js_parse_program','js_parse_function_decl','incant_script::compiler'):
            require(name not in symbols, f'Source compiler linked: {name}')
        require(sentinel.encode() not in executable.read_bytes(), 'Source comment embedded')
        player, identity = package(executable,out,kind,llvm)
        maps = list(Path(build_dir).glob('*/shipping/build/incant_game_player-*/out/script-symbols.json'))
        require(len(maps)==1,'Missing script symbol metadata')
        shutil.copy2(maps[0],out/'symbols/script-symbols.json')
    require(not Path(src).exists() and not Path(build_dir).exists(), 'Source/build storage remains')
    runtime=out/'player'
    for name in ('audio','models'):
        shutil.copytree(source/'player'/name,runtime/name)
    for name in ('static.scene','stream.scene','clips.bin','actions.json'):
        shutil.copy2(source/'player'/name,runtime/name)
    (runtime/'textures').mkdir()
    camera='00000000000000000000000000000004'
    def command(scene='static.scene',actions='actions.json',camera_id=camera):
        return [str(player),'--window',str(runtime/scene),str(runtime/'audio'),str(runtime/'clips.bin'),str(runtime/actions),str(runtime/'models'),str(runtime/'textures'),camera_id,'--lifecycle-probe']
    results=[]
    for scene in ('static.scene','stream.scene'):
        process=subprocess.run(command(scene),capture_output=True,text=True,timeout=30,cwd=runtime)
        require(process.returncode==0,f'Native lifecycle failed: {process.stderr}')
        report=json.loads(process.stdout);probe=report['lifecycle_probe']
        require(report['tick']==40 and report['accepted_frames']==40+report['presentation_refreshes'] and report['on_screen'],'Frame publication failed')
        require(probe['actual_resize'] and probe['actual_minimize_restore'] and 16 <= probe['suspended_tick'] <= 19 and probe['explicit_surface_recreation'] and report['surface_recreations']==1,'Lifecycle incomplete')
        results.append({'scene':scene,'report':report,'stderr':process.stderr})
    bad={'unsupported':{'kind':'button','bindings':[{'type':'gamepad_button','id':0,'button':'south'}]}}
    (runtime/'unsupported-actions.json').write_text(json.dumps(bad))
    rejected=[]
    for label,cmd in [('unsupported-gamepad',command(actions='unsupported-actions.json')),('malformed-camera',command(camera_id='bad'))]:
        process=subprocess.run(cmd,capture_output=True,text=True,timeout=10,cwd=runtime)
        require(process.returncode!=0 and not process.stdout,f'{label} accepted')
        rejected.append({'case':label,'stderr':process.stderr})
    result={'scope':'actual native surface/device startup and numerical lifecycle; no pixel/appearance or audible-output review','compiler':compiler,'target':target,'player_sha256':sha(player),'player_bytes':player.stat().st_size,'symbol_identity':identity,'fixture_ledger_sha256':sha(source/'result.json'),'fixture_inputs':{p.name:sha(p) for p in runtime.iterdir() if p.suffix=='.scene'},'source_and_build_deleted':True,'codegen':flags,'source_compiler_symbols_absent':True,'dynamic_source_routes_rejected':True,'results':results,'rejected':rejected}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(f'Native window proof passed: {out}/result.json')

if __name__=='__main__':
    main()
