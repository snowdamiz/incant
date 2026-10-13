#!/usr/bin/env python3
"""lookdev.py BUILD OUT_DIR NAME:X,Y,Z,PITCH,YAW ...: offscreen look-dev views of a built level.

Copies BUILD/project to a scratch directory, adds one temporary Camera per view through an
RPC command.execute transaction on that copy (the original project is never touched), saves,
and renders each view with `incant screenshot --camera` (authoring renderer; diagnostic cubes
for Transform-only entities appear there and nowhere else). Development aid only.
"""
import json
import math
import runpy
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BIN = Path.home() / '.codex/incant-evidence/0060/bin/incant_headless'


def quat(pitch, yaw):
    sp, cp = math.sin(math.radians(pitch) / 2), math.cos(math.radians(pitch) / 2)
    sy, cy = math.sin(math.radians(yaw) / 2), math.cos(math.radians(yaw) / 2)
    return [cy * sp, sy * cp, -sy * sp, cy * cp]


def main():
    build, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).absolute()
    out.mkdir(parents=True, exist_ok=True)
    scratch = out / 'scratch-project'
    if scratch.exists():
        shutil.rmtree(scratch)
    shutil.copytree(build / 'project', scratch)
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    rpc = Rpc(BIN, scratch / 'core.json', scratch / 'core.journal.jsonl')
    views = []
    try:
        read = rpc.call('project.read')
        scene = next(iter(read['project']['scenes']))
        commands = []
        for k, spec in enumerate(sys.argv[3:]):
            name, _, values = spec.partition(':')
            x, y, z, pitch, yaw = map(float, values.split(','))
            ident = '01CSEN' + '%020d' % (800 + k)
            views.append((name, ident))
            commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': ident, 'name': 'Look-dev ' + name, 'parent': None, 'provenance': None, 'components': {
                    'Transform': {'translation': [x, y, z], 'rotation': quat(pitch, yaw), 'scale': [1, 1, 1]},
                    'Camera': {'projection': {'kind': 'perspective'}, 'fov_degrees': 50.0, 'near': 0.1,
                               'far': 520.0}}}})
        rpc.call('command.execute', {'expected_revision': read['revision'], 'description': 'Look-dev cameras',
                                     'commands': commands})
        rpc.call('project.save')
    finally:
        rpc.close()
    for name, ident in views:
        png = out / f'{name}.png'
        png.unlink(missing_ok=True)
        result = subprocess.run([str(BIN), 'screenshot', str(scratch / 'core.json'), str(png), '--camera', ident,
                                 '--width', '1280', '--height', '960'], capture_output=True, text=True)
        stats = json.loads(result.stdout)['geometry'] if result.returncode == 0 else result.stderr[-300:]
        print(name, png, stats if isinstance(stats, str) else (stats['model_draw_calls'], stats['model_triangles']))


if __name__ == '__main__':
    main()
