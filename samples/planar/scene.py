#!/usr/bin/env python3
"""Author the Pearl Cove presentation of the Planar Sample through public commands.

    python3 -B samples/planar/scene.py <new-output> --binary <incant_headless> [--play]

Generates the original art (samples/planar/art), imports the atlas and the HUD
font with the public importer, then authors the project only through RPC command
transactions: controller and input, terrain and camera, actors, then the HUD
(string table, UI nodes and the controller's presentation bindings). It checks
Undo/Redo, save and journal reopening. --play also runs native-play scenarios on
this layout with a UI viewport and checks that Play leaves the authored files
unchanged. No GPU or device claim.
"""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from art import build as art_build, cove  # noqa: E402
import hud  # noqa: E402

SPEED = 3.0
PLAYER_RADIUS, PEARL_RADIUS, URCHIN_RADIUS, EXIT_RADIUS = 0.3, 0.35, 0.4, 0.4
FULL = 4294967295


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


def transform(x, y, z=0.0):
    return {'translation': [x, y, z], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


def collider(radius, sensor):
    return {'shape': {'kind': 'circle', 'radius': radius}, 'density': 1000, 'friction': 0,
            'restitution': 0, 'sensor': sensor, 'memberships': FULL, 'filter': FULL}


def sprite(texture, name, size, pivot, layer, order=0):
    return {'texture': texture, 'region': {'frame': cove.frame_id(name)}, 'size': size,
            'pivot': pivot, 'tint': [1, 1, 1, 1], 'layer': layer, 'order': order,
            'sampling': 'nearest'}


def flipbook(names, fps, playing=True):
    return {'frames': [{'frame': cove.frame_id(n)} for n in names],
            'rate': {'numerator': fps, 'denominator': 1}, 'looping': playing, 'playing': playing}


# Paused state frames: frame 0 is what the current controller shows. The other
# frames are ready for the controller hook in PRESENTATION.md (seek on a state
# transition); nothing selects them today.
POSES = ['beachcomber/0', 'beachcomber/1', 'beachcomber/2']  # default, stung, cheering
BOAT = ['boat/0', 'boat/1']  # lantern dark, lantern lit (all pearls collected)


def entity(scene, ident, name, components):
    return {'op': 'create_entity', 'scene_id': scene, 'entity': {
        'id': ident, 'name': name, 'parent': None, 'provenance': None, 'components': components}}


def input_actions():
    keys = [('KeyD', [1, 0]), ('KeyA', [-1, 0]), ('KeyW', [0, 1]), ('KeyS', [0, -1]),
            ('ArrowRight', [1, 0]), ('ArrowLeft', [-1, 0]), ('ArrowUp', [0, 1]), ('ArrowDown', [0, -1])]
    pad = [('dpad_right', [1, 0]), ('dpad_left', [-1, 0]), ('dpad_up', [0, 1]), ('dpad_down', [0, -1])]
    return {'move': {'kind': 'axis2', 'dead_zone': 0.1, 'bindings': (
        [{'type': 'key', 'code': code, 'scale': scale} for code, scale in keys]
        + [{'type': 'gamepad_stick', 'id': 0, 'stick': 'left', 'invert_y': True}]
        + [{'type': 'gamepad_button', 'id': 0, 'button': b, 'scale': s} for b, s in pad])}}


def terrain_commands(scene, texture):
    layers, _ = cove.tile_layers()
    camera = {'fov_degrees': 60, 'near': 0.1, 'far': 100, 'projection': {
        'kind': 'pixel_perfect', 'pixels_per_unit': cove.PIXELS_PER_METRE,
        'reference_resolution': list(cove.REFERENCE_RESOLUTION)}}
    return [
        entity(scene, cove.TERRAIN, 'Cove terrain', {'Transform': transform(0, 0), 'Tilemap': {
            'tileset': texture, 'size': [cove.WIDTH, cove.HEIGHT], 'cell_size': [1, 1],
            'layers': layers}}),
        entity(scene, cove.CAMERA, 'Cove camera', {'Transform': transform(*cove.CAMERA_AT),
                                                    'Camera': camera}),
    ]


def actor_commands(scene, texture):
    commands = [entity(scene, cove.PLAYER, 'Beachcomber', {
        'Transform': transform(*cove.START), 'Velocity': {'linear': [0, 0, 0]},
        'RigidBody2D': {'motion': 'dynamic', 'gravity_scale': 0, 'linear_damping': 0,
                        'angular_damping': 0, 'can_sleep': True, 'ccd': True},
        'Collider2D': collider(PLAYER_RADIUS, False),
        'SpriteRenderer': sprite(texture, POSES[0], [1, 1], [0.5, 0.25], 2),
        'SpriteFlipbook': flipbook(POSES, 1, playing=False)})]
    twinkle = ['clam/0'] * 5 + ['clam/1', 'clam/2', 'clam/3']
    for index, (ident, at) in enumerate(zip(cove.PEARLS, cove.PEARL_AT)):
        phase = twinkle[index * 3:] + twinkle[:index * 3]  # pearls twinkle in turn
        commands.append(entity(scene, ident, f'Pearl {index + 1}', {
            'Transform': transform(*at), 'Collider2D': collider(PEARL_RADIUS, True),
            'SpriteRenderer': sprite(texture, phase[0], [1, 1], [0.5, 0.5], 1),
            'SpriteFlipbook': flipbook(phase, 8)}))
    for index, (ident, at) in enumerate(zip(cove.URCHINS, cove.URCHIN_AT)):
        frames = ['urchin/0', 'urchin/1'] if index % 2 == 0 else ['urchin/1', 'urchin/0']
        commands.append(entity(scene, ident, f'Sea urchin {index + 1}', {
            'Transform': transform(*at), 'Collider2D': collider(URCHIN_RADIUS, True),
            'SpriteRenderer': sprite(texture, frames[0], [1, 1], [0.5, 0.5], 1),
            'SpriteFlipbook': flipbook(frames, 2)}))
    commands.append(entity(scene, cove.BOAT, 'Rowboat exit', {
        'Transform': transform(*cove.EXIT_AT), 'Collider2D': collider(EXIT_RADIUS, True),
        'SpriteRenderer': sprite(texture, BOAT[0], [2, 1], [0.5, 1.0], 1),
        'SpriteFlipbook': flipbook(BOAT, 1, playing=False)}))
    shimmer = ['glint/3'] * 6 + ['glint/0', 'glint/1', 'glint/2', 'glint/1', 'glint/0']
    for index, (ident, at) in enumerate(zip(cove.GLINTS, cove.GLINT_AT)):
        phase = shimmer[index * 3:] + shimmer[:index * 3]
        commands.append(entity(scene, ident, f'Water glint {index + 1}', {
            'Transform': transform(*at),
            'SpriteRenderer': sprite(texture, phase[0], [0.5, 0.5], [0.5, 0.5], 1),
            'SpriteFlipbook': flipbook(phase, 6 + index % 2)}))
    # The controller has no Transform: it is not spatial, so authoring previews
    # draw no placeholder cube for it.
    commands.append(entity(scene, cove.CONTROLLER, 'Planar gameplay', {'Script': controller()}))
    return commands


def controller():
    """Gameplay roles only; the HUD transaction adds presentation bindings."""
    return {'source': cove.SCRIPT, 'props': {
        'player': cove.PLAYER, 'exit': cove.BOAT, 'pickups': cove.PEARLS,
        'hazards': cove.URCHINS, 'speed': SPEED}}


class Runner:
    """Records every child process (argv, exit status, output) in the output."""

    def __init__(self, out, binary):
        self.out, self.binary, self.count = out, binary, 0

    def run(self, *argv):
        argv = [str(a) for a in (self.binary, *argv)]
        result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=300)
        write(self.out / f'process-{self.count:02d}.json', {
            'argv': argv, 'exit': result.returncode, 'stdout': result.stdout[-20000:],
            'stderr': result.stderr[-20000:]})
        self.count += 1
        if result.returncode:
            raise RuntimeError(result.stderr[-4000:])
        return json.loads(result.stdout)


