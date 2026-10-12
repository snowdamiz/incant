#!/usr/bin/env python3
"""Public authored source/bindings/data preflight before an expensive Android build."""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import struct
import sys
import wave

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process

TICKS = 12
ID = lambda n: f'{n:026d}'
ACTORS = [(ID(11), ID(1), 'first 語', 3, 0), (ID(13), ID(1), 'second é', 1.5, -3),
          (ID(12), ID(2), 'opposite 🙂', 2, 10)]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def files(directory):
    return {str(p.relative_to(directory)): digest(p) for p in sorted(directory.rglob('*')) if p.is_file()}


def bits(value):
    return struct.pack('>d', value).hex()


def oracle(released):
    positions = {entity: float(x) for entity, _, _, _, x in ACTORS}
    velocities = {entity: 0. for entity, _, _, _, _ in ACTORS}
    logs, checks = [], []
    for tick in range(1, TICKS + 1):
        active = not released or tick < 4 or tick >= 7
        for entity, source, label, speed, _ in ACTORS:
            # NativeSession advances the world using previously committed
            # velocities, then the script atomically publishes this tick's input.
            positions[entity] += velocities[entity] * (1 / 60)
            velocity = speed * (1 if source == ID(1) else -1) if active else 0.
            logs.append({'tick': tick, 'level': 'info', 'message': ';'.join([
                str(tick), label, bits(positions[entity]), bits(velocity), bits(1 / 60),
                '18446744073709551617', bits(struct.unpack('<f', struct.pack('<f', 1 / 3))[0])])})
            velocities[entity] = velocity
            for prop, value in [('translation', positions[entity]), ('velocity', velocity)]:
                checks.append({'name': f'{tick}-{entity}-{prop}', 'tick': tick, 'expect': {
                    'type': 'vector', 'entity': entity, 'property': prop, 'value': [value, 0, 0], 'tolerance': 1e-12}})
    return {'logs': {'entries': logs, 'dropped': 0}, 'positions': positions,
            'observations': {'format': 'incant-native-observation', 'version': 1,
                             'entities': [actor[0] for actor in ACTORS], 'checks': checks}}


