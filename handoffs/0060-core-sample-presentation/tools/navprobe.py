#!/usr/bin/env python3
"""navprobe.py OUT --binary INCANT_HEADLESS: query the level navmesh between test points.

Builds a disposable project (public init + one RPC transaction) holding only the
Core Sample's collision entities and NavigationMesh from samples/core/art/level.py,
plus a one-tick script that calls navigation.findPath for each probe pair and logs
the corner count. Development diagnostic only; nothing here ships.
"""
import argparse
import json
import runpy
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'samples/core'))
from art import ids, level  # noqa: E402

PAIRS = {
    'floor-floor': [(-0.5, 0.62, 10.0), (3.3, 0.62, -10.3)],
    'plateau-plateau': [(-4.2, 2.22, -16.0), (8.2, 2.22, -15.6)],
    'east-ramp-mid-to-floor': [(7.6, 1.2, 0.0), (7.6, 0.62, 5.0)],
    'east-ramp-mid-to-shelf': [(7.6, 1.2, 0.0), (8.6, 2.22, -6.0)],
    'west-ramp-mid-to-floor': [(-9.2, 1.4, -9.7), (-8.0, 0.62, -4.0)],
    'west-ramp-mid-to-plateau': [(-9.2, 1.4, -9.7), (-8.0, 2.22, -16.0)],
    'plateau-to-floor': [(-4.2, 2.22, -16.0), (-0.5, 0.62, 10.0)],
    'shelf-to-floor': [(8.6, 2.22, -8.0), (7.6, 0.62, 5.0)],
    'crash-west-ramp-to-east-floor': [(-8.574, 0.867, -7.838), (2.296, 0.606, -0.678)],
    'rim-west-to-camp': [(-10.0, 2.22, -20.0), (6.0, 0.62, 12.0)],
    'rim-east-to-start': [(10.0, 2.22, -20.0), (-0.5, 0.62, 12.5)],
    'shelf-corner-to-west-floor': [(11.0, 2.22, -3.0), (-9.8, 0.62, 12.5)],
}
SCRIPT = """import type { NativeBehavior } from '@incant/runtime/native';
const PAIRS: [string, [number, number, number], [number, number, number]][] = %s;
export default { update({ navigation, log, clock }) {
  if (clock.tick !== 1) return undefined;
  for (const [name, start, end] of PAIRS) {
    const path = navigation.findPath('%s', { start, end, snap_distance: 2, max_visited: 32768 });
    log(JSON.stringify({ name, corners: path ? path.points.length : -1, visited: path ? path.visited : null,
      corridor: path ? path.corridor.length : null,
      points: path ? path.points.map((p) => p.map((v) => Math.round(v * 100) / 100)) : null }));
  }
  return undefined;
} } satisfies NativeBehavior;
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--pair', action='append', help='probe only these pairs (one process each)')
    args = parser.parse_args()
    out = args.output.absolute().resolve()
    binary = args.binary.resolve(strict=True)
    if args.pair is None:
        for name in PAIRS:
            result = subprocess.run([sys.executable, '-B', __file__, str(out / name), '--binary', str(binary),
                                     '--pair', name], text=True, capture_output=True)
            print(result.stdout.strip() or (name + ' FAILED ' + result.stderr.strip()[-200:]))
        return
    out.mkdir(parents=True)
    selected = {k: PAIRS[k] for k in args.pair}
    project = out / 'nav.json'
    subprocess.run([str(binary), 'init', str(project), '--entities', '0'], check=True, capture_output=True)
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    rpc = Rpc(binary, project, out / 'nav.journal.jsonl')
    nav, script = ids.entity(5), ids.make('script', 91)
    try:
        read = rpc.call('project.read')
        scene = next(iter(read['project']['scenes']))
        colliders = level.colliders()
        commands = [
            {'op': 'write_script', 'script_id': script, 'expected_source': {'state': 'missing'},
             'source': SCRIPT % (json.dumps([[k, *v] for k, v in selected.items()]), nav)},
            {'op': 'upsert_script', 'script': {'id': script, 'name': 'Nav probe', 'path': 'nav.ts'}}]
        names = {}
        for i, (name, parts) in enumerate(colliders.items()):
            ident = ids.entity(6 + i)
            names[name] = ident
            commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': ident, 'name': name, 'parent': None, 'provenance': None, 'components': {
                    'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                    'Collider': {'shape': {'type': 'compound', 'parts': parts}, 'density': 1000, 'friction': 0.5,
                                 'restitution': 0, 'sensor': False, 'memberships': 4294967295,
                                 'filter': 4294967295}}}})
        commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': nav, 'name': 'Navigation', 'parent': None, 'provenance': None, 'components': {
                'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                'NavigationMesh': {'settings': level.navigation_settings(), 'links': [],
                                   'sources': [{'entity': e, 'geometry': 'collider'} for e in names.values()]}}}})
        commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': ids.entity(99), 'name': 'Probe', 'parent': None, 'provenance': None, 'components': {
                'Script': {'source': script, 'props': {}}}}})
        rpc.call('command.execute', {'expected_revision': read['revision'], 'description': 'Nav probe',
                                     'commands': commands})
        rpc.call('project.save')
    finally:
        rpc.close()
    result = subprocess.run([str(binary), 'native-play', str(project), '--ticks', '1'], text=True,
                            capture_output=True, timeout=300)
    if result.returncode:
        raise SystemExit(result.stderr[-3000:])
    report = json.loads(result.stdout)
    rows = [json.loads(e['message']) for e in report['logs']['entries']]
    (out / 'paths.json').write_text(json.dumps(rows, indent=1) + '\n', encoding='utf-8')
    for row in rows:
        print(row['name'], 'corners', row['corners'], 'visited', row['visited'], 'corridor', row['corridor'])


if __name__ == '__main__':
    main()