def author(out, runner):
    project_dir = out / 'project'
    project_dir.mkdir()
    project, journal = project_dir / 'planar.json', project_dir / 'planar.journal.jsonl'
    runner.run('init', project, '--entities', 0, '--name', 'Pearl Cove')
    art = art_build.build(project_dir / 'art')
    # The importer assigns asset IDs; frame IDs come from the atlas descriptor.
    texture = runner.run('import', project, 'art/cove.atlas.json')['asset']['id']
    fonts = project_dir / 'fonts'
    fonts.mkdir()
    shutil.copyfile(ROOT / hud.FONT_SOURCE, fonts / Path(hud.FONT_SOURCE).name)
    shutil.copyfile(ROOT / hud.FONT_LICENSE, fonts / Path(hud.FONT_LICENSE).name)
    font = runner.run('import', project, 'fonts/' + Path(hud.FONT_SOURCE).name)['asset']['id']
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    calls = []
    rpc = Rpc(runner.binary, project, journal)

    def call(method, params=None):
        value = rpc.call(method, params)
        calls.append({'method': method, 'params': params,
                      'result_revision': value.get('revision') if isinstance(value, dict) else None})
        return value

    try:
        before = call('project.read')
        scene = next(iter(before['project']['scenes']))
        steps = [
            ('Add Planar Sample controller and input', [
                {'op': 'write_script', 'script_id': cove.SCRIPT, 'expected_source': {'state': 'missing'},
                 'source': (HERE / 'gameplay.ts').read_text()},
                {'op': 'upsert_script', 'script': {'id': cove.SCRIPT, 'name': 'Planar Sample',
                                                   'path': 'gameplay.ts'}},
                {'op': 'set_input_actions', 'actions': input_actions()}]),
            ('Lay out Pearl Cove terrain and camera', terrain_commands(scene, texture)),
            ('Place Pearl Cove actors and controller', actor_commands(scene, texture)),
            ('Add Pearl Cove HUD and localized strings', hud.commands(scene, font, controller())),
        ]
        transactions = []
        for description, commands in steps:
            revision = call('project.read')['revision']
            result = call('command.execute', {'expected_revision': revision,
                                              'description': description, 'commands': commands})
            transactions.append({'description': description, 'id': result['transaction_id'],
                                 'commands': len(commands)})
        authored = call('project.read')
        for _ in steps:
            call('history.undo')
        if call('project.read')['project'] != before['project'] or (project_dir / 'gameplay.ts').exists():
            raise RuntimeError('Undo did not restore the initial project')
        for _ in steps:
            call('history.redo')
        if call('project.read')['project'] != authored['project']:
            raise RuntimeError('Redo did not restore the authored project')
        history = call('history.read')
        call('project.save')
        authored = call('project.read')
    finally:
        rpc.close()
    rpc = Rpc(runner.binary, project, journal)
    try:
        if rpc.call('project.read') != authored:
            raise RuntimeError('journal reopen differs from the saved project')
    finally:
        rpc.close()
    validation = runner.run('validate', project)
    write(out / 'authoring-calls.json', calls)
    return {'project': project, 'scene': scene, 'texture': texture, 'font': font, 'art': art,
            'transactions': transactions, 'history': history, 'validation': validation}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--play', action='store_true', help='also run native-play scenarios')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    runner = Runner(out, args.binary.resolve(strict=True))
    binary_sha = digest(runner.binary)
    authored = author(out, runner)
    project_dir = authored['project'].parent
    files = {str(p.relative_to(project_dir)): digest(p)
             for p in sorted(project_dir.rglob('*')) if p.is_file() and '.incant' not in p.parts}
    result = {'passed': True, 'binary_sha256': binary_sha, 'project': str(authored['project']),
              'scene': authored['scene'], 'camera': cove.CAMERA, 'atlas': authored['texture'],
              'font': authored['font'], 'hud': {'root': hud.ROOT, 'status': hud.STATUS,
                                                'prompt': hud.PROMPT, 'table': hud.TABLE},
              'gameplay_sha256': digest(HERE / 'gameplay.ts'), 'art_files': authored['art']['files'],
              'tile_frames': authored['art']['tile_frames'],
              'sprite_frames': authored['art']['sprite_frames'],
              'transactions': authored['transactions'], 'history': authored['history'],
              'undo_redo_reopen': True, 'authored_files': files}
    if args.play:
        playtest = runpy.run_path(str(HERE / 'playtest.py'))
        result['scenarios'] = playtest['run_all'](out, runner, authored['project'])
        after = {str(p.relative_to(project_dir)): digest(p)
                 for p in sorted(project_dir.rglob('*')) if p.is_file() and '.incant' not in p.parts}
        if after != files or digest(runner.binary) != binary_sha:
            raise RuntimeError('Play changed the authored project or the tool')
        result['play_left_project_unchanged'] = True
    write(out / 'result.json', result)
    print(json.dumps({'passed': True, 'output': str(out), 'project': str(authored['project'])}))


if __name__ == '__main__':
    main()