def recording(released=False, failure=False):
    frames = [{'tick': 1, 'events': [{'type': 'key', 'code': 'KeyD', 'down': True}]}]
    if released:
        frames += [{'tick': t, 'events': [{'type': 'key', 'code': 'KeyD', 'down': down}]}
                   for t, down in [(4, False), (7, True)]]
    if failure:
        frames.append({'tick': 4, 'events': [{'type': 'key', 'code': 'KeyF', 'down': True}]})
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0, 'ticks': TICKS, 'frames': frames}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--play-binary', type=Path, required=True)
    parser.add_argument('--physics-mode', choices=['deterministic', 'fast'], required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    authoring = out / 'authoring'
    authoring.mkdir()
    project, journal = authoring / 'game.json', authoring / 'game.journal.jsonl'
    binary = args.binary.resolve(strict=True)
    play_binary = args.play_binary.resolve(strict=True)
    processes, calls = [], []

    def execute(argv):
        argv = list(map(str, argv))
        r = native_process.run(argv, cwd=ROOT, timeout=180)
        record = {'argv': argv, 'exit_code': r.returncode, 'stdout': r.stdout, 'stderr': r.stderr}
        write(out / f'process-{len(processes):02d}.json', record)
        processes.append(record)
        if r.returncode:
            raise RuntimeError(r.stderr)
        return json.loads(r.stdout)

    execute([binary, 'init', project, '--entities', 0, '--name', 'Authored Android CPU project'])
    with wave.open(str(authoring / 'tone.wav'), 'wb') as stream:
        stream.setnchannels(1)
        stream.setsampwidth(2)
        stream.setframerate(48000)
        stream.writeframes(b''.join(struct.pack('<h', 3000 if n % 48 < 24 else -1500) for n in range(4800)))
    clip = execute([binary, 'import', project, 'tone.wav'])['asset']['id']
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    rpc = Rpc(binary, project, journal)

    def call(method, params=None):
        record = {'method': method, 'params': params}
        calls.append(record)
        write(out / 'authoring-calls.json', calls)
        try:
            result = rpc.call(method, params)
            record['result'] = result
        except Exception as error:
            record['error'] = str(error)
            raise
        finally:
            write(out / 'authoring-calls.json', calls)
        return result

    try:
        before = call('project.read')
        scene = next(iter(before['project']['scenes']))
        commands = []
        for n, name in [(1, 'mover'), (2, 'other'), (3, 'shared')]:
            source_commands = [{'op': 'write_script', 'script_id': ID(n), 'expected_source': {'state': 'missing'},
                          'source': (HERE / (name + '.ts')).read_text()},
                         {'op': 'upsert_script', 'script': {'id': ID(n), 'name': name, 'path': name + '.ts'}}]
            # Public source transactions intentionally admit one source write.
            current = call('project.read')
            call('command.execute', {'expected_revision': current['revision'],
                                    'description': 'Author Android source ' + name, 'commands': source_commands})
        commands += [{'op': 'set_physics_mode', 'mode': args.physics_mode}]
        commands += [{'op': 'set_input_actions', 'actions': {
            name: {'kind': 'button', 'bindings': [{'type': 'key', 'code': key}]}
            for name, key in [('right', 'KeyD'), ('fail', 'KeyF')]}}]
        for entity, source, label, speed, x in ACTORS:
            commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': entity, 'name': label, 'parent': None, 'provenance': None,
                'components': {'Transform': {'translation': [x, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                               'Velocity': {'linear': [0, 0, 0]}, 'Script': {'source': source, 'props': {'speed': speed, 'label': label}}}}})
        commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': ID(21), 'name': 'Captured tone', 'parent': None, 'provenance': None,
            'components': {'AudioSource': {'clip': clip, 'gain_db': -6, 'pan': 0, 'rate': 1,
                                           'looping': True, 'playing': True, 'streaming': False, 'start_seconds': 0}}}})
        current = call('project.read')
        call('command.execute', {'expected_revision': current['revision'], 'description': 'Attach bound Android project', 'commands': commands})
        authored = call('project.read')
        for _ in range(4):
            call('history.undo')
        assert call('project.read')['project'] == before['project']
        assert not any((authoring / (name + '.ts')).exists() for name in ('mover', 'other', 'shared'))
        for _ in range(4):
            call('history.redo')
        assert call('project.read')['project'] == authored['project']
        call('project.save')
        authored = call('project.read')
    finally:
        rpc.close()
    rpc = Rpc(binary, project, journal)
    try:
        assert call('project.read') == authored
    finally:
        rpc.close()
    authoring_hashes = files(authoring)
    results = {}
    for name, released in [('held', False), ('released', True), ('repeat', False)]:
        expected = oracle(released)
        write(out / (name + '-expected.json'), expected)
        write(out / (name + '-input.json'), recording(released))
        write(out / (name + '-observe.json'), expected['observations'])
        report = execute([play_binary, 'native-play', project, '--ticks', TICKS, '--input-replay', out / (name + '-input.json'),
                          '--observe', out / (name + '-observe.json'), '--pcm-output', out / (name + '.pcm')])
        assert report['passed'] and report['script_source_parsing'] and not report['gpu_submitted']
        assert report['logs'] == expected['logs'], name
        assert report['frames'] == TICKS * 800
        for entity, x in expected['positions'].items():
            assert report['observations']['entities'][entity]['translation'] == [x, 0, 0]
        pcm = (out / (name + '.pcm')).read_bytes()
        assert len(pcm) == TICKS * 800 * 8 and any(pcm)
        assert hashlib.sha256(pcm).hexdigest() == report['pcm_sha256']
        write(out / (name + '-native.json'), report)
        results[name] = report
    assert results['held'] == results['repeat']
    assert (out / 'held.pcm').read_bytes() == (out / 'released.pcm').read_bytes()
    write(out / 'failure-input.json', recording(failure=True))
    assert files(authoring) == authoring_hashes
    write(out / 'result.json', {'passed': True, 'project': str(project), 'scene': scene,
                               'headless_sha256': digest(binary), 'play_binary_sha256': digest(play_binary),
                               'physics_mode': args.physics_mode, 'authoring_undo_redo_reopen': True,
                               'authoring_unchanged': authoring_hashes, 'processes': len(processes),
                               'runs': list(results), 'checks_per_run': len(oracle(False)['observations']['checks']),
                               'sources': {p.name: digest(p) for p in HERE.iterdir() if p.is_file()}})
    print(json.dumps({'passed': True, 'output': str(out), 'runs': list(results)}))


if __name__ == '__main__':
    main()
