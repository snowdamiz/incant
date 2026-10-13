#!/usr/bin/env python3
"""Public Wasm/Android preflight: captured Rust, native per-field oracle and Wasmtime."""
import argparse
import json
import math
from pathlib import Path
import runpy
import shutil
import struct
import sys
import time
import wave

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process
from native_package import require, sha256, write_json

# Reuse the accepted real Rust audio game and its independent native oracle.
# Every desired declaration/identity/epoch word is observed; no aggregate checksum.
audio = runpy.run_path(str(ROOT / 'tools/probes/native-wasm-audio-export.py'))
TICKS, ACTIONS = audio['TICKS'], audio['ACTIONS']
ulid, entities, observation, check_logs = (audio[key] for key in ('ulid', 'entities', 'observation', 'check_logs'))
SOURCE_PATHS = [Path(__file__).relative_to(ROOT), Path('tools/probes/native-wasm-audio-export.py'),
                Path('tools/probes/native-wasm-audio-export/behavior.rs'),
                Path('crates/incant_cook/examples/audio_package_oracle.rs'),
                Path('crates/incant_wasm_runtime/tests/support/audio_transport.rs'),
                Path('tools/rust_guest.py'), Path('tools/probes/script-source-transactions.py')]


def files(root):
    return {str(path.relative_to(root)): sha256(path.read_bytes())
            for path in sorted(root.rglob('*')) if path.is_file()}


