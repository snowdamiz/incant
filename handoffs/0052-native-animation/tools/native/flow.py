"""flow.py: keyboard steps of the 0052 review, run inside burst.sh (the window is in front).

Adapted from 0050 (`keys`, `tab-to`) with Play-specific steps:

  flow.py keys <code[:mods]>...   press keys in order, printing the focused element after each
  flow.py tab-to <name> [max]     press Tab until the focused element is named <name>
  flow.py wait <word> [seconds]   poll the header until a state word (Playing, Paused, Live...)
                                  is shown; prints when it appeared
  flow.py hold <seconds>          stay in front for that long (a running session advances only
                                  while the editor is frontmost); aborts if another app comes
                                  forward
  flow.py steps <n> [code[:mods]] press Step (option-command-P, or the given key, e.g. Space on a
                                  focused Step button) n times, waiting after each press until
                                  the paused tick has grown by exactly one

Every key goes through winctl, which refuses to post unless the review's PID is frontmost.
Prints one JSON line per observation.
"""

import datetime
import json
import re
import subprocess
import sys
import time

SCRATCH = '/tmp/incant0052'
WINCTL = f'{SCRATCH}/winctl'
PID = open(f'{SCRATCH}/main.pid').read().strip()
TAB = '48'
STEP = '35:cmd,alt'


def winctl(*args):
    out = subprocess.run([WINCTL, PID, *args], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        raise SystemExit(f'winctl {" ".join(args)} failed: {out.stderr.strip()}')
    return out.stdout


def frontmost():
    out = subprocess.run([WINCTL, 'frontmost'], capture_output=True, text=True, timeout=10)
    return out.stdout.strip()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='milliseconds')


def focused():
    node = json.loads(winctl('ax', 'focused'))
    return {'role': node.get('role'), 'name': node.get('label') or node.get('title') or node.get('value'),
            'dom': node.get('dom'), 'box': [node.get(k) for k in ('x', 'y', 'w', 'h')]}


def report(event, **extra):
    print(json.dumps({'at': now(), 'event': event, 'focus': focused(), **extra}, ensure_ascii=False), flush=True)


def press(spec):
    code, _, mods = spec.partition(':')
    winctl('key', code, mods or 'none')


def found(role, needle):
    return [json.loads(line) for line in winctl('ax', 'find', role, needle).splitlines() if line.strip()]


def name(node):
    return node.get('label') or node.get('title') or node.get('value')


def header():
    """The Viewport region's 36 px header line, where the Play state and paused tick are shown."""
    region = next(n for n in found('AXGroup', 'Viewport') if n.get('sub') == 'AXLandmarkRegion' and name(n) == 'Viewport')
    return region['x'], region['y'], region['w'], 36


def inside(node, area):
    x, y, w, h = area
    return node.get('x') is not None and x <= node['x'] < x + w and y <= node['y'] < y + h


def header_words(word):
    area = header()
    return [name(n) for n in found('AXStaticText', word) if inside(n, area)]


def ticks():
    """Every 'tick N' text: the header's paused tick and the Runtime block's read tick."""
    values = []
    for node in found('AXStaticText', 'tick '):
        match = re.fullmatch(r'tick ([\d,]+)', name(node) or '')
        if match:
            values.append(int(match.group(1).replace(',', '')))
    return values


def wait(word, seconds):
    start = time.monotonic()
    while time.monotonic() - start < seconds:
        if word in header_words(word):
            print(json.dumps({'at': now(), 'event': 'shown', 'word': word,
                              'after_s': round(time.monotonic() - start, 2)}), flush=True)
            return
        time.sleep(0.1)
    raise SystemExit(f'{word} not shown within {seconds} s')


def hold(seconds):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if frontmost() != PID:
            raise SystemExit('another app came forward; stopping the hold')
        time.sleep(0.1)
    print(json.dumps({'at': now(), 'event': 'held', 'seconds': seconds}), flush=True)


def steps(count, key=STEP):
    for index in range(count):
        before = ticks()
        if not before:
            raise SystemExit('no paused tick is shown')
        press(key)
        deadline = time.monotonic() + 3
        while True:
            after = ticks()
            if after and max(after) == max(before) + 1 and min(after) == max(after):
                break
            if time.monotonic() > deadline:
                raise SystemExit(f'step {index + 1}: tick did not advance by one ({before} -> {after})')
            time.sleep(0.05)
        print(json.dumps({'at': now(), 'event': 'step', 'key': key, 'index': index + 1, 'before': before, 'after': after, 'focus': focused()}), flush=True)


def main():
    command = sys.argv[1]
    if command == 'keys':
        for spec in sys.argv[2:]:
            press(spec)
            time.sleep(0.25)
            report('key', key=spec)
    elif command == 'tab-to':
        limit = int(sys.argv[3]) if len(sys.argv) > 3 else 8
        for _ in range(limit):
            press(TAB)
            time.sleep(0.2)
            report('tab')
            if focused()['name'] == sys.argv[2]:
                return
        raise SystemExit(f'{sys.argv[2]} not reached in {limit} Tabs')
    elif command == 'wait':
        wait(sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else 20)
    elif command == 'hold':
        hold(float(sys.argv[2]))
    elif command == 'steps':
        steps(int(sys.argv[2]), *sys.argv[3:4])
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main()
