#!/usr/bin/env python3
"""Public authored IK controls/checkpoints in source-deleted ordinary CPU players.

Separate canonical CPU deformation tests exercise actual joint/vertex solving.
This process proof observes accepted controls and native render input, not pixels.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
SCRIPT, FIRST, CAMERA, SECOND, CONSTRAINT = [f'{n:026}' for n in (1, 2, 3, 4, 9)]
ENTITIES = (FIRST, SECOND)
TICKS = 40


def require(ok, message):
    if not ok:
        raise ValueError(message)


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def files(root):
    return {str(p.relative_to(root)): {'sha256': digest(p), 'bytes': p.stat().st_size}
            for p in sorted(root.rglob('*')) if p.is_file()}


def transform(at):
    return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


def fixture(root):
    # Reuse only the numerical binary-accessor builder, then install a real
    # two-bone hierarchy. No art or external source assets are used.
    helper = runpy.run_path(str(ROOT / 'tools/probes/native-animation-graph-export.py'))
    helper['fixture'](root)
    path = root / 'rig.gltf'
    model = json.loads(path.read_text())
    model['nodes'] = [{'children': [1]}, {'translation': [1, 0, 0], 'children': [2]},
                      {'translation': [0, 1, 0]}, {'mesh': 0, 'skin': 0}]
    model['scenes'] = [{'nodes': [0, 3]}]
    model['skins'][0].update(joints=[2], skeleton=0)
    write(path, model)


def expected(entity, tick):
    x = 1 if tick == 1 else (1 if entity == FIRST else -1) * (1 + (tick - 1) / 64)
    weight = 1 if tick == 1 else ((tick - 1) % 3) / 2
    return [x, weight, tick]


def observations(start, end, wrong=False):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': list(ENTITIES),
            'checks': [{'name': f'{entity}-{tick}', 'tick': tick, 'expect': {
                'type': 'vector', 'entity': entity, 'property': 'translation',
                'value': [expected(entity, tick)[0] + int(wrong), *expected(entity, tick)[1:]], 'tolerance': 0}}
                for tick in range(start + 1, end + 1) for entity in ENTITIES]}


def check_logs(logs, start, end):
    entries = logs['entries']
    require(len(entries) == 2 * (end - start) and logs['dropped'] == 0, 'Accepted log count')
    for index in range(0, len(entries), 2):
        tick = start + index // 2 + 1
        seen = set()
        for entry in entries[index:index + 2]:
            row = json.loads(entry['message'])
            require(len(row) == 6 and row[0] == entry['tick'] == tick and row[1] in ENTITIES,
                    'Tick/identity pairing')
            require(row[1] not in seen and entry['level'] == 'info', 'Duplicate/log level')
            seen.add(row[1])
            require(row[2:5] == expected(row[1], tick), 'IK numeric snapshot')
            require(row[5] == int(tick >= 4 and (tick - 1) % 3 == 0), 'Timer continuation')


def framing(path):
    value = bytearray(path.read_bytes())
    require(value[:8] == b'INCSES01' and struct.unpack_from('<I', value, 8)[0] == 1, 'Session v1 framing')
    lengths = struct.unpack_from('<4I', value, 80)
    require(len(value) == 128 + sum(lengths), 'Bounded sections')
    require(value[96:128] == hashlib.sha256(value[:96] + value[128:]).digest(), 'Session checksum')
    world = value[128:128 + lengths[0]]
    require(struct.unpack_from('<I', world, 8)[0] == 9, 'IK world v9')
    require(world[32:64] == hashlib.sha256(world[:32] + world[64:]).digest(), 'World checksum')
    return value, lengths


def verify(output, package, authored, exported, removed, original_inputs, provenance):
    import native_process
    player = package / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    package_files = files(package)
    require(not original_inputs.exists() and not list((package / 'symbols').glob('*.js.map')), 'Source-bearing inputs remain')
    env = {'PATH': str(player.parent), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    diagnostics = {
        'corrupt': 'Error: Checkpoint(Integrity)',
        'wrong-program': 'Error: Checkpoint(ProgramMismatch)',
        'wrong-constraint': 'Error: Checkpoint(World(Scene(Invalid("IK state binding"))))',
        'wrong-chain': 'Error: Checkpoint(World(Scene(Invalid("IK state binding"))))',
        'oversized': 'Error: "runtime input size limit"',
        'assertion': 'Error: "checkpoint withheld: native observation assertions failed"',
    }
    runs, refused = [], []

    def launch(name, ticks, start=0, load=None, save=None, success=True, wrong=False):
        plan, pcm = output / (name + '-observe.json'), output / (name + '.pcm')
        write(plan, observations(start, start + ticks, wrong))
        command = [player, '--package', player.parent / 'data', '--ticks', ticks, '--output', pcm, '--observe', plan]
        for flag, value in [('--load-checkpoint', load), ('--save-checkpoint', save)]:
            if value is not None:
                command += [flag, value]
        argv = list(map(str, command))
        result = native_process.run(argv, cwd=player.parent, env=env, timeout=90)
        receipt = {'argv': argv, 'cwd': str(player.parent), 'environment_keys': sorted(env),
                   'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
        write(output / (name + '-receipt.json'), receipt)
        require((result.returncode == 0) == success, name + ': ' + result.stderr[-4000:])
        if not success:
            error = result.stderr.strip()
            matched = error == diagnostics[name] if name in diagnostics else bool(re.fullmatch(
                r'Error: Os \{ code: \d+, kind: AlreadyExists, message: "[^"\n]*" \}', error))
            require(result.returncode == 1 and matched, name + ': wrong refusal diagnostic: ' + error)
            require(name == 'no-clobber' or not pcm.exists(), 'Rejected PCM published')
            require(save is None or name == 'no-clobber' or not save.exists(), 'Rejected save published')
            receipt['matched_diagnostic'] = error
            write(output / (name + '-receipt.json'), receipt)
            refused.append(receipt)
            return
        value = json.loads(result.stdout)
        require(value['tick'] == start + ticks and value['observations']['passed'], 'Tick/observations')
        require(value['render_input']['model_count'] == 2 and not value['render_input']['gpu_submitted'], 'CPU native host')
        check_logs(value['logs'], start, start + ticks)
        framing(save)
        write(output / (name + '.json'), value)
        runs.append({'name': name, 'report': value, 'save_sha256': digest(save), 'pcm_sha256': digest(pcm), 'pcm_bytes': pcm.stat().st_size})
        return value

    baseline, middle, resumed, repeat = [output / (n + '.save') for n in ('baseline', 'middle', 'resumed', 'repeat')]
    full = launch('baseline', TICKS, save=baseline)
    first = launch('middle', 13, save=middle)
    for name, final in [('resumed', resumed), ('repeat', repeat)]:
        continuation = launch(name, TICKS - 13, 13, middle, final)
        require(final.read_bytes() == baseline.read_bytes(), 'Final save continuation differs')
        require((output / 'baseline.pcm').read_bytes() == (output / 'middle.pcm').read_bytes() + (output / (name + '.pcm')).read_bytes(), 'PCM clock continuation differs')
        require(first['logs']['entries'] + continuation['logs']['entries'] == full['logs']['entries'], 'Timer/state/IK log continuation differs')
        require(continuation['render_input'] == full['render_input'], 'Final retained input differs')
    require(full['logs'] == authored['native_play_logs'], 'Native Play and packaged player disagree')
    content, lengths = framing(middle)
    for name, offset in [('corrupt', len(content) - 1), ('wrong-program', 16),
                         ('wrong-constraint', 128 + lengths[0] - 94 + 16),
                         ('wrong-chain', 128 + lengths[0] - 94 + 32)]:
        bad = content.copy()
        if name == 'wrong-chain':
            # Distinct, bounded node IDs but a different chain than cooked data.
            bad[offset:offset + 6] = struct.pack('<3H', 0, 2, 1)
        else:
            bad[offset] ^= 1
        if name.startswith('wrong-'):
            if name != 'wrong-program':
                world = bad[128:128 + lengths[0]]
                bad[160:192] = hashlib.sha256(world[:32] + world[64:]).digest()
            bad[96:128] = hashlib.sha256(bad[:96] + bad[128:]).digest()
        path = output / (name + '.save'); path.write_bytes(bad)
        launch(name, 1, 13, path, success=False)
    oversized = output / 'oversized.save'
    with oversized.open('wb') as file:
        file.truncate(256 * 1024 * 1024 + 1)
    launch('oversized', 1, load=oversized, success=False)
    launch('assertion', 1, 13, middle, output / 'assertion.save', success=False, wrong=True)
    sentinel = output / 'existing.save'; sentinel.write_bytes(b'preserve existing')
    launch('no-clobber', 1, 13, middle, sentinel, success=False)
    require(sentinel.read_bytes() == b'preserve existing' and not list(output.glob('.incant-checkpoint-*')), 'Create-only save publication')
    require(files(package) == package_files, 'Package changed during controls')
    write(output / 'result.json', {'passed': True, 'provenance': provenance, 'authoring': authored, 'export': exported,
        'package_files': package_files, 'player_sha256': digest(player), 'player_bytes': player.stat().st_size,
        'runs': runs, 'refusals': refused, 'authoring_deleted': True, 'removed_source_maps': removed,
        'limits': ['CPU canonical input/control/checkpoint proof; deformation is separately exercised by actual CPU solver tests',
            'No GPU/window/appearance/device/performance acceptance', 'Ordinary load saves opt-in typed script state/timers, not arbitrary VM globals',
            'IK Rust/Wasm command ABI remains a follow-up; existing unsupported solver/module saves remain explicit',
            'PCM and create-only checkpoint publication are separate operations']})
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--existing-package', type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    if args.existing_package:
        require(args.binary is None and not args.author, 'Existing package cannot author')
        output.mkdir(parents=True, exist_ok=False)
        package = args.existing_package.resolve(strict=True)
        prior = json.loads((package.parent / 'package-preparation.json').read_text())
        verify(output, package, prior['authoring'], prior['export'], prior['removed_source_maps'],
               package.parent / 'authoring', {'mode': 'existing-package', 'preparation_sha256': digest(package.parent / 'package-preparation.json'), 'helper_sha256': digest(Path(__file__))})
        return
    require(args.binary is not None, 'Fresh proof requires --binary')
    binary = args.binary.resolve(strict=True)
    inputs = output / 'authoring'; project, journal = inputs / 'project.json', inputs / 'project.journal.jsonl'
    receipts = []

    def execute(command, **kwargs):
        argv = list(map(str, command)); result = native_process.run(argv, **kwargs)
        receipts.append({'argv': argv, 'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        write(output / ('author-child-processes.json' if args.author else 'processes.json'), receipts)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    if args.author:
        fixture(inputs)
        imported = json.loads(execute([binary, 'import', project, 'rig.gltf']).stdout)
        document = json.loads(project.read_text()); scene = next(iter(document['scenes']))
        require(document['settings']['tick_rate'] == 60, 'Default fixed rate')
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        source = (ROOT / 'tools/probes/native-animation-ik-export/behavior.ts').read_text()
        rpc = Rpc(binary, project, journal); calls = []

        def call(method, params=None):
            result = rpc.call(method, params) if params is not None else rpc.call(method)
            calls.append({'method': method, 'params': params, 'result': result}); return result

        try:
            revision = call('project.read')['revision']
            commands = [{'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                        {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Authored IK', 'path': 'ik.ts'}}]
            for entity in ENTITIES:
                commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {'id': entity, 'name': 'IK instance',
                    'parent': None, 'provenance': None, 'components': {'Transform': transform([0, 0, 0]),
                    'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': False},
                    'Animation': {'clip': 0 if entity == FIRST else 1, 'seconds': 0, 'speed': 1, 'playing': True, 'looping': True},
                    'AnimationIk': {'constraints': [{'id': CONSTRAINT, 'chain': [0, 1, 2], 'target': [1, 1, 0], 'pole': [0, 1, 0], 'weight': 1}]},
                    'Script': {'source': SCRIPT, 'props': {'first': entity == FIRST}}}}})
            commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {'id': CAMERA, 'name': 'Camera', 'parent': None,
                'provenance': None, 'components': {'Transform': transform([0, 0, 8]), 'Camera': {'fov_degrees': 60, 'near': .1,
                    'far': 200, 'projection': {'kind': 'perspective'}}}}})
            created = call('command.execute', {'expected_revision': revision, 'description': 'Author connected IK proof', 'commands': commands})
            require(created['revision'] == revision + 1, 'Atomic authored transaction')
            call('history.undo'); require(call('project.read')['project'] == document and not (inputs / 'ik.ts').exists(), 'Undo failed')
            call('history.redo'); authored = call('project.read')
            played = call('play.run', {'seconds': TICKS / 60, 'camera': CAMERA, 'observe': observations(0, TICKS)})
            write(output / 'native-play.json', played)
            require(played['passed'] and not played['state']['gpu_submitted'] and played['state']['cpu_render_input'], 'Native CPU Play')
            check_logs(played['state']['logs'], 0, TICKS)
            require(call('project.read') == authored, 'Play mutated authoring')
            require(call('project.save')['saved'], 'Save failed')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(call('project.read') == authored, 'Journal reopen failed')
            require(call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == source, 'Source pairing')
        finally:
            rpc.close()
        write(output / 'public-commands.json', calls)
        write(output / 'authoring.json', {'command_bus_only': True, 'undo_redo_reopen': True,
            'project_sha256': digest(project), 'source_sha256': hashlib.sha256(source.encode()).hexdigest(),
            'binary_sha256': digest(binary), 'files': files(inputs), 'native_play_logs': played['state']['logs']})
        return
    output.mkdir(parents=True, exist_ok=False); inputs.mkdir()
    execute([binary, 'init', project, '--entities', '0'])
    execute([sys.executable, Path(__file__).resolve(), output, '--binary', binary, '--author'], timeout=240)
    authored = json.loads((output / 'authoring.json').read_text())
    execute([sys.executable, ROOT / 'tools/export_native.py', project, output / 'package', '--camera', CAMERA], timeout=1800)
    package = output / 'package'; exported = json.loads((package / 'export.json').read_text())
    require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Trusted export identity')
    removed = {}
    for path in sorted((package / 'symbols').glob('*.js.map')):
        require(path.is_file() and not path.is_symlink(), 'Unexpected debug-map artifact')
        removed[str(path.relative_to(package))] = digest(path); path.unlink()
    shutil.rmtree(inputs)  # Only this helper's newly created disposable fixture.
    write(output / 'package-preparation.json', {'authoring': authored, 'export': exported, 'removed_source_maps': removed, 'package_files': files(package)})
    verify(output, package, authored, exported, removed, inputs,
           {'mode': 'fresh-public-export', 'helper_sha256': digest(Path(__file__)),
            'behavior_sha256': digest(ROOT / 'tools/probes/native-animation-ik-export/behavior.ts')})


if __name__ == '__main__':
    main()
