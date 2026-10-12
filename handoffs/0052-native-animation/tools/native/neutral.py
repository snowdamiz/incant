"""neutral.py: print a window-relative point ("x y") on empty titlebar, where a parked pointer
hovers nothing: midway between the last panel toggle and the account button. Read-only.

Copied from 0050, whose first three captures were taken with the pointer parked inside the Undo
button, so its hover fill shows in them.
"""

import json
import subprocess

SCRATCH = '/tmp/incant0052'
PID = open(f'{SCRATCH}/main.pid').read().strip()
out = subprocess.run([f'{SCRATCH}/winctl', PID, 'ax', 'dump', '8'], capture_output=True, text=True, timeout=60, check=True)
nodes = [json.loads(line) for line in out.stdout.splitlines() if line.strip()]
toggle = next(n for n in nodes if n.get('title') == 'Inspector and agent panel')
account = next(n for n in nodes if 'ChatGPT account' in (n.get('title') or ''))
left, right = toggle['x'] + toggle['w'], account['x']
if right - left < 40:
    raise SystemExit(f'no empty titlebar between {left} and {right}')
print(f'{(left + right) / 2:.0f} 20')
