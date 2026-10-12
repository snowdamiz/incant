#!/usr/bin/env python3
"""Play the built Core Sample level headlessly on the ordinary native player.

Usage: python3 -B samples/core/playtest.py BUILD OUTPUT --player PLAYER_DIR

BUILD is a scene.py output (project/ and receipt.json); PLAYER_DIR holds the
exported `incant_game_player` and its cooked `data/` for that same project (built
by the export route; the package receipt pins the project digest). Each scenario
runs the player's CPU route (cooked bytecode, no source parsing, no GPU) with an
input recording and a 640x480 UI viewport, because `incant native-play` refuses
scenes with screen-space UI. The explicit argument form of `--package` is used with
a 16 kHz PCM sink, since the packaged form's 48 kHz sink caps a run at 1250 ticks. With the unmodified controller it checks:
  win    routes/win-input.json, a stick recording planned on this level's real simulation
         (handoff 0060 tool plan_route.py): all samples, the lift, at least one health left
  lose   no input: chasers path over the navmesh (both ramps) and win by contact
  locked straight to the lift without samples: the exit stays locked
Each run also requires every chaser to have planned a navmesh route. Authoring is
hashed before and after to prove Play changes nothing. Not a device, performance
or shipping proof; the controller's own seven fixtures live in verify.py.
"""
import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from art import level  # noqa: E402

