#!/usr/bin/env python3
"""Public authored Rust/Wasm import, native development and ordinary package proof.

Creates and removes only this probe's private fixture, compiler inputs and new
export debug source maps. Repository source and unrelated caches stay intact.
No GPU, physical-device, performance, full SDK or phase completion claim.
"""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--wabt', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    sys.path.insert(0, str(root / 'tools'))
    import export_native
    import native_process
    import rust_guest
    from native_package import require, sha256, write_json
    Rpc = runpy.run_path(str(root / 'tools/probes/script-source-transactions.py'))['Rpc']
    output = args.output.resolve(); output.mkdir(parents=True, exist_ok=False)
    inputs = output / 'authoring'; inputs.mkdir()
    build = output / 'guest-build'; build.mkdir()
    project = inputs / 'project.json'; journal = inputs / 'project.journal.jsonl'
    binary = args.binary.resolve(strict=True)
    def execute(argv, **kwargs):
        result = native_process.run(list(map(str, argv)), text=True, encoding='utf-8', **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result
    def call_json(argv, **kwargs): return json.loads(execute(argv, **kwargs).stdout)
    execute([binary, 'init', project, '--entities', '0'])
    rustc = subprocess.check_output(['rustup', 'which', '--toolchain', '1.99.0', 'rustc'], text=True).strip()
    source = build / 'guest.rs'; source.write_bytes((root / 'crates/incant_play/fixtures/authored_module.rs').read_bytes())
    rust_guest.compile(rustc, source, build / 'guest.wasm')
    shutil.copy2(build / 'guest.wasm', inputs / 'guest.wasm')
    imported = call_json([binary, 'import', project, 'guest.wasm'])
    require(imported['asset']['kind'] == 'wasm', 'Public import did not create module asset')
    # Same bytes reimport is a true history-preserving cache hit.
    again = call_json([binary, 'import', project, 'guest.wasm'])
    require(again['cache_hit'] and not again['changed'] and again['revision'] == imported['revision'], 'Unchanged import changed history')
    entity = '00000000000000000000000002'
    scene = next(iter(json.loads(project.read_text())['scenes']))
    rpc = Rpc(binary, project, journal)
    try:
        before = rpc.call('project.read')
        def change(commands, description):
            current = rpc.call('project.read')
            return rpc.call('command.execute', {'expected_revision': current['revision'], 'description': description, 'commands': commands})
        change([{'op':'create_entity','scene_id':scene,'entity':{'id':entity,'name':'Mover','parent':None,'provenance':None,'components':{'Transform':{'translation':[0,0,0],'rotation':[0,0,0,1],'scale':[1,1,1]}}}},
                {'op':'set_input_actions','actions':{'right':{'kind':'button','bindings':[{'type':'key','code':'ArrowRight'}]}}},
                {'op':'set_scene_program','scene_id':scene,'program':{'kind':'wasm','module':imported['asset']['id']}}], 'Author module controls')
        selected = rpc.call('project.read')['project']
        rpc.call('history.undo'); require(rpc.call('project.read')['project'] == before['project'], 'Selection Undo not atomic')
        rpc.call('history.redo'); require(rpc.call('project.read')['project'] == selected, 'Selection Redo differs')
        change([{'op':'set_scene_program','scene_id':scene,'program':None}], 'Clear module')
        require('program' not in rpc.call('project.read')['project']['scenes'][scene], 'Clear did not restore default')
        rpc.call('history.undo'); require(rpc.call('project.read')['project'] == selected, 'Clear Undo differs')
        rpc.call('project.save')
        saved = rpc.call('project.read')
    finally: rpc.close()
    rpc = Rpc(binary, project, journal)
    try: require(rpc.call('project.read') == saved, 'Editor-default journal path reopen differs')
    finally: rpc.close()
    replay = output / 'input.json'
    write_json(replay, {'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':20,
        'frames':[{'tick':t,'events':[{'type':'key','code':'ArrowRight','down':down}]} for t,down in [(1,True),(7,False),(10,True),(16,False)]]})
    observation = output / 'observe.json'
    write_json(observation, {'format':'incant-native-observation','version':1,'entities':[entity],
        'checks':[{'name':'clock-input-edges-motion','tick':20,'expect':{'type':'vector','entity':entity,'property':'translation','value':[1.2,20,2],'tolerance':1e-9}}]})
    removed = {}
    for path in [source, inputs / 'guest.wasm']:
        removed[str(path.relative_to(output))] = sha256(path.read_bytes()); path.unlink()
    shutil.rmtree(build)
    dev = call_json([binary, 'native-play', project, '--ticks','20','--input-replay',replay,'--observe',observation])
    require(dev['passed'] and dev['observations']['passed'] and not dev['script_source_parsing'], 'Captured Wasmtime run failed')
    write_json(output / 'development.json', dev)
    cache = inputs / '.incant/cache/wasm' / (imported['asset']['sha256'] + '.wasm.json')
    original_cache = cache.read_bytes()
    corrupt = json.loads(original_cache); corrupt['prepared_sha256'] = '0' * 64
    write_json(cache, corrupt)
    rejected = native_process.run([str(binary), 'native-play', str(project), '--ticks', '1'], text=True, encoding='utf-8')
    require(rejected.returncode != 0 and 'identity mismatch' in rejected.stderr, 'Cache mismatch was not refused')
    cache.write_bytes(original_cache)
    report = export_native.export(project, output / 'package', wabt=args.wabt.resolve())
    require(report['program']['kind'] == 'wasm', 'Ordinary export did not select authored Wasm')
    for path in inputs.rglob('*'):
        if path.is_file(): removed[str(path.relative_to(output))] = sha256(path.read_bytes())
    shutil.rmtree(inputs)
    for path in (output / 'package/symbols').glob('*.map'):
        removed[str(path.relative_to(output))] = sha256(path.read_bytes()); path.unlink()
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    environment = {'PATH':str(player.parent),'HOME':str(output/'absent-home'),'TMPDIR':str(output),'TEMP':str(output),'TMP':str(output)}
    environment.update({k:os.environ[k] for k in ('SystemRoot','SystemDrive','WINDIR') if k in os.environ})
    runs = []
    for index in range(3):
        value = call_json([player,'--package',player.parent/'data','--ticks','20','--input-recording',replay,'--observe',observation,'--output',output/f'run-{index}.pcm'], cwd=player.parent, env=environment)
        require(value['observations']['passed'] and value['logs'] == dev['logs'] and value['tick'] == dev['tick'] and value['frames'] == dev['frames'], 'Development/package clock, controls or log parity failed')
        write_json(output/f'run-{index}.json',value); runs.append(value)
    refused_pcm, refused_save = output / 'unsupported.pcm', output / 'unsupported.checkpoint'
    refused = native_process.run([str(player),'--package',str(player.parent/'data'),'--ticks','1','--output',str(refused_pcm),'--save-checkpoint',str(refused_save)], cwd=player.parent, env=environment, text=True, encoding='utf-8')
    require(refused.returncode != 0 and 'static module state ABI' in refused.stderr and not refused_pcm.exists() and not refused_save.exists(), 'Static checkpoint was not refused before publication')
    result = {'passed':True,'target':report['target'],'player_sha256':report['player_sha256'],'program':report['program'],
        'public_import_select_undo_clear_reopen':True,'source_deleted_before_development_and_export':True,
        'authoring_cache_deleted_before_three_runs':True,'private_build_deleted':report['build_storage_removed_before_startup'],
        'fresh_ordinary_package_runs':len(runs),'ticks_each':20,'clock_controls_logs_equal':True,'deleted_private_file_sha256':removed,
        'cache_identity_mismatch_refused':True,'static_module_checkpoint':'Unsupported: static module state ABI','no_fixture_selection_flag':True,
        'limitations':['local desktop CPU proof','no rendered pixel, phone, performance, full SDK or phase gate','guest VM checkpoint not implemented']}
    write_json(output/'result.json',result); print(json.dumps(result,indent=2))


if __name__ == '__main__': main()
