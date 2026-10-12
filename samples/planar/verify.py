#!/usr/bin/env python3
"""Run numerical tilemap/sprite/2D-physics gameplay through public native Play.

The white atlas is a test payload, not sample art. All authored project/script
changes use the command bus. No GPU, device, persistence or visual gate is claimed.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import runpy
import shutil
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parents[2]
PRESENTATION = runpy.run_path(str(ROOT / 'samples/planar/presentation.py'))
ID = lambda n: f'{n:026d}'
SCRIPT, PLAYER, MAP, EXIT, HAZARD, CONTROLLER, CAMERA, FRAME, ATLAS, LAYER = map(ID, range(1, 11))
PICKUPS = list(map(ID, (11, 12)))
NAMES = ('win', 'gamepad', 'lose', 'wall', 'locked-exit', 'diagonal', 'repeat-win')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def fixture_image(path):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>2I5B', 1, 1, 8, 6, 0, 0, 0))
                     + chunk(b'IDAT', zlib.compress(b'\x00\xff\xff\xff\xff')) + chunk(b'IEND', b''))


def transform(at):
    return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


def sensor(radius, enabled=True):
    return {'shape': {'kind': 'circle', 'radius': radius}, 'density': 1000,
            'friction': 0, 'restitution': 0, 'sensor': enabled, 'memberships': 4294967295, 'filter': 4294967295}


def room(scene, texture):
    cells = [FRAME if x in (0, 13) or y in (0, 9) or (x == 6 and y != 4) else None
             for y in range(10) for x in range(14)]
    sprite = {'texture': texture, 'region': {'frame': FRAME}, 'size': [.6, .6], 'pivot': [.5, .5], 'tint': [1, 1, 1, 1]}
    specs = [
        (PLAYER, 'Player', {'Transform': transform([2.5, 2.5, 0]), 'Velocity': {'linear': [0, 0, 0]},
            'RigidBody2D': {'motion': 'dynamic', 'gravity_scale': 0, 'linear_damping': 0, 'angular_damping': 0,
                          'can_sleep': True, 'ccd': True}, 'Collider2D': sensor(.3, False), 'SpriteRenderer': sprite}),
        (MAP, 'Solid tiles', {'Transform': transform([0, 0, 0]), 'Tilemap': {'tileset': texture,
            'size': [14, 10], 'cell_size': [1, 1], 'layers': [{'id': LAYER, 'cells': cells, 'solid': True}]}}),
        (EXIT, 'Exit sensor', {'Transform': transform([11.5, 4.5, 0]), 'Collider2D': sensor(.4), 'SpriteRenderer': sprite}),
        (HAZARD, 'Hazard sensor', {'Transform': transform([2.5, 7.5, 0]), 'Collider2D': sensor(.4), 'SpriteRenderer': sprite}),
        (CAMERA, 'Camera', {'Transform': transform([7, 5, 20]), 'Camera': {
            'fov_degrees': 60, 'near': .1, 'far': 100, 'projection': {'kind': 'orthographic', 'vertical_size': 12}}}),
        (CONTROLLER, 'Planar gameplay', {'Transform': transform([0, 0, 0]), 'Script': {'source': SCRIPT,
            'props': {'player': PLAYER, 'exit': EXIT, 'pickups': PICKUPS, 'hazards': [HAZARD], 'speed': 3}}}),
    ]
    for entity, at in zip(PICKUPS, ([4.5, 2.5, 0], [9.5, 4.5, 0])):
        specs.append((entity, 'Pickup sensor', {'Transform': transform(at), 'Collider2D': sensor(.3), 'SpriteRenderer': sprite}))
    return [{'op': 'create_entity', 'scene_id': scene, 'entity': {
        'id': entity, 'name': name, 'parent': None, 'provenance': None, 'components': components}}
        for entity, name, components in specs]


def replay(name):
    moves = {
        'win': [(1, 1, 0), (61, 0, 1), (101, 1, 0), (241, 0, 0)],
        'lose': [(1, 0, 1)], 'wall': [(1, 1, 0)],
        'locked-exit': [(1, 0, 1), (41, 1, 0), (221, 0, 0)],
        'diagonal': [(1, 1, 1), (31, 0, 0)],
    }['win' if name in ('gamepad', 'repeat-win') else name]
    frames = []
    for tick, x, y in moves:
        if name == 'gamepad':
            events = ([{'type': 'gamepad_connected', 'id': 0}] if tick == 1 else []) + [
                {'type': 'gamepad_axis', 'id': 0, 'axis': axis, 'value': value}
                for axis, value in [('left_x', x), ('left_y', -y)]]
        else:
            events = [{'type': 'key', 'code': key, 'down': bool(value)} for key, value in
                      [('KeyD', x > 0), ('KeyA', x < 0), ('KeyW', y > 0), ('KeyS', y < 0)]]
        frames.append({'tick': tick, 'events': events})
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0,
            'ticks': 300 if name != 'diagonal' else 60, 'frames': frames}


def observations(name, ticks):
    won = name in ('win', 'gamepad', 'repeat-win')
    removed = PICKUPS if won else [PICKUPS[0]] if name == 'wall' else [PICKUPS[1]] if name == 'locked-exit' else []
    checks = [{'name': 'pickup-' + entity, 'tick': ticks, 'expect': {
        'type': 'exists', 'entity': entity, 'value': entity not in removed}} for entity in PICKUPS]
    if won or name in ('lose', 'diagonal'):
        checks.append({'name': 'stopped', 'tick': ticks, 'expect': {
            'type': 'vector', 'entity': PLAYER, 'property': 'velocity', 'value': [0, 0, 0], 'tolerance': 0}})
    return {'format': 'incant-native-observation', 'version': 1, 'entities': [PLAYER], 'checks': checks}


def verify_reports(reports):
    for name, report in reports.items():
        assert report['passed'] and report['runtime'] == 'native' and report['observations']['passed']
        assert report['cpu_render_input'] and not report['gpu_submitted'] and report['script_source_parsing']
        logs = report['logs']; assert not logs['dropped']
        messages = [entry['message'] for entry in logs['entries']]
        picked = [message.split()[1] for message in messages if message.startswith('planar.pickup')]
        assert len(picked) == len(set(picked)), 'Duplicate sensor reward'
        wins = sum(message.startswith('planar.won') for message in messages)
        losses = sum(message.startswith('planar.lost') for message in messages)
        at = report['observations']['entities'][PLAYER]['translation']
        assert abs(at[2]) < 1e-9, 'Planar body escaped XY'
        if name in ('win', 'gamepad', 'repeat-win'):
            assert picked == PICKUPS and wins == 1 and losses == 0
            assert 10.7 < at[0] < 11.2 and abs(at[1] - 4.5) < .01, at
        elif name == 'lose':
            assert losses == 1 and wins == 0 and not picked
            assert abs(at[0] - 2.5) < .01 and 6.7 < at[1] < 7.1, at
        else:
            assert wins == losses == 0
        if name == 'wall': assert 5.65 < at[0] < 5.75 and abs(at[1] - 2.5) < .01, at
        if name == 'locked-exit': assert abs(at[0] - 11.5) < .01 and abs(at[1] - 4.5) < .01, at
        if name == 'diagonal': assert abs(math.hypot(at[0] - 2.5, at[1] - 2.5) - 1.5) < .01, at
    for name in ('gamepad', 'repeat-win'):
        for key in ('logs', 'observations', 'pcm_sha256', 'entity_count'): assert reports[name][key] == reports['win'][key], (name, key)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--presentation', action='store_true', help='Add optional numerical state-frame/HUD roles')
    parser.add_argument('output', type=Path); parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args(); out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    inputs = out / 'authoring'; inputs.mkdir(); binary = args.binary.resolve(strict=True); binary_sha = digest(binary)
    project = inputs / 'planar.json'; journal = inputs / 'planar.journal.jsonl'; processes = []

    def execute(argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=180)
        write(out / f'process-{len(processes):02d}.json', {'argv': list(map(str, argv)), 'exit': result.returncode,
              'stdout': result.stdout, 'stderr': result.stderr}); processes.append(result.returncode)
        if result.returncode: raise RuntimeError(result.stderr[-8000:])
        return json.loads(result.stdout)

    execute([binary, 'init', project, '--entities', 0, '--name', 'Planar numerical sample'])
    fixture_image(inputs / 'white.png')
    write(inputs / 'tiles.atlas.json', {'version': 1, 'id': ATLAS, 'maximum_size': [8, 8], 'padding': 1,
                                      'frames': {FRAME: {'name': 'Numerical tile', 'path': 'white.png'}}})
    texture = execute([binary, 'import', project, 'tiles.atlas.json'])['asset']['id']
    font = None
    if args.presentation:
        shutil.copyfile(ROOT / 'crates/incant_text/tests/fonts/NotoSans-Regular.ttf', inputs / 'font.ttf')
        font = execute([binary, 'import', project, 'font.ttf'])['asset']['id']
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    rpc = Rpc(binary, project, journal); calls = []

    def call(method, params=None):
        value = rpc.call(method, params); calls.append({'method': method, 'params': params, 'result': value})
        write(out / 'authoring-calls.json', calls); return value

    try:
        before = call('project.read'); scene = next(iter(before['project']['scenes']))
        entities = room(scene, texture)
        source = (ROOT / 'samples/planar/gameplay.ts').read_text()
        if args.presentation:
            source = PRESENTATION['observed_source'](source)
            controller = next(c['entity'] for c in entities if c['entity']['id'] == CONTROLLER)
            controller['components']['Script']['props'].update(PRESENTATION['props'](globals()))
            entities += PRESENTATION['commands'](scene, font, globals())
        call('command.execute', {'expected_revision': before['revision'], 'description': 'Author numerical 2D sample', 'commands': [
            {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'},
             'source': source},
            {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Planar Sample', 'path': 'gameplay.ts'}},
            {'op': 'set_input_actions', 'actions': {'move': {'kind': 'axis2', 'dead_zone': .1, 'bindings': [
                {'type': 'key', 'code': key, 'scale': value} for key, value in
                [('KeyD', [1, 0]), ('KeyA', [-1, 0]), ('KeyW', [0, 1]), ('KeyS', [0, -1])]] + [
                    {'type': 'gamepad_stick', 'id': 0, 'stick': 'left', 'invert_y': True}]}}}, *entities]})
        authored = call('project.read'); call('history.undo')
        assert call('project.read')['project'] == before['project'] and not (inputs / 'gameplay.ts').exists()
        call('history.redo'); assert call('project.read')['project'] == authored['project']
        call('project.save'); authored = call('project.read')
    finally: rpc.close()
    rpc = Rpc(binary, project, journal)
    try: assert call('project.read') == authored
    finally: rpc.close()
    hashes = {p.name: digest(p) for p in inputs.iterdir() if p.is_file()}; results = {}
    for name in NAMES:
        timeline = replay(name); replay_path = out / f'{name}-input.json'; write(replay_path, timeline)
        plan = out / f'{name}-observe.json'; checks = observations(name, timeline['ticks'])
        if args.presentation: PRESENTATION['extend_observations'](checks)
        write(plan, checks)
        report = execute([binary, 'native-play', project, '--camera', CAMERA, '--ticks', timeline['ticks'],
                          '--input-replay', replay_path, '--observe', plan,
                          *(['--ui-viewport', PRESENTATION['VIEWPORT']] if args.presentation else [])])
        write(out / f'{name}.json', report); results[name] = report
    verify_reports(results)
    presentation = {name: PRESENTATION['verify_report'](name, report, globals()) for name, report in results.items()} if args.presentation else None
    assert digest(binary) == binary_sha and {p.name: digest(p) for p in inputs.iterdir() if p.is_file()} == hashes
    refusals = PRESENTATION['refusal_controls'](binary, out, out / 'refusals', globals()) if args.presentation else []
    write(out / 'result.json', {'passed': True, 'binary_sha256': binary_sha,
        'gameplay_sha256': digest(ROOT / 'samples/planar/gameplay.ts'), 'authoring_undo_redo_reopen': True,
        'authoring_unchanged': True, 'presentation': presentation, 'refusals': [r['name'] for r in refusals], 'authored_source_sha256': digest(inputs / 'gameplay.ts'), 'project_sha256': digest(project), 'runs': list(results), 'limits': ['Numerical CPU fixture', 'No visual/device/physics save gate']})
    print(json.dumps({'passed': True, 'runs': len(results), 'output': str(out)}))


if __name__ == '__main__': main()
