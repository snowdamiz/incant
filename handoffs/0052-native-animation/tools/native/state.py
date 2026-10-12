"""state.py <label> [dump.jsonl]: read-only accessibility snapshot of the owned 0052 window.

Adapted from 0050. Runs `winctl <pid> window`, `ax focuspath` and one `ax dump`, and prints one
JSON object: the window frame, the focus path, the Play transport (enabled state and help), the
header state words and paused tick, the scene/camera picker and notice, the game view's bounds,
and the Inspector's texts (its header, Runtime block and component rows) in reading order.
winctl redacts account names before anything reaches this script. Posts no input.
"""

import datetime
import json
import subprocess
import sys

SCRATCH = '/tmp/incant0052'
WINCTL = f'{SCRATCH}/winctl'
STATES = ('Playing', 'Paused', 'Preparing…', 'Stopping…', 'Play failed', 'Play unavailable', 'Live',
          'Suspended', 'Not attached', 'Waiting', 'Error', 'No engine')


def run(*args):
    out = subprocess.run([WINCTL, *args], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        raise SystemExit(f'winctl {" ".join(args)} failed: {out.stderr.strip()}')
    return out.stdout


def lines(*args):
    return [json.loads(line) for line in run(*args).splitlines() if line.strip()]


def name(node):
    return node.get('label') or node.get('title') or node.get('value')


def box(node):
    return [node.get(k) for k in ('x', 'y', 'w', 'h')]


def main():
    label = sys.argv[1]
    pid = open(f'{SCRATCH}/main.pid').read().strip()
    at = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='milliseconds')
    window = json.loads(run(pid, 'window'))
    path = lines(pid, 'ax', 'focuspath')
    dump = lines(pid, 'ax', 'dump')
    if len(sys.argv) > 2:
        with open(sys.argv[2], 'w', encoding='utf-8') as handle:
            for node in dump:
                handle.write(json.dumps(node, ensure_ascii=False) + '\n')
    buttons = {}
    for node in dump:
        if node['role'] == 'AXButton' and name(node) in ('Play', 'Pause', 'Resume', 'Step', 'Stop', 'Dismiss'):
            buttons.setdefault(name(node), []).append({'box': box(node), 'enabled': node.get('enabled', True), 'help': node.get('help')})
    picker = [{'name': name(n), 'box': box(n)} for n in dump
              if n['role'] in ('AXButton', 'AXPopUpButton') and (name(n) or '').startswith('Play scene and camera')]
    notice = [name(n) for n in dump if 'could not start' in (name(n) or '') or 'stopped at tick' in (name(n) or '')]
    # Header words only from the Viewport region's 36 px header line: the Animation component
    # has a field labelled "Playing" too.
    region = next((n for n in dump if n.get('sub') == 'AXLandmarkRegion' and name(n) == 'Viewport'), None)

    def in_header(n):
        return region is not None and n.get('x') is not None and region['x'] <= n['x'] < region['x'] + region['w'] \
            and region['y'] <= n['y'] < region['y'] + 36

    words = sorted({name(n) for n in dump if n['role'] == 'AXStaticText' and name(n) in STATES and in_header(n)})
    ticks = {'header': [name(n) for n in dump if n['role'] == 'AXStaticText' and (name(n) or '').startswith('tick ') and in_header(n)],
             'other': [name(n) for n in dump if n['role'] == 'AXStaticText' and (name(n) or '').startswith('tick ') and not in_header(n)]}
    game = [{'role': n['role'], 'sub': n.get('sub'), 'box': box(n), 'focused': n.get('focused', False)}
            for n in dump if (n.get('label') or n.get('title')) == 'Game view']
    # The Inspector is the right-hand column: everything whose left edge lies at or past the
    # Inspector region's left edge, in reading order.
    regions = [n for n in dump if (n.get('label') or n.get('title')) == 'Inspector' and n.get('w')]
    left = min((n['x'] for n in regions), default=None)
    inspector = []
    if left is not None:
        for n in dump:
            if n.get('x') is None or n['x'] < left - 0.5 or n['role'] not in (
                    'AXStaticText', 'AXTextField', 'AXButton', 'AXCheckBox', 'AXHeading', 'AXPopUpButton', 'AXDisclosureTriangle'):
                continue
            text = name(n)
            if text:
                inspector.append({'role': n['role'], 'text': text, 'box': box(n),
                                  **({'enabled': False} if n.get('enabled') is False else {})})
    print(json.dumps({
        'label': label,
        'at': at,
        'window': window,
        'viewport_region': box(region) if region else None,
        # In the background the app may report no focused element at all ({"focused": null}).
        'focus': [{'role': n['role'], 'name': name(n), 'box': box(n)} for n in path[:4] if 'role' in n],
        'buttons': buttons,
        'picker': picker,
        'notice': notice,
        'words': words,
        'ticks': ticks,
        'game_view': game,
        'inspector_left': left,
        'inspector': inspector[:80],
        'nodes': len(dump),
    }, ensure_ascii=False))


if __name__ == '__main__':
    main()
