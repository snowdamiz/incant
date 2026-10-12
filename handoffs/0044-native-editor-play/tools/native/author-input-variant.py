#!/usr/bin/env python3
"""Authors the 0044 native input-review fixture through public incant_cmd RPC only.

A second disposable project beside the granted fixture, for native review of game-input
routing, the scene/camera picker and engine error placement:

- scene "Main": Key Probe (X counts ticks with KeyD held, Y ticks with the left button
  held, Z sums the vertical wheel; scale X is 2 while the game has focus), Pointer Probe
  (translation X, Y = the pointer position the game received), and two cameras;
- scene "Lit Without Camera": one point light and no camera, which the engine refuses
  to start (spatial content needs an explicit camera).

`chunks` are world-wide views (sdk/ts/src/native.d.ts), so each probe changes only the
entities its source is attached to (`instances`). The first fixture, `input-variant`, ran
both probes over every entity, so the pointer probe overwrote the key probe's X and Y;
this corrected version writes `input-variant-2`.

Nothing is written around the command bus: `incant_headless init`, then two
`command.execute` transactions (the bus takes one script source write per transaction)
and `project.save`, as tools/probes/native-editor-play.py does. Run from the repository root:

  python3 -I handoffs/0044-native-editor-play/tools/native/author-input-variant.py \
      --headless /tmp/incant-native-review-0044/incant_headless
"""
import argparse
import json
import runpy
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
OUT = ROOT / 'artifacts/native-review-0044/input-variant-2'

KEY_SCRIPT, POINTER_SCRIPT = '00000000000000000000000011', '00000000000000000000000012'
KEY_PROBE, POINTER_PROBE = '00000000000000000000000021', '00000000000000000000000022'
CAMERA, SIDE_CAMERA = '00000000000000000000000031', '00000000000000000000000032'
LIT_SCENE, LAMP = '00000000000000000000000041', '00000000000000000000000042'

KEY_SOURCE = """import type { NativeBehavior } from '@incant/runtime/native';
// 0044 review probe. X: ticks with KeyD held. Y: ticks with the left button held.
// Z: summed vertical wheel (pixels and lines). Scale X: 2 while the game has focus.
export default { update({ instances, chunks, input }) {
  const right = input.keyboard.held.includes('KeyD') ? 1 : 0;
  const up = input.mouse.buttons.held.includes('left') ? 1 : 0;
  const wheel = input.mouse.wheel_pixels[1] + input.mouse.wheel_lines[1];
  for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++) {
    if (!instances.some((instance) => instance.entity === chunk.ids[i])) continue;
    const t = i * chunk.transformStride;
    chunk.transforms[t] += right;
    chunk.transforms[t + 1] += up;
    chunk.transforms[t + 2] += wheel;
    chunk.transforms[t + 7] = input.focused ? 2 : 1;
  }
} } satisfies NativeBehavior;
"""

POINTER_SOURCE = """import type { NativeBehavior } from '@incant/runtime/native';
// 0044 review probe. Translation X, Y: the pointer position the game received.
export default { update({ instances, chunks, input }) {
  const at = input.mouse.position;
  if (!at) return;
  for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++) {
    if (!instances.some((instance) => instance.entity === chunk.ids[i])) continue;
    const t = i * chunk.transformStride;
    chunk.transforms[t] = at[0];
    chunk.transforms[t + 1] = at[1];
  }
} } satisfies NativeBehavior;
"""


def transform(translation=(0, 0, 0)):
    return {'translation': list(translation), 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--headless', type=Path, required=True)
    args = parser.parse_args()
    module = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))
    Rpc, require = module['Rpc'], module['require']
    OUT.mkdir(parents=True, exist_ok=False)
    project, journal = OUT / 'project.json', OUT / 'project.journal.jsonl'
    subprocess.run([str(args.headless), 'init', str(project), '--entities', '0', '--name', 'Native Play Input Review'],
                   check=True, capture_output=True)
    scene = next(iter(json.loads(project.read_text())['scenes']))
    camera = {'projection': {'kind': 'perspective'}, 'fov_degrees': 60, 'near': 0.1, 'far': 100}

    def entity(scene_id, identity, name, components):
        return {'op': 'create_entity', 'scene_id': scene_id, 'entity': {
            'id': identity, 'name': name, 'parent': None, 'provenance': None, 'components': components}}

    rpc = Rpc(args.headless, project, journal)
    try:
        # The bus takes one script source write per transaction: two transactions.
        first = rpc.call('command.execute', {'expected_revision': 0, 'description': 'Author native Play input review fixture', 'commands': [
            {'op': 'write_script', 'script_id': KEY_SCRIPT, 'source': KEY_SOURCE, 'expected_source': {'state': 'missing'}},
            {'op': 'upsert_script', 'script': {'id': KEY_SCRIPT, 'name': 'Key Probe', 'path': 'key-probe.ts'}},
            entity(scene, KEY_PROBE, 'Key Probe', {'Transform': transform(), 'Script': {'source': KEY_SCRIPT, 'props': {}}}),
            entity(scene, CAMERA, 'Camera', {'Transform': transform((0, 2, -8)), 'Camera': camera}),
            entity(scene, SIDE_CAMERA, 'Side Camera', {'Transform': transform((8, 2, 0)), 'Camera': camera}),
            {'op': 'create_scene', 'scene': {'id': LIT_SCENE, 'name': 'Lit Without Camera', 'entities': {}}},
            entity(LIT_SCENE, LAMP, 'Lamp', {'Transform': transform((0, 3, 0)),
                                             'PointLight': {'color': [1, 0.8, 0.5], 'intensity': 200, 'range': 8}}),
        ]})
        require(first['revision'] == 1, 'first fixture transaction did not apply once')
        second = rpc.call('command.execute', {'expected_revision': 1, 'description': 'Add the pointer probe', 'commands': [
            {'op': 'write_script', 'script_id': POINTER_SCRIPT, 'source': POINTER_SOURCE, 'expected_source': {'state': 'missing'}},
            {'op': 'upsert_script', 'script': {'id': POINTER_SCRIPT, 'name': 'Pointer Probe', 'path': 'pointer-probe.ts'}},
            entity(scene, POINTER_PROBE, 'Pointer Probe', {'Transform': transform(), 'Script': {'source': POINTER_SCRIPT, 'props': {}}}),
        ]})
        require(second['revision'] == 2, 'second fixture transaction did not apply once')
        require(rpc.call('project.save')['saved'], 'fixture save failed')
    finally:
        rpc.close()
    print(json.dumps({'project': str(project.relative_to(ROOT)), 'revision': 2}))


if __name__ == '__main__':
    main()
