#!/usr/bin/env python3
"""Exercise Core Sample gameplay against the real native runtime, without art.

The collider-only room is a numerical test fixture, not the finished sample level.
All authoring and script writes use the ordinary command bus. Gameplay uses no
legacy document-backed Play route. No renderer/device/performance claim.
"""
import argparse
import hashlib
import json
import runpy
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ID = lambda n: f'{n:026d}'
SCRIPT, PLAYER, FLOOR, WALL, EXIT, NAV, CONTROLLER = map(ID, range(1, 8))
ENEMIES = list(map(ID, (10, 11)))
PICKUPS = list(map(ID, (12, 13)))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def transform(at):
    return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


def collider(shape, sensor=False):
    return {'shape': shape, 'density': 1000, 'friction': .5, 'restitution': 0,
            'sensor': sensor, 'memberships': 4294967295, 'filter': 4294967295}


def room(scene):
    body = {'motion': 'kinematic', 'gravity_scale': 0, 'linear_damping': 0,
            'angular_damping': 0, 'can_sleep': True, 'ccd': True}
    capsule = collider({'type': 'capsule', 'half_height': .3, 'radius': .3})
    specs = [
        (PLAYER, 'Player', {'Transform': transform([-5, .62, 0]), 'Velocity': {'linear': [0, 0, 0]},
                            'RigidBody': body, 'Collider': capsule}),
        (FLOOR, 'Collision floor', {'Transform': transform([0, -.5, 0]),
                                  'Collider': collider({'type': 'box', 'half_extents': [10, .5, 7]})}),
        (WALL, 'Route obstacle', {'Transform': transform([0, 1.5, 3]),
                                'Collider': collider({'type': 'box', 'half_extents': [1, 1.5, 1.4]})}),
        (EXIT, 'Exit', {'Transform': transform([5, .62, 0])}),
        (NAV, 'Navigation', {'NavigationMesh': {
            'settings': {'min': [-10, -2, -7], 'max': [10, 5, 7], 'cell_size': .25,
                         'cell_height': .1, 'tile_cells': 16, 'agent_radius': .35,
                         'agent_height': 1.2, 'max_climb': .3, 'max_slope_degrees': 45},
            'sources': [{'entity': n, 'geometry': 'collider'} for n in [FLOOR, WALL]]}}),
        (CONTROLLER, 'Core Sample gameplay', {'Transform': transform([0, 0, 0]),
            'Script': {'source': SCRIPT, 'props': {'player': PLAYER, 'navigation': NAV, 'exit': EXIT,
                'enemies': ENEMIES, 'pickups': PICKUPS, 'jumpSpeed': 8}}}),
    ]
    for entity, at in zip(ENEMIES, [[5, .62, 3], [4, .62, 4]]):
        specs.append((entity, 'Chaser', {'Transform': transform(at), 'Velocity': {'linear': [0, 0, 0]},
                                       'RigidBody': body, 'Collider': capsule}))
    for entity, x in zip(PICKUPS, [-2, 2]):
        specs.append((entity, 'Pickup', {'Transform': transform([x, .62, 0]),
            'Collider': collider({'type': 'sphere', 'radius': .35}, True)}))
    return [{'op': 'create_entity', 'scene_id': scene, 'entity': {
        'id': entity, 'name': name, 'parent': None, 'provenance': None, 'components': components}}
        for entity, name, components in specs]


