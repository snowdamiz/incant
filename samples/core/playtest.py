#!/usr/bin/env python3
"""Play the built Core Sample level headlessly on the ordinary native player.

Usage: python3 -B samples/core/playtest.py BUILD OUTPUT --player PLAYER_DIR

BUILD is a scene.py output (project/ and receipt.json); PLAYER_DIR holds the
exported `incant_game_player` and its cooked `data/` for that same project (built
by the export route; the package receipt pins the project digest). Each scenario
runs the player's CPU route (cooked bytecode, no source parsing, no GPU) with an
input recording and a 640x480 UI viewport, using the existing explicit-file
player route. The 16 kHz PCM sink also supports
the retained pre-streaming player used by handoff 0060; newer streaming players
can run the complete route at 48 kHz. With the unmodified controller it checks:
  win    routes/win-input.json, a stick recording planned on this level's real simulation
         (handoff 0060 tool plan_route.py): all samples, the lift, at least one health left
  lose   no input: chasers path over the navmesh (both ramps) and win by contact
  locked routes/locked-input.json: alive inside the lift with 3/4 samples,
         then 30 stopped ticks inside the trigger without winning
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
LOCKED_ARRIVAL = 1559
EXIT_RADIUS = 1.0  # The unchanged gameplay.ts victory distance, in model/world metres.


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


def observe(receipt, ticks, locked=False):
    actors = [receipt['entities']['player'], *receipt['enemies']]
    checks = [{'name': 'runs-to-end', 'tick': ticks,
               'expect': {'type': 'exists', 'entity': actors[0], 'value': True}}]
    if locked:
        actors += [receipt['entities']['exit'], *receipt['samples']]
        for tick in range(LOCKED_ARRIVAL, ticks + 1):
            for role in ('player', 'exit'):
                checks.append({'name': f'locked-{role}-{tick}', 'tick': tick, 'expect': {
                    'type': 'vector', 'entity': receipt['entities'][role], 'property': 'translation',
                    'value': list(level.EXIT), 'tolerance': .1 if role == 'player' else 0}})
            for i, entity in enumerate(receipt['samples']):
                checks.append({'name': f'locked-inventory-{i}-{tick}', 'tick': tick, 'expect': {
                    'type': 'exists', 'entity': entity, 'value': i == 3}})
    return {'format': 'incant-native-observation', 'version': 1, 'entities': actors, 'checks': checks}


def locked_exit_checks(report, receipt, ticks):
    """A terminal/dead actor near the lift is not evidence of its inventory gate.

    Observe both actual positions on each of 30 stopped ticks from arrival. The
    native checks also require precisely the uncollected fourth pickup to remain.
    Logs must be complete so absence of a terminal event is meaningful.
    """
    logs = report['logs']
    messages = [e['message'] for e in logs['entries']]
    collected = [m.split()[1] for m in messages if m.startswith('core.pickup ')]
    damage = [m for m in messages if m.startswith('core.damage ')]
    health = int(damage[-1].split('health=')[1]) if damage else 3
    records = report['observations']['checks']
    expected = observe(receipt, ticks, locked=True)['checks']
    # Native observations are canonicalized by tick/name, not request order.
    records = sorted(records, key=lambda row: (row['tick'], row['name']))
    expected = sorted(expected, key=lambda row: (row['tick'], row['name']))
    exact = len(records) == len(expected) and all(
        r['name'] == e['name'] and r['tick'] == e['tick'] and r['expect'] == e['expect'] and r['passed']
        and (e['expect']['type'] != 'exists' or r['actual'] is e['expect']['value'])
        for r, e in zip(records, expected))
    distances = {}
    if exact:
        by_name = {r['name']: r['actual'] for r in records}
        for tick in range(LOCKED_ARRIVAL, ticks + 1):
            a, b = by_name[f'locked-player-{tick}'], by_name[f'locked-exit-{tick}']
            distances[str(tick)] = math.dist(a, b)
    checks = {
        'complete_observations': exact and report['observations']['passed'],
        'complete_gameplay_logs': logs['dropped'] == 0,
        'incomplete_inventory': collected == receipt['samples'][:3],
        'live_player': 0 < health <= 3 and not any(m.startswith('core.lost ') for m in messages),
        'no_victory': not any(m.startswith('core.won ') for m in messages),
        'inside_exit_for_entire_dwell': len(distances) == 30 and all(
            math.isfinite(d) and d <= EXIT_RADIUS for d in distances.values()),
        'thirty_stopped_ticks': ticks == LOCKED_ARRIVAL + 29,
    }
    return {'passed': all(checks.values()), 'checks': checks, 'health': health,
            'exit_distances': distances, 'pickup_ids': collected}


def authored_files(project):
    return {str(p.relative_to(project.parent)): digest(p)
            for p in sorted(project.parent.rglob('*')) if p.is_file()}


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
    if (digest(binary) != package['player_sha256'] or
            digest(HERE / 'gameplay.ts') != receipt['gameplay_sha256'] or
            package['producer_receipt']['compiler']['modules'] != [
                {'bytes': (HERE / 'gameplay.ts').stat().st_size, 'path': 'gameplay.ts',
                 'sha256': receipt['gameplay_sha256']}]):
        raise RuntimeError('player/controller identity does not match the authoring receipt')
    for path, expected in package['data']['files'].items():
        if digest(player_dir / 'data' / path) != expected:
            raise RuntimeError(f'packaged data identity mismatch: {path}')
    project = build / 'project/core.json'
    authored = authored_files(project)
    win = json.loads((HERE / 'routes/win-input.json').read_text(encoding='utf-8'))
    scenarios = {'win': win, 'lose': stick_replay([(0, 0), (0, 0.001)], total=1500),
                 'locked': json.loads((HERE / 'routes/locked-input.json').read_text(encoding='utf-8'))}
    results = {}
    for name, replay in scenarios.items():
        (out / f'{name}-input.json').write_text(json.dumps(replay, indent=1) + '\n', encoding='utf-8')
        (out / f'{name}-observe.json').write_text(json.dumps(observe(receipt, replay['ticks'], locked=name == 'locked')) + '\n',
                                                  encoding='utf-8')
        # The explicit form of `--package data`: same cooked files, but a 16 kHz PCM
        # sink so a full route fits the player's one-million-frame output bound.
        data = player_dir / 'data'
        manifest = json.loads((data / 'package.json').read_text(encoding='utf-8'))
        command = [str(binary), str(data / 'world.scene'), str(data / 'audio'), str(data / 'clips.bin'),
                                 str(out / f'{name}-input.json'), str(data / 'input-actions.json'),
                                 str(replay['ticks']), '16000', str(out / f'{name}.pcm'),
                                 '--cpu-render-input', str(data / 'models'), str(data / 'textures'),
                                 manifest['camera'], '--font-cache', str(data / 'fonts'),
                                 '--ui-viewport', '640x480', '--observe', str(out / f'{name}-observe.json')]
        result = subprocess.run(command, cwd=player_dir, text=True, capture_output=True, timeout=600,
                                env={'PATH': str(player_dir), 'TMPDIR': str(out)})
        (out / f'{name}.stdout').write_text(result.stdout, encoding='utf-8')
        (out / f'{name}.stderr').write_text(result.stderr, encoding='utf-8')
        (out / f'{name}-command.json').write_text(json.dumps({
            'argv': command, 'cwd': str(player_dir), 'exit_code': result.returncode,
            'stdout_sha256': digest(out / f'{name}.stdout'),
            'stderr_sha256': digest(out / f'{name}.stderr')}, indent=1) + '\n', encoding='utf-8')
        if result.returncode:
            raise RuntimeError(f'{name}: {result.stderr[-4000:]}')
        report = json.loads(result.stdout)
        (out / f'{name}.json').write_text(json.dumps(report, indent=1) + '\n', encoding='utf-8')
        results[name] = summarize(report, receipt)
        results[name]['ticks'] = replay['ticks']
        results[name]['observations_passed'] = report['observations']['passed']
        results[name]['pcm_sha256'] = digest(out / f'{name}.pcm')
        if name == 'locked':
            results[name]['exit_proof'] = locked_exit_checks(report, receipt, replay['ticks'])
    win, lose, locked = results['win'], results['lose'], results['locked']
    checks = {
        'win_collects_all': win['pickups'] == len(level.SAMPLES),
        'win_reaches_lift': win['won_tick'] is not None and win['lost_tick'] is None,
        'lose_by_contact': lose['lost_tick'] is not None and len(lose['damage']) == 3,
        'locked_exit_refuses': locked['exit_proof']['passed'],
        'observations_passed': all(r['observations_passed'] for r in results.values()),
        'every_chaser_routes': all(len(r['routes']) == len(level.ENEMIES) for r in results.values()),
        'authoring_unchanged': authored_files(project) == authored,
    }
    summary = {'passed': all(checks.values()), 'checks': checks, 'results': results,
               'player_sha256': digest(binary), 'project_sha256': digest(project),
               'package_sha256': digest(player_dir.parent / 'package.json'),
               'gameplay_sha256': digest(HERE / 'gameplay.ts'),
               'probe_sha256': digest(Path(__file__)),
               'recordings': {name: digest(out / f'{name}-input.json') for name in scenarios},
               'route': 'ordinary packaged player, CPU render-input with 640x480 UI, cooked bytecode'}
    (out / 'playtest.json').write_text(json.dumps(summary, indent=1) + '\n', encoding='utf-8')
    print(json.dumps(summary, indent=1))
    if not summary['passed']:
        sys.exit(1)


if __name__ == '__main__':
    main()
