#!/usr/bin/env python3
"""Native-play scenarios for the Pearl Cove layout (used by scene.py --play).

Routes are axis-aligned segments of whole metres from the authored start, so
every turn happens at a cell centre with at least one metre of clearance from
each urchin's trigger radius. Runs carry a logical UI viewport because the scene
has an authored HUD. CPU native Play only; no GPU or device claim.
"""
import json
import math
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from art import cove  # noqa: E402

RATE = 60
STEP = 3.0 / RATE  # metres per tick at the authored speed
VECTORS = {'up': (0, 1), 'down': (0, -1), 'left': (-1, 0), 'right': (1, 0), 'stop': (0, 0)}
KEYS = {'wasd': ('KeyD', 'KeyA', 'KeyW', 'KeyS'),
        'arrows': ('ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown')}
A, B, C = cove.PEARLS
# The full collection route: west alcove, east alcove, south beach, then the pier.
WIN = [('up', 3), ('left', 3), ('up', 1), ('down', 1), ('right', 10), ('up', 1), ('right', 3),
       ('right', 1), ('down', 4), ('left', 7), ('down', 3)]
ROUTES = {
    'win': WIN,
    'urchin-loss': [('right', 7), ('down', 1)],
    'locked-boat': [('right', 4), ('down', 3)],
    'shore-and-cliff': [('down', 2), ('up', 7)],
}
SCENARIOS = [('keyboard-win', 'win', 'wasd'), ('arrows-win', 'win', 'arrows'),
             ('gamepad-win', 'win', 'stick'), ('dpad-win', 'win', 'dpad'),
             ('urchin-loss', 'urchin-loss', 'wasd'),
             ('locked-boat', 'locked-boat', 'wasd'), ('shore-and-cliff', 'shore-and-cliff', 'wasd')]
SETTLE = 40  # ticks after the last segment, so stopping and sensors resolve
UI_VIEWPORT = '640x360'  # the authored HUD needs a logical UI viewport


def segments(route):
    tick, out = 1, []
    for direction, metres in route:
        ticks = round(metres / STEP)
        out.append((tick, direction, ticks))
        tick += ticks
    return out, tick


def replay(route, device):
    parts, end = segments(route)
    frames = []
    for tick, direction, _ in parts + [(end, 'stop', 0)]:
        x, y = VECTORS[direction]
        connect = [{'type': 'gamepad_connected', 'id': 0}] if tick == 1 else []
        if device == 'stick':
            events = connect + [
                {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': x},
                {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': -y}]
        elif device == 'dpad':
            events = connect + [{'type': 'gamepad_button', 'id': 0, 'button': button, 'value': float(held)}
                                for button, held in (('dpad_right', x > 0), ('dpad_left', x < 0),
                                                     ('dpad_up', y > 0), ('dpad_down', y < 0))]
        else:
            right, left, up, down = KEYS[device]
            events = [{'type': 'key', 'code': code, 'down': held} for code, held in
                      ((right, x > 0), (left, x < 0), (up, y > 0), (down, y < 0))]
        frames.append({'tick': tick, 'events': events})
    return {'format': 'incant-input', 'version': 1, 'tick_rate': RATE, 'start_tick': 0,
            'ticks': end + SETTLE, 'frames': frames}


def observe(name, ticks):
    won = name.endswith('-win')
    gone = {'urchin-loss': [], 'locked-boat': [], 'shore-and-cliff': []}.get(name, cove.PEARLS)
    checks = [{'name': 'pickup-' + p, 'tick': ticks, 'expect': {
        'type': 'exists', 'entity': p, 'value': p not in gone}} for p in cove.PEARLS]
    if won or name == 'urchin-loss':
        checks.append({'name': 'stopped', 'tick': ticks, 'expect': {
            'type': 'vector', 'entity': cove.PLAYER, 'property': 'velocity',
            'value': [0, 0, 0], 'tolerance': 0}})
    return {'format': 'incant-native-observation', 'version': 1,
            'entities': [cove.PLAYER], 'checks': checks}


def verify(name, report):
    if not (report['passed'] and report['runtime'] == 'native' and report['observations']['passed']):
        raise RuntimeError(f'{name}: native run or observations failed')
    if report['gpu_submitted'] or report['logs']['dropped']:
        raise RuntimeError(f'{name}: unexpected GPU submission or dropped logs')
    messages = [entry['message'] for entry in report['logs']['entries']]
    picked = [m.split()[1] for m in messages if m.startswith('planar.pickup')]
    wins = sum(m.startswith('planar.won') for m in messages)
    losses = sum(m.startswith('planar.lost') for m in messages)
    x, y, z = report['observations']['entities'][cove.PLAYER]['translation']
    if abs(z) > 1e-9:
        raise RuntimeError(f'{name}: player left the XY plane')
    expected = {
        'win': (picked == [A, B, C] and wins == 1 and losses == 0
                and abs(x - 10.5) < 0.05 and 2.25 < y < 2.75),
        'urchin-loss': (not picked and losses == 1 and wins == 0
                        and abs(x - 13.5) < 0.05 and 4.1 < y < 4.25),
        'locked-boat': (not picked and wins == losses == 0
                        and abs(x - 10.5) < 0.05 and abs(y - 2.3) < 0.05),
        'shore-and-cliff': (not picked and wins == losses == 0
                            and abs(x - 6.5) < 0.05 and abs(y - 9.7) < 0.05),
    }[name if name in ('urchin-loss', 'locked-boat', 'shore-and-cliff') else 'win']
    if not expected:
        raise RuntimeError(f'{name}: unexpected outcome {picked} wins={wins} '
                           f'losses={losses} at {[x, y]}')
    return {'tick': report['tick'], 'pickups': len(picked), 'won': wins, 'lost': losses,
            'final_translation': [x, y]}


def run_all(out, runner, project):
    results, reports = {}, {}
    for name, route, device in SCENARIOS:
        timeline = replay(ROUTES[route], device)
        replay_path, plan_path = out / f'{name}-input.json', out / f'{name}-observe.json'
        replay_path.write_text(json.dumps(timeline, indent=2) + '\n')
        plan_path.write_text(json.dumps(observe(name, timeline['ticks']), indent=2) + '\n')
        report = runner.run('native-play', project, '--camera', cove.CAMERA,
                            '--ui-viewport', UI_VIEWPORT, '--ticks', timeline['ticks'],
                            '--input-replay', replay_path, '--observe', plan_path)
        (out / f'{name}.json').write_text(json.dumps(report, indent=2) + '\n')
        reports[name] = report
        results[name] = verify(name, report)
    for name in ('arrows-win', 'gamepad-win', 'dpad-win'):
        for key in ('logs', 'observations', 'pcm_sha256', 'entity_count'):
            if reports[name][key] != reports['keyboard-win'][key]:
                raise RuntimeError(f'{name} differs from keyboard-win in {key}')
    route = sum(m for _, m in WIN)
    results['route_metres'] = route
    results['route_seconds_at_speed'] = math.ceil(route / 3.0)
    return results