def replay(name):
    key = lambda code, down: {'type': 'key', 'code': code, 'down': down}
    frames = []
    ticks = 900 if name == 'lose' else 210 if name == 'wall' else 300
    if name in ('win', 'jump'):
        frames = [{'tick': 1, 'events': [key('KeyD', True)]},
                  {'tick': 250, 'events': [key('KeyD', False)]}]
    if name == 'jump':
        frames.extend([{'tick': 5, 'events': [key('Space', True)]},
                       {'tick': 6, 'events': [key('Space', False)]}])
    if name == 'skip-pickup':
        # A higher authored jump clears pickup one; the exit must remain locked.
        frames = [{'tick': 1, 'events': [key('KeyD', True)]},
                  {'tick': 35, 'events': [key('Space', True)]},
                  {'tick': 36, 'events': [key('Space', False)]},
                  {'tick': 250, 'events': [key('KeyD', False)]}]
    if name == 'wall':
        frames = [{'tick': 1, 'events': [key('KeyD', True)]},
                  {'tick': 60, 'events': [key('KeyD', False), key('KeyS', True)]},
                  {'tick': 120, 'events': [key('KeyS', False), key('KeyD', True)]}]
    if name == 'gamepad':
        frames = [{'tick': 1, 'events': [{'type': 'gamepad_connected', 'id': 0},
                  {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': 1}]},
                  {'tick': 250, 'events': [{'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': 0}]}]
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0,
            'ticks': ticks, 'frames': sorted(frames, key=lambda frame: frame['tick'])}


def observations(name, ticks):
    won = name in ('win', 'repeat-win', 'gamepad', 'jump')
    checks = [{'name': 'collected-entities-removed', 'tick': ticks,
               'expect': {'type': 'entity_count', 'value': 8 if won else 10 if name == 'lose' else 9}}]
    removed = PICKUPS if won else [] if name == 'lose' else [PICKUPS[1] if name == 'skip-pickup' else PICKUPS[0]]
    for entity in PICKUPS:
        checks.append({'name': 'pickup-' + entity, 'tick': ticks,
                       'expect': {'type': 'exists', 'entity': entity, 'value': entity not in removed}})
    if won or name == 'lose':
        for tick in ([200, ticks] if won else [500, ticks]):
            for entity in [PLAYER, *ENEMIES]:
                checks.append({'name': f'terminal-stop-{entity}-{tick}', 'tick': tick, 'expect': {
                    'type': 'vector', 'entity': entity, 'property': 'velocity', 'value': [0, 0, 0], 'tolerance': 0}})
    if name == 'jump':
        checks.append({'name': 'jump-clears-ground', 'tick': 20, 'expect': {
            'type': 'vector', 'entity': PLAYER, 'property': 'physics_translation',
            'value': [-4, 2, 0], 'tolerance': .4}})
    return {'format': 'incant-native-observation', 'version': 1,
            'entities': [PLAYER, *ENEMIES], 'checks': checks}


def verify_reports(reports):
    for name, report in reports.items():
        assert report['passed'] and report['runtime'] == 'native'
        # native-play compiles source for development; shipping is verified separately.
        assert report['script_source_parsing'] and not report['gpu_submitted']
        assert report['observations']['passed'] and not report['logs']['dropped']
        events = report['logs']['entries']; messages = [event['message'] for event in events]
        routes = [message for message in messages if message.startswith('core.route')]
        assert len(routes) == 2 and all(int(message.rsplit('=', 1)[1]) >= 3 for message in routes)
        picked = [message.split()[1] for message in messages if message.startswith('core.pickup')]
        assert len(picked) == len(set(picked)), 'A sensor granted the same pickup twice'
        damage = [event for event in events if event['message'].startswith('core.damage')]
        assert all(b['tick'] - a['tick'] >= 45 for a, b in zip(damage, damage[1:])), 'Invulnerability was bypassed'
        if name in ('win', 'repeat-win', 'gamepad', 'jump'):
            assert picked == PICKUPS
            assert sum(message.startswith('core.won') for message in messages) == 1
            assert not any(message.startswith('core.lost') for message in messages)
            p = report['observations']['entities'][PLAYER]['translation']
            assert 4 <= p[0] <= 6 and .55 <= p[1] <= .7 and abs(p[2]) < .5, p
        if name == 'lose':
            assert not picked and [event['message'] for event in damage] == [
                'core.damage health=2', 'core.damage health=1', 'core.damage health=0']
            assert sum(message.startswith('core.lost') for message in messages) == 1
        if name in ('lose', 'skip-pickup', 'wall'):
            assert not any(message.startswith('core.won') for message in messages), 'Exit accepted missing pickups'
        if name == 'jump': assert messages.count('core.jump 1') == 1
        if name == 'wall':
            p = report['observations']['entities'][PLAYER]['translation']
            assert -1.5 < p[0] < -1.29 and 2.5 < p[2] < 3.5, p
    for name in ('repeat-win', 'gamepad'):
        for field in ('logs', 'observations', 'pcm_sha256', 'entity_count'):
            assert reports[name][field] == reports['win'][field], (name, field)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    inputs = out / 'authoring'; inputs.mkdir()
    project, journal = inputs / 'core.json', inputs / 'core.journal.jsonl'
    binary = args.binary.resolve(strict=True)
    binary_sha = digest(binary)
    processes = []

    def execute(argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, text=True, capture_output=True, timeout=180)
        index = len(processes)
        write(out / f'process-{index:02d}.json', {'argv': list(map(str, argv)), 'exit': result.returncode,
                                               'stdout': result.stdout, 'stderr': result.stderr})
        processes.append(result.returncode)
        if result.returncode: raise RuntimeError(result.stderr[-8000:])
        return json.loads(result.stdout)

    execute([binary, 'init', project, '--entities', '0', '--name', 'Core Sample gameplay test'])
    document = json.loads(project.read_text()); scene = next(iter(document['scenes']))
    rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc'](binary, project, journal)
    source = (ROOT / 'samples/core/gameplay.ts').read_text()
    calls = []

    def call(method, params=None):
        result = rpc.call(method, params)
        calls.append({'method': method, 'params': params, 'result': result})
        write(out / 'authoring-calls.json', calls)
        return result

    try:
        commands = [
            {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
            {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Core Sample', 'path': 'gameplay.ts'}},
            {'op': 'set_input_actions', 'actions': {
                'move': {'kind': 'axis2', 'dead_zone': .1, 'bindings': [
                    {'type': 'key', 'code': code, 'scale': value} for code, value in
                    [('KeyD', [1, 0]), ('KeyA', [-1, 0]), ('KeyW', [0, -1]), ('KeyS', [0, 1])]] +
                    [{'type': 'gamepad_stick', 'id': 0, 'stick': 'left', 'invert_y': False}]},
                'jump': {'kind': 'button', 'bindings': [{'type': 'key', 'code': 'Space'},
                    {'type': 'gamepad_button', 'id': 0, 'button': 'south'}]}}},
            *room(scene),
        ]
        call('command.execute', {'expected_revision': call('project.read')['revision'],
                                'description': 'Author Core Sample gameplay fixture', 'commands': commands})
        authored = call('project.read')
        call('history.undo')
        assert call('project.read')['project'] == document and not (inputs / 'gameplay.ts').exists()
        call('history.redo'); assert call('project.read')['project'] == authored['project']
        call('project.save')
        authored = call('project.read')
    finally:
        rpc.close()
    rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc'](binary, project, journal)
    try: assert call('project.read') == authored
    finally: rpc.close()
    author_hashes = {p.name: digest(p) for p in inputs.iterdir() if p.is_file()}
    results = {}
    for name in ('win', 'gamepad', 'jump', 'lose', 'repeat-win', 'skip-pickup', 'wall'):
        timeline = replay('win' if name == 'repeat-win' else name)
        path = out / f'{name}-input.json'; write(path, timeline)
        observe = out / f'{name}-observe.json'; write(observe, observations(name, timeline['ticks']))
        report = execute([binary, 'native-play', project, '--ticks', timeline['ticks'], '--input-replay', path, '--observe', observe])
        write(out / f'{name}.json', report)
        results[name] = report
    verify_reports(results)
    assert digest(binary) == binary_sha
    assert {p.name: digest(p) for p in inputs.iterdir() if p.is_file()} == author_hashes
    write(out / 'runs.json', {'binary_sha256': binary_sha, 'source_sha256': digest(ROOT / 'samples/core/gameplay.ts'),
                            'authoring_undo_redo_reopen': True, 'authoring_unchanged': True, 'reports': results})
    print(json.dumps({'passed': True, 'runs_completed': len(results), 'output': str(out)}))


if __name__ == '__main__':
    main()
