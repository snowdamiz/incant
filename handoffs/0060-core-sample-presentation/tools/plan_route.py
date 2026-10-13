#!/usr/bin/env python3
"""plan_route.py BUILD PLAYER_DIR OUT: plan a winning stick route on the real simulation.

Goals are the remaining samples (nearest first), then the lift; each goal is reached
through hand-placed corridor waypoints. A hit costs 150 points but is allowed while two
or more health remain; a candidate that would lose is never taken.

Receding horizon: every STEP ticks, try a few stick headings around the direction of
the next coarse waypoint, simulate the whole run so far plus each candidate for a
look-ahead window on the packaged player (deterministic), and keep the candidate
that avoids damage, keeps clear of chasers and makes progress. The result is an
ordinary input recording; samples/core/playtest.py replays it as its win route.
Development search aid only.
"""
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'samples/core'))
from art import level  # noqa: E402

STEP, LOOK, MAX_TICKS = 24, 48, 3500
OFFSETS = [0, 30, -30, 60, -60, 95, -95, 140, -140, 180]


def run(player_dir, out, frames, ticks, receipt, sample_every=12):
    data = player_dir / 'data'
    manifest = json.loads((data / 'package.json').read_text())
    record = {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0, 'ticks': ticks,
              'frames': frames}
    actors = [receipt['entities']['player'], *receipt['enemies']]
    first = max(1, ticks - STEP - LOOK)
    checks = []
    for t in range(first, ticks + 1, sample_every):
        for a in actors:
            checks.append({'name': f'{a}-{t}', 'tick': t, 'expect': {'type': 'vector', 'entity': a,
                           'property': 'translation', 'value': [0, 0, 0], 'tolerance': 1000}})
    (out / 'r.json').write_text(json.dumps(record))
    (out / 'o.json').write_text(json.dumps({'format': 'incant-native-observation', 'version': 1,
                                            'entities': actors, 'checks': checks[:250]}))
    pcm = out / 'r.pcm'
    pcm.unlink(missing_ok=True)
    result = subprocess.run([str(player_dir / 'incant_game_player'), str(data / 'world.scene'), str(data / 'audio'),
                             str(data / 'clips.bin'), str(out / 'r.json'), str(data / 'input-actions.json'),
                             str(ticks), '16000', str(pcm), '--cpu-render-input', str(data / 'models'),
                             str(data / 'textures'), manifest['camera'], '--font-cache', str(data / 'fonts'),
                             '--ui-viewport', '640x480', '--observe', str(out / 'o.json')],
                            cwd=player_dir, text=True, capture_output=True, timeout=600,
                            env={'PATH': str(player_dir), 'HOME': str(out / 'absent'), 'TMPDIR': str(out)})
    if result.returncode:
        raise RuntimeError(result.stderr[-800:])
    report = json.loads(result.stdout)
    track = {}
    for check in report['observations']['checks']:
        entity, tick = check['name'].rsplit('-', 1)
        track.setdefault(int(tick), {})[entity] = check['actual']
    logs = [(e['tick'], e['message']) for e in report['logs']['entries']]
    return report, track, logs


# Corridor waypoints into each goal (approach side first), level coordinates (x, z).
GOALS = {
    'S1': [(-6.2, 5.2)],
    'S2': [(1.8, -8.4), (3.3, -10.3)],
    'S3': [(7.6, 3.4), (7.6, -2.8), (8.6, -9.4)],
    'S4': [(-9.1, -6.6), (-9.1, -13.2), (-8.0, -16.8)],
    'LIFT': [(-1.3, -6.2), (-1.3, -13.0), (0.5, -15.0), (0.5, -17.4)],
}
UPPER = {'S3', 'S4'}


def upper(pos):
    return pos[1] > 1.4


