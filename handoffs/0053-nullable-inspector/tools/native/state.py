"""state.py <label> [dump.jsonl]: read-only accessibility snapshot of the owned 0053 window.

Adapted from 0052. Runs `winctl <pid> window`, `ax focuspath` and one `ax dump`, and prints one
JSON object: the window frame, the focus path, the Play transport (enabled state and help), the
header state words and paused tick, the status line, the Inspector's texts in reading order, and
two additions for this recheck:

  animation    every node of the Animation component, from its heading down to the next
               component heading, with its name AND its value (WebKit names a text field by its
               label, so the number sits in AXValue)
  unsupported  any node anywhere in the window whose text holds the Inspector's unsupported-type
               notice or a raw-value fallback

winctl redacts account names before anything reaches this script. Posts no input.
"""

import datetime
import json
import subprocess
import sys

SCRATCH = '/tmp/incant0053'
WINCTL = f'{SCRATCH}/winctl'
STATES = ('Playing', 'Paused', 'Preparing…', 'Stopping…', 'Play failed', 'Play unavailable', 'Live',
          'Suspended', 'Not attached', 'Waiting', 'Error', 'No engine')
# FieldView's Unsupported and Mismatch notice texts (editor/ui/src/components/inspector/FieldView.tsx);
# every Notice ends in "raw value:".
FALLBACKS = ('not supported', 'Unsupported field type', 'Cannot show as', 'raw value')
STATUS = ('Play stopped', 'Runtime changes', 'Paused at tick', 'could not start', 'stopped at tick')


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


def texts(node):
    return [node.get(k) for k in ('title', 'label', 'value', 'help') if node.get(k)]


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
    status = sorted({name(n) for n in dump if n['role'] == 'AXStaticText' and any(s in (name(n) or '') for s in STATUS)})
    # Header words only from the Viewport region's 36 px header line: the Animation component
    # has a field labelled "Playing" too.
    region = next((n for n in dump if n.get('sub') == 'AXLandmarkRegion' and name(n) == 'Viewport'), None)

    def in_header(n):
        return region is not None and n.get('x') is not None and region['x'] <= n['x'] < region['x'] + region['w'] \
            and region['y'] <= n['y'] < region['y'] + 36

    words = sorted({name(n) for n in dump if n['role'] == 'AXStaticText' and name(n) in STATES and in_header(n)})
    ticks = {'header': [name(n) for n in dump if n['role'] == 'AXStaticText' and (name(n) or '').startswith('tick ') and in_header(n)],
             'other': [name(n) for n in dump if n['role'] == 'AXStaticText' and (name(n) or '').startswith('tick ') and not in_header(n)]}
    # The Inspector is the right-hand column: everything whose left edge lies at or past the
    # Inspector region's left edge, in reading order.
    regions = [n for n in dump if (n.get('label') or n.get('title')) == 'Inspector' and n.get('w')]
    left = min((n['x'] for n in regions), default=None)
    inspector = []
    column = []
    if left is not None:
        column = [n for n in dump if n.get('x') is not None and n['x'] >= left - 0.5]
        for n in column:
            if n['role'] not in ('AXStaticText', 'AXTextField', 'AXButton', 'AXCheckBox', 'AXHeading', 'AXPopUpButton', 'AXDisclosureTriangle'):
                continue
            text = name(n)
            if text:
                inspector.append({'role': n['role'], 'text': text, 'box': box(n),
                                  **({'enabled': False} if n.get('enabled') is False else {})})
    # The Animation component: from its heading to the next component heading below it.
    headings = [n for n in column if n['role'] == 'AXHeading']
    top = next((n for n in headings if name(n) == 'Animation'), None)
    animation = []
    if top is not None:
        below = [n['y'] for n in headings if n['y'] > top['y'] + 1]
        bottom = min(below) if below else float('inf')
        for n in column:
            if top['y'] <= n['y'] < bottom and n['role'] in ('AXHeading', 'AXStaticText', 'AXTextField', 'AXCheckBox', 'AXButton'):
                animation.append({'role': n['role'], 'title': n.get('title'), 'label': n.get('label'), 'value': n.get('value'),
                                  'box': box(n), **({'focused': True} if n.get('focused') else {}),
                                  **({'enabled': False} if n.get('enabled') is False else {})})
    unsupported = [{'role': n['role'], 'text': ' | '.join(texts(n)), 'box': box(n)}
                   for n in dump if any(f in t for t in texts(n) for f in FALLBACKS)]
    scroll = [{'box': box(n)} for n in column if n['role'] == 'AXScrollArea']
    print(json.dumps({
        'label': label,
        'at': at,
        'window': window,
        'viewport_region': box(region) if region else None,
        # In the background the app may report no focused element at all ({"focused": null}).
        'focus': [{'role': n['role'], 'name': name(n), 'box': box(n)} for n in path[:4] if 'role' in n],
        'buttons': buttons,
        'words': words,
        'ticks': ticks,
        'status': status,
        'inspector_left': left,
        'inspector_regions': [box(n) for n in regions],
        'inspector_scroll': scroll,
        'animation': animation,
        'unsupported': unsupported,
        'inspector': inspector[:80],
        'nodes': len(dump),
    }, ensure_ascii=False))


if __name__ == '__main__':
    main()
