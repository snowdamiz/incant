#!/usr/bin/env python3
"""Show that tools/tests/test_planar_presentation.py rejects real regressions.

Each mutation edits one source file, runs the test module, then restores the
exact original bytes and verifies the hash. A mutation passes when the test
module fails. Commit first; never edit sources while this runs.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

WORKTREE = Path(__file__).resolve().parents[3]
MUTATIONS = [
    ('urchin on the route', 'samples/planar/art/cove.py',
     'URCHIN_AT = [(5.5, 8.5), (14.5, 7.5), (13.5, 3.5)]',
     'URCHIN_AT = [(5.5, 8.5), (12.5, 7.5), (13.5, 3.5)]'),
    ('pearl inside a boulder', 'samples/planar/art/cove.py',
     'PEARL_AT = [(3.5, 8.5), (16.5, 8.5), (17.5, 4.5)]',
     'PEARL_AT = [(3.5, 8.5), (15.5, 6.5), (17.5, 4.5)]'),
    ('color above the tone-map identity range', 'samples/planar/art/palette.py',
     'SAND_LIGHT = (226, 208, 166)', 'SAND_LIGHT = (240, 208, 166)'),
    ('gravity on the player body', 'samples/planar/scene.py',
     "'RigidBody2D': {'motion': 'dynamic', 'gravity_scale': 0,",
     "'RigidBody2D': {'motion': 'dynamic', 'gravity_scale': 1,"),
    ('pickup rendered as a solid', 'samples/planar/scene.py',
     "'Transform': transform(*at), 'Collider2D': collider(PEARL_RADIUS, True),",
     "'Transform': transform(*at), 'Collider2D': collider(PEARL_RADIUS, False),"),
    # HUD revision: each of these would ship a wrong or unlocalizable HUD.
    ('static result labels bound', 'samples/planar/hud.py',
     "'hud': {'table': TABLE, 'status': STATUS, 'prompt': PROMPT}})",
     "'hud': {'table': TABLE, 'status': STATUS, 'prompt': PROMPT}, 'results': {}})"),
    ('a translation missing', 'samples/planar/hud.py',
     "        'de': 'Von einem Seeigel gestochen',\n", ''),
    ('non-canonical HUD identity', 'samples/planar/hud.py',
     "cove.ulid('HD', n)", "cove.ulid('UI', n)"),
    ('static text in a live label', 'samples/planar/hud.py',
     "{'content': '', 'fonts'", "{'content': 'Pearls 0 of 3', 'fonts'"),
    ('unknown message argument', 'samples/planar/hud.py',
     "'en': 'Pearls {collected} of {total}',", "'en': 'Pearls {count} of {total}',"),
]


def main():
    results = []
    for name, relative, before, after in MUTATIONS:
        path = WORKTREE / relative
        original = path.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode()
        if text.count(before) != 1:
            raise SystemExit(f'{name}: anchor is not unique in {relative}')
        try:
            path.write_text(text.replace(before, after))
            run = subprocess.run([sys.executable, '-B', '-m', 'unittest', 'tools/tests/test_planar_presentation.py'],
                                 cwd=WORKTREE, capture_output=True, text=True, timeout=300)
        finally:
            path.write_bytes(original)
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise SystemExit(f'{relative} was not restored')
        failures = [line for line in run.stderr.splitlines() if line.startswith(('FAIL:', 'ERROR:'))]
        results.append({'mutation': name, 'file': relative, 'detected': run.returncode != 0, 'failures': failures})
    clean = subprocess.run([sys.executable, '-B', '-m', 'unittest', 'tools/tests/test_planar_presentation.py'],
                           cwd=WORKTREE, capture_output=True, text=True, timeout=300)
    print(json.dumps({'mutations': results, 'unmutated_passes': clean.returncode == 0}, indent=2))
    return 0 if clean.returncode == 0 and all(r['detected'] for r in results) else 1


if __name__ == '__main__':
    sys.exit(main())