def main():
    build, player_dir, out = (Path(a).absolute() for a in sys.argv[1:4])
    out.mkdir(parents=True)
    receipt = json.loads((build / 'receipt.json').read_text())
    player, enemies = receipt['entities']['player'], receipt['enemies']
    frames = [{'tick': 1, 'events': [{'type': 'gamepad_connected', 'id': 0}]}]
    tick, history, health = 2, [], 3
    pos = level.PLAYER_START
    remaining = ['S1', 'S2', 'S3', 'S4']
    goal, route, stuck = None, [], 0
    while tick < MAX_TICKS:
        if not route:
            if not remaining:
                goal = 'LIFT'
            else:
                goal = min(remaining, key=lambda g: math.hypot(GOALS[g][-1][0] - pos[0], GOALS[g][-1][1] - pos[2]))
            if goal == 'LIFT' and remaining:
                break
            # Start from the corridor point nearest to the current position on the same level.
            points = GOALS[goal]
            first = min(range(len(points)), key=lambda i: math.hypot(points[i][0] - pos[0], points[i][1] - pos[2]))
            route = list(points[first:]) if (goal in UPPER) == upper(pos) or goal == 'LIFT' else list(points)
            stuck = 0
        target = route[0]
        base = math.atan2(target[1] - pos[2], target[0] - pos[0])
        best = None
        for offset in OFFSETS:
            a = base + math.radians(offset)
            sx, sz = round(math.cos(a), 5), round(math.sin(a), 5)
            trial = frames + [{'tick': tick, 'events': [
                {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': sx},
                {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': sz}]}]
            report, track, logs = run(player_dir, out, trial, tick + STEP + LOOK, receipt)
            hits = [t for t, m in logs if m.startswith('core.damage') and t >= tick]
            lost = any(m.startswith('core.lost') for _, m in logs)
            won = any(m.startswith('core.won') for _, m in logs)
            clearance = min(math.hypot(s_[player][0] - s_[e][0], s_[player][2] - s_[e][2])
                            for t, s_ in track.items() if t >= tick for e in enemies)
            at = track[max(t for t in track if t <= tick + STEP)][player]
            dist = math.hypot(target[0] - at[0], target[1] - at[2])
            progress = math.hypot(target[0] - pos[0], target[1] - pos[2]) - dist
            moved = math.hypot(at[0] - pos[0], at[2] - pos[2])
            score = progress * 5 + min(clearance, 3.0) * 1.5 + moved - 150 * len(hits) - \
                (10000 if lost or len(hits) >= health else 0) + (5000 if won else 0)
            if best is None or score > best[0]:
                best = (score, sx, sz, at, hits, clearance, won, logs)
            if offset == 0 and not hits and clearance > 2.4:
                break
        score, sx, sz, at, hits, clearance, won, logs = best
        frames.append({'tick': tick, 'events': [
            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': sx},
            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': sz}]})
        collected = {m.split()[1] for t, m in logs if m.startswith('core.pickup') and t < tick + STEP}
        health = 3 - len([t for t, m in logs if m.startswith('core.damage') and t < tick + STEP])
        history.append({'tick': tick, 'goal': goal, 'target': target, 'stick': [sx, sz],
                        'clearance': round(clearance, 2), 'hits': hits, 'health': health,
                        'at': [round(v, 2) for v in at]})
        print(json.dumps(history[-1]), flush=True)
        tick += STEP
        stuck = stuck + 1 if math.hypot(at[0] - pos[0], at[2] - pos[2]) < 0.5 else 0
        pos = at
        ids_by_goal = {'S%d' % (i + 1): receipt['samples'][i] for i in range(4)}
        remaining = [g for g in remaining if ids_by_goal[g] not in collected]
        if goal != 'LIFT' and goal not in remaining:
            route = []
        while route and math.hypot(route[0][0] - pos[0], route[0][1] - pos[2]) < 0.7:
            route.pop(0)
            if not route and goal != 'LIFT':
                route = []
        if stuck >= 4 and goal != 'LIFT' and len(remaining) > 1:
            remaining = remaining[1:] + remaining[:1]
            route = []
        if won:
            break
    frames.append({'tick': tick, 'events': [{'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': 0},
                                            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': 0}]})
    total = tick + 90
    report, track, logs = run(player_dir, out, frames, total, receipt)
    result = {'ticks': total, 'frames': frames, 'logs': logs, 'history': history}
    (out / 'plan.json').write_text(json.dumps(result, indent=1))
    print(json.dumps({'ticks': total, 'won': [t for t, m in logs if m.startswith('core.won')],
                      'pickups': [m for _, m in logs if m.startswith('core.pickup')],
                      'damage': [t for t, m in logs if m.startswith('core.damage')]}))


if __name__ == '__main__':
    main()
