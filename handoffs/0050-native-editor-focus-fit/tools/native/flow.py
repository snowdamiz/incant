"""flow.py: keyboard steps of the 0050 review, run inside burst.sh (the window is in front).

  flow.py choose <target>        open the scene/camera picker, move with Down to the item named
                                 <target>, press Return; checks focus came back to the picker
  flow.py keys <code[:mods]>...  press keys in order, printing the focused element after each
  flow.py watch <code[:mods]> <seconds>
                                 press one key, then print the focused element every 0.2 s
  flow.py tab-to <name> [max]    press Tab until the focused element is named <name>

Every key goes through winctl, which refuses to post unless the review's PID is frontmost.
Prints one JSON line per observation.
"""

import datetime
import json
import subprocess
import sys
import time

SCRATCH = '/tmp/incant0050'
WINCTL = f'{SCRATCH}/winctl'
PID = open(f'{SCRATCH}/main.pid').read().strip()
RETURN, DOWN, TAB = '36', '125', '48'


def winctl(*args):
    out = subprocess.run([WINCTL, PID, *args], capture_output=True, text=True, timeout=60)
    if out.returncode != 0:
        raise SystemExit(f'winctl {" ".join(args)} failed: {out.stderr.strip()}')
    return out.stdout


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


def choose(target):
    clicked = json.loads(winctl('ax', 'click', 'AXPopUpButton', 'Play scene and camera'))
    report('picker clicked', picker=clicked.get('label'))
    time.sleep(0.3)
    for _ in range(8):
        current = focused()
        # An item's spoken name can continue after its label (a scene's "No camera" note).
        spoken = current['name'] or ''
        if spoken.startswith(target) and not spoken[len(target):len(target) + 1].isalnum():
            press(RETURN)
            time.sleep(0.3)
            report('chosen', target=target)
            after = focused()
            if not (after['role'] == 'AXPopUpButton' and (after['name'] or '').endswith(f': {target}')):
                raise SystemExit(f'focus did not return to the picker: {after}')
            return
        press(DOWN)
        time.sleep(0.15)
    raise SystemExit(f'{target} not reached in the menu')


def main():
    command = sys.argv[1]
    if command == 'choose':
        choose(sys.argv[2])
    elif command == 'keys':
        for spec in sys.argv[2:]:
            press(spec)
            time.sleep(0.25)
            report('key', key=spec)
    elif command == 'watch':
        report('before', key=sys.argv[2])
        press(sys.argv[2])
        end = time.monotonic() + float(sys.argv[3])
        while time.monotonic() < end:
            time.sleep(0.2)
            report('after', key=sys.argv[2])
    elif command == 'tab-to':
        limit = int(sys.argv[3]) if len(sys.argv) > 3 else 8
        for _ in range(limit):
            press(TAB)
            time.sleep(0.2)
            report('tab')
            if focused()['name'] == sys.argv[2]:
                return
        raise SystemExit(f'{sys.argv[2]} not reached in {limit} Tabs')
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main()