def main():
    require(__debug__, 'Proof requires Python assertions enabled')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--play-binary', type=Path, required=True)
    parser.add_argument('--oracle', type=Path, required=True)
    parser.add_argument('--physics-mode', choices=('deterministic', 'fast'), required=True)
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    binary, player, oracle = (path.resolve(strict=True) for path in (args.binary, args.play_binary, args.oracle))
    source_hashes = {str(path): sha256((ROOT / path).read_bytes()) for path in SOURCE_PATHS}
    commands, calls, removed = [], [], {}

    def execute(argv, expected=0):
        argv = list(map(str, argv)); index = len(commands)
        record = {'argv': argv, 'cwd': str(ROOT), 'exit_code': None}; commands.append(record)
        write_json(out / 'commands.json', commands); started = time.monotonic()
        result = native_process.run(argv, cwd=ROOT, timeout=600)
        record.update(exit_code=result.returncode, seconds=time.monotonic()-started)
        for stream in ('stdout', 'stderr'):
            data = getattr(result, stream).encode(); path = out / f'command-{index:03d}.{stream}'
            path.write_bytes(data); record[stream] = {'path': path.name, 'sha256': sha256(data)}
        write_json(out / 'commands.json', commands)
        require(result.returncode == expected, f'Command {index}: expected {expected}: {result.stderr[-4000:]}')
        return result

    def value(argv):
        return json.loads(execute(argv).stdout)

    authoring = out / 'authoring'; authoring.mkdir()
    project, journal = authoring / 'game.json', authoring / 'game.journal.jsonl'
    build = out / 'guest-build'; build.mkdir()
    value([binary, 'init', project, '--entities', 0])
    clips = []
    for name, rate in [('static', 22050), ('stream', 32000)]:
        path = authoring / (name + '.wav')
        with wave.open(str(path), 'wb') as stream:
            stream.setnchannels(2); stream.setsampwidth(2); stream.setframerate(rate)
            stream.writeframes(b''.join(struct.pack('<hh', int(7000*math.sin(i*.13)), int(6000*math.cos(i*.17))) for i in range(2309)))
        imported = value([binary, 'import', project, path.name]); clips.append(imported['asset']['id'])
    source = build / 'behavior.rs'; source.write_bytes((ROOT / SOURCE_PATHS[2]).read_bytes())
    compiled = value([sys.executable, ROOT / 'tools/rust_guest.py', source, build / 'behavior.wasm', '--rustup', '/opt/homebrew/bin/rustup'])
    write_json(out / 'guest-compile.json', compiled)
    shutil.copy2(build / 'behavior.wasm', authoring / 'behavior.wasm')
    module = value([binary, 'import', project, 'behavior.wasm'])
    write_json(out / 'module-import.json', module)
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    rpc = Rpc(binary, project, journal)

    def call(method, params=None):
        result = rpc.call(method, params)
        calls.append({'method': method, 'params': params, 'result': result})
        write_json(out / 'public-commands.json', calls)
        return result

    try:
        before = call('project.read')
        transaction = [{'op': 'delete_scene', 'scene_id': scene} for scene in before['project']['scenes']]
        transaction += [{'op': 'create_scene', 'scene': {'id': ulid(99), 'name': 'Authored Android Rust module', 'entities': entities(clips)}},
                        {'op': 'set_scene_program', 'scene_id': ulid(99), 'program': {'kind': 'wasm', 'module': module['asset']['id']}},
                        {'op': 'set_physics_mode', 'mode': args.physics_mode},
                        {'op': 'set_input_actions', 'actions': {name: {'kind': 'button', 'bindings': [{'type': 'key', 'code': key}]} for _, name, key in ACTIONS}}]
        call('command.execute', {'expected_revision': before['revision'], 'description': 'Author Android captured Rust program and physics selection', 'commands': transaction})
        selected = call('project.read')['project']; call('history.undo')
        require(call('project.read')['project'] == before['project'], 'Atomic module/mode Undo')
        call('history.redo'); require(call('project.read')['project'] == selected, 'Atomic module/mode Redo')
        call('project.save'); saved = call('project.read')
    finally:
        rpc.close()
    rpc = Rpc(binary, project, journal)
    try:
        require(call('project.read') == saved, 'Persistent journal reopened through public RPC with --journal')
    finally:
        rpc.close()
    for path in [authoring / name for name in ('static.wav', 'stream.wav', 'behavior.wasm')]:
        removed[str(path.relative_to(out))] = sha256(path.read_bytes()); path.unlink()
    removed.update({str((build / path).relative_to(out)): digest for path, digest in files(build).items()})
    shutil.rmtree(build); write_json(out / 'removed.json', removed)
    unchanged = files(authoring)
    rows = [json.loads(line) for line in execute([oracle, project, ulid(99), out / 'oracle.scene', out / 'oracle.pcm', TICKS]).stdout.splitlines()]
    require(len(rows) == TICKS and [row['tick'] for row in rows] == list(range(1, TICKS+1)), 'Native oracle clocks')
    require(sum(row['pcm_nonzero'] for row in rows) > 40 and any(row['pcm_silent'] for row in rows), 'Native mixer played and paused')
    write_json(out / 'oracle.json', rows); write_json(out / 'observe.json', observation(rows))
    require(len(observation(rows)['checks']) == 248, 'Bounded public observations')
    replay = {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0, 'ticks': TICKS, 'frames': []}
    write_json(out / 'positive-input.json', replay)
    development = []
    for index in range(2):
        report = value([player, 'native-play', project, '--camera', ulid(300), '--ticks', TICKS,
                        '--input-replay', out / 'positive-input.json', '--observe', out / 'observe.json', '--pcm-output', out / f'development-{index}.pcm'])
        require(report['passed'] and report['observations']['passed'] and not report['script_source_parsing']
                and report['cpu_render_input'] and not report['gpu_submitted'], 'Captured-only Wasmtime development')
        check_logs(report['logs'], rows, require)
        require((out / f'development-{index}.pcm').read_bytes() == (out / 'oracle.pcm').read_bytes(), 'Exact native oracle PCM')
        write_json(out / f'development-{index}.json', report); development.append(report)
    require(development[0] == development[1], 'Fresh development session parity')
    for name, _, key in ACTIONS:
        recording = {**replay, 'frames': [{'tick': 14, 'events': [{'type': 'key', 'code': key, 'down': True}]}]}
        path = out / (name + '-input.json'); write_json(path, recording)
        pcm = out / ('refused-' + name + '.pcm')
        response = execute([player, 'native-play', project, '--camera', ulid(300), '--ticks', TICKS,
                            '--input-replay', path, '--pcm-output', pcm], expected=1)
        require(response.stderr.strip() == audio['DEV_ERRORS'][name] and not response.stdout and not pcm.exists(),
                'Exact development refusal with no output: ' + name)
    require(files(authoring) == unchanged, 'Development changed project/cache/journal')
    require(source_hashes == {str(path): sha256((ROOT / path).read_bytes()) for path in SOURCE_PATHS}, 'Preflight source changed')
    write_json(out / 'result.json', {'passed': True, 'project': str(project), 'scene': ulid(99), 'physics_mode': args.physics_mode,
                                   'authoring_unchanged': unchanged, 'program_asset': module['asset'], 'sources': source_hashes,
                                   'binaries': {str(path): sha256(path.read_bytes()) for path in (binary, player, oracle)},
                                   'oracle_scene_sha256': sha256((out / 'oracle.scene').read_bytes()), 'ticks': TICKS,
                                   'records_per_tick': 6, 'scalars_per_tick': 186, 'bounded_vector_checks': 248,
                                   'public_import_select_mode_undo_redo_reopen': True, 'source_deleted': removed,
                                   'development_runs': 2, 'refusals': [name for name, _, _ in ACTIONS]})
    print(json.dumps({'passed': True, 'output': str(out)}))


if __name__ == '__main__':
    main()
