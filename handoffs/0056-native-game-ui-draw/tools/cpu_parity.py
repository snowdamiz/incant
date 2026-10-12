#!/usr/bin/env python3
"""Replay the GPU review's desktop sequence through the ordinary CPU package player.

Usage: python3 -I -B cpu_parity.py PACKAGE_OUT REVIEW_DIR OUTPUT_DIR

Writes an incant-input v2 recording of the same physical keys and UI receipts
(pointer positions taken from the review's own record), runs the packaged
`--package data` CPU route with `--ui-viewport 960x540` under an empty
environment, and compares the final accepted UI owner (focus, values, glyph
count, UI atlas SHA-256) with the GPU review's composed game.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

NAME, ARABIC, CJK = '0000000000000000000000002A', '0000000000000000000000002C', '0000000000000000000000002E'


def key(code, down):
    return {'type': 'key', 'code': code, 'down': down}


def main():
    package, review, output = (Path(a).resolve() for a in sys.argv[1:4])
    output.mkdir(parents=True, exist_ok=False)
    data = package / 'player' / 'data'
    player = package / 'player' / 'incant_game_player'
    record = json.loads((package / 'package.json').read_text(encoding='utf-8'))
    actual = hashlib.sha256(player.read_bytes()).hexdigest()
    assert actual == record['player_sha256'], 'player changed after packaging'
    case = next(c for c in json.loads((review / 'review.json').read_text(encoding='utf-8'))['cases']
                if c['case'] == 'desktop-1x')
    rate = json.loads((data / 'empty-input.json').read_text(encoding='utf-8'))['tick_rate']
    press, release = case['pointers']
    select = [key('ShiftLeft', True)]
    for _ in range(len('Lovelace')):
        select += [key('ArrowLeft', True), key('ArrowLeft', False)]
    select.append(key('ShiftLeft', False))
    frames = [
        {'tick': 2, 'events': [key('Tab', True)]},
        {'tick': 3, 'events': [key('Tab', False)]},
        {'tick': press['tick'], 'events': [],
         'ui': [{'type': 'pointer', 'position': press['position'], 'pressed': True}]},
        {'tick': release['tick'], 'events': [],
         'ui': [{'type': 'pointer', 'position': release['position'], 'pressed': False}]},
        {'tick': 6, 'events': [], 'ui': [{'type': 'focus', 'entity_id': NAME}]},
        {'tick': 7, 'events': select},
        {'tick': 8, 'events': [], 'ui': [{'type': 'commit', 'entity_id': NAME, 'text': 'Byron'}]},
        {'tick': 9, 'events': [], 'ui': [{'type': 'focus', 'entity_id': ARABIC}]},
        {'tick': 10, 'events': [], 'ui': [{'type': 'focus', 'entity_id': CJK}]},
        {'tick': 11, 'events': [], 'ui': [{'type': 'preedit', 'entity_id': CJK, 'text': '語', 'cursor': [3, 3]}]},
        {'tick': 12, 'events': [], 'ui': [{'type': 'commit', 'entity_id': CJK, 'text': '語'}]},
        {'tick': 13, 'events': [key('Enter', True)]},
        {'tick': 14, 'events': [key('Enter', False)]},
    ]
    assert (press['tick'], release['tick']) == (4, 5), case['pointers']
    recording = {'format': 'incant-input', 'version': 2, 'tick_rate': rate, 'start_tick': 0,
                 'ticks': case['final']['tick'], 'frames': frames}
    (output / 'recording.json').write_text(json.dumps(recording, indent=2, ensure_ascii=False) + '\n',
                                          encoding='utf-8')
    home = output / 'absent-home'
    result = subprocess.run([str(player), '--package', str(data), '--ticks', str(case['final']['tick']),
                             '--output', str(output / 'run.pcm'), '--ui-viewport', '960x540',
                             '--input-recording', str(output / 'recording.json')],
                            cwd=player.parent, env={'PATH': str(player.parent), 'HOME': str(home),
                                                    'TMPDIR': str(output)},
                            capture_output=True, text=True, encoding='utf-8', timeout=120)
    assert result.returncode == 0, result.stderr[-4000:]
    report = json.loads(result.stdout)
    (output / 'cpu-report.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n',
                                            encoding='utf-8')
    cpu = report['render_input']['ui']
    gpu = case['final']
    comparison = {
        'player_sha256': actual, 'ticks': gpu['tick'], 'cpu_tick': report['render_input']['tick'],
        'focus': [cpu['focus'], gpu['focus']], 'values': [cpu['values'], gpu['values']],
        'glyphs': [cpu['glyphs'], gpu['glyphs']], 'atlas_sha256': [cpu['atlas_sha256'], gpu['atlas_sha256']],
        'gpu_submitted': report['render_input']['gpu_submitted'],
    }
    comparison['equal'] = all(a == b for a, b in (comparison[k] for k in ('focus', 'values', 'glyphs', 'atlas_sha256'))) \
        and comparison['cpu_tick'] == gpu['tick']
    (output / 'comparison.json').write_text(json.dumps(comparison, indent=2, ensure_ascii=False) + '\n',
                                            encoding='utf-8')
    print(json.dumps(comparison, ensure_ascii=False))
    assert comparison['equal'], 'CPU route and GPU review disagree'


if __name__ == '__main__':
    main()