SPEED = 3.4
RATE = 60
LOCKED_ROUTE = [(-0.5, 10.0), (-4.0, 2.0), (-8.0, -6.0), (-9.2, -7.2), (-9.2, -13.2), (0.5, -14.5), (0.5, -17.4)]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def stick_replay(route, extra_ticks=60, total=None):
    """Hold the left stick toward each waypoint for its straight-line travel time."""
    frames, tick = [{'tick': 1, 'events': [{'type': 'gamepad_connected', 'id': 0}]}], 2
    for (ax, az), (bx, bz) in zip(route, route[1:]):
        dx, dz = bx - ax, bz - az
        dist = math.hypot(dx, dz)
        frames.append({'tick': tick, 'events': [
            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': round(dx / dist, 6)},
            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': round(dz / dist, 6)}]})
        tick += max(1, math.ceil(dist / SPEED * RATE))
    frames.append({'tick': tick, 'events': [{'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': 0},
                                            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': 0}]})
    ticks = total or tick + extra_ticks
    return {'format': 'incant-input', 'version': 1, 'tick_rate': RATE, 'start_tick': 0, 'ticks': ticks,
            'frames': frames}


def observe(receipt, ticks):
    actors = [receipt['entities']['player'], *receipt['enemies']]
    return {'format': 'incant-native-observation', 'version': 1, 'entities': actors, 'checks': [
        {'name': 'runs-to-end', 'tick': ticks, 'expect': {'type': 'exists', 'entity': actors[0], 'value': True}}]}


def summarize(report, receipt):
    messages = [(e['tick'], e['message']) for e in report['logs']['entries']]
    pick = [m for _, m in messages if m.startswith('core.pickup')]
    damage = [(t, m) for t, m in messages if m.startswith('core.damage')]
    routes = [m for _, m in messages if m.startswith('core.route')]
    won = [t for t, m in messages if m.startswith('core.won')]
    lost = [t for t, m in messages if m.startswith('core.lost')]
    final = report['observations']['entities']
    player = final[receipt['entities']['player']]['translation']
    return {'pickups': len(pick), 'pickup_order': [m.split()[1] for m in pick], 'damage': damage,
            'routes': routes, 'won_tick': won[0] if won else None, 'lost_tick': lost[0] if lost else None,
            'player_final': [round(v, 3) for v in player],
            'enemies_final': {e: [round(v, 3) for v in final[e]['translation']] for e in receipt['enemies']}}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('build', type=Path, help='scene.py OUTPUT directory (project/ and receipt.json)')
    parser.add_argument('output', type=Path)
    parser.add_argument('--player', type=Path, required=True, help='directory with incant_game_player and data/')
    args = parser.parse_args()
    build = args.build.resolve(strict=True)
    out = args.output.absolute().resolve()
    out.mkdir(parents=True, exist_ok=False)
    player_dir = args.player.resolve(strict=True)
    binary = player_dir / 'incant_game_player'
    package = json.loads((player_dir.parent / 'package.json').read_text(encoding='utf-8'))
    if package['project_sha256'] != digest(build / 'project/core.json'):
        raise RuntimeError('player package was cooked from a different project')
    receipt = json.loads((build / 'receipt.json').read_text(encoding='utf-8'))
    project = build / 'project/core.json'
    authored = {p.name: digest(p) for p in project.parent.iterdir() if p.is_file()}
    win = json.loads((HERE / 'routes/win-input.json').read_text(encoding='utf-8'))
    scenarios = {'win': win, 'lose': stick_replay([(0, 0), (0, 0.001)], total=1500),
                 'locked': stick_replay(LOCKED_ROUTE, 120)}
    results = {}
    for name, replay in scenarios.items():
        (out / f'{name}-input.json').write_text(json.dumps(replay, indent=1) + '\n', encoding='utf-8')
        (out / f'{name}-observe.json').write_text(json.dumps(observe(receipt, replay['ticks'])) + '\n',
                                                  encoding='utf-8')
        # The explicit form of `--package data`: same cooked files, but a 16 kHz PCM
        # sink so a full route fits the player's one-million-frame output bound.
        data = player_dir / 'data'
        manifest = json.loads((data / 'package.json').read_text(encoding='utf-8'))
        result = subprocess.run([str(binary), str(data / 'world.scene'), str(data / 'audio'), str(data / 'clips.bin'),
                                 str(out / f'{name}-input.json'), str(data / 'input-actions.json'),
                                 str(replay['ticks']), '16000', str(out / f'{name}.pcm'),
                                 '--cpu-render-input', str(data / 'models'), str(data / 'textures'),
                                 manifest['camera'], '--font-cache', str(data / 'fonts'),
                                 '--ui-viewport', '640x480', '--observe', str(out / f'{name}-observe.json')],
                                cwd=player_dir, text=True, capture_output=True, timeout=600,
                                env={'PATH': str(player_dir), 'HOME': str(out / 'absent-home'), 'TMPDIR': str(out)})
        if result.returncode:
            raise RuntimeError(f'{name}: {result.stderr[-4000:]}')
        report = json.loads(result.stdout)
        (out / f'{name}.json').write_text(json.dumps(report, indent=1) + '\n', encoding='utf-8')
        results[name] = summarize(report, receipt)
        results[name]['ticks'] = replay['ticks']
    win, lose, locked = results['win'], results['lose'], results['locked']
    checks = {
        'win_collects_all': win['pickups'] == len(level.SAMPLES),
        'win_reaches_lift': win['won_tick'] is not None and win['lost_tick'] is None,
        'lose_by_contact': lose['lost_tick'] is not None and len(lose['damage']) == 3,
        'locked_exit_refuses': locked['won_tick'] is None and locked['pickups'] == 0,
        'every_chaser_routes': all(len(r['routes']) == len(level.ENEMIES) for r in results.values()),
        'authoring_unchanged': {p.name: digest(p) for p in project.parent.iterdir() if p.is_file()} == authored,
    }
    summary = {'passed': all(checks.values()), 'checks': checks, 'results': results,
               'player_sha256': digest(binary), 'project_sha256': digest(project),
               'route': 'ordinary packaged player, CPU render-input with 640x480 UI, cooked bytecode'}
    (out / 'playtest.json').write_text(json.dumps(summary, indent=1) + '\n', encoding='utf-8')
    print(json.dumps(summary, indent=1))
    if not summary['passed']:
        sys.exit(1)


if __name__ == '__main__':
    main()
