"""state.py <label> [dump.jsonl]: read-only accessibility snapshot of the owned 0050 window.

Runs `winctl <pid> ax focuspath` and one `ax dump`, and prints one JSON object: the focus path,
the Play transport, picker and notice, the header words, the composer bar and the separators.
winctl redacts account names before anything reaches this script. Posts no input.
"""

import datetime
import json
import subprocess
import sys

SCRATCH = '/tmp/incant0050'
WINCTL = f'{SCRATCH}/winctl'


def lines(*args):
    out = subprocess.run([WINCTL, *args], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        raise SystemExit(f'winctl {" ".join(args)} failed: {out.stderr.strip()}')
    return [json.loads(line) for line in out.stdout.splitlines() if line.strip()]


def name(node):
    return node.get('label') or node.get('title') or node.get('value')


def box(node):
    return [node.get(k) for k in ('x', 'y', 'w', 'h')]


def main():
    label = sys.argv[1]
    pid = open(f'{SCRATCH}/main.pid').read().strip()
    at = datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='milliseconds')
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
    # WebKit exposes the picker (a button with aria-haspopup) as a pop-up button.
    picker = [{'name': name(n), 'box': box(n)} for n in dump if n['role'] in ('AXButton', 'AXPopUpButton') and (name(n) or '').startswith('Play scene and camera')]
    notice = [name(n) for n in dump if 'could not start' in (name(n) or '') or 'stopped at tick' in (name(n) or '')]
    words = sorted({name(n) for n in dump if n['role'] == 'AXStaticText' and name(n) in (
        'Playing', 'Paused', 'Preparing…', 'Stopping…', 'Play failed', 'Play unavailable', 'Live', 'Suspended', 'Ready')})
    model = [{'value': n.get('value'), 'box': box(n)} for n in dump if n['role'] == 'AXTextField' and n.get('label') == 'Model']
    separators = [{'name': name(n) if n.get('label') else n.get('title'), 'value': n.get('value'), 'box': box(n)}
                  for n in dump if n['role'] == 'AXSplitter' or (n.get('label') or '').startswith('Resize')]
    account = [n.get('label') or n.get('title') for n in dump if 'ChatGPT account' in (n.get('label') or n.get('title') or '')]
    usage = [name(n) for n in dump if 'tokens used' in (name(n) or '')]
    menus = [name(n) for n in dump if n['role'] == 'AXMenu']
    items = [{'name': name(n), 'focused': n.get('focused', False)} for n in dump if n['role'] in ('AXMenuItem', 'AXMenuItemRadio') or n.get('sub') == 'AXMenuItemRadio']
    agent = [n.get('title') for n in dump if n['role'] == 'AXHeading' and (n.get('title') or '').startswith('Agent')]
    composer = {name(n): box(n) for n in dump if name(n) in ('Model', 'Approval', 'Send', 'Message to the agent') and n['role'] != 'AXStaticText'}
    print(json.dumps({
        'label': label,
        'at': at,
        # In the background the app may report no focused element at all ({"focused": null}).
        'focus': [{'role': n['role'], 'name': name(n), 'box': box(n)} for n in path[:4] if 'role' in n],
        'buttons': buttons,
        'picker': picker,
        'notice': notice,
        'words': words,
        'model': model,
        'separators': separators,
        'account': account,
        'usage': usage,
        'menus': menus,
        'menu_items': items,
        'agent': agent,
        'composer': composer,
        'nodes': len(dump),
    }, ensure_ascii=False))


if __name__ == '__main__':
    main()
