#!/usr/bin/env python3
"""Build the Core Sample presentation level as an ordinary Incant project.

Usage:
  python3 -B samples/core/scene.py OUTPUT --binary INCANT_HEADLESS [--art ART_DIR]
      [--locale en|en-XA|ar|zh-Hans] [--state playing|collected|won|lost]

OUTPUT must not exist. Every project change goes through the public engine:
`incant init`, `incant import` (one import transaction per source) and RPC
`command.execute` transactions with the bus's own user provenance. No project
JSON is written by this script. Source art is generated first (or reused from
--art after its manifest digests are checked) under OUTPUT/project/art.

After authoring, every transaction is undone and redone through RPC history and
compared with the recorded snapshots, the project is saved, and a fresh RPC
process must reopen it identically. The receipt (OUTPUT/receipt.json) records
IDs, digests and the transaction list. `--locale`/`--state` add one final,
undoable presentation transaction (UI text in that locale, the selected HUD
state); gameplay bindings themselves remain Astra's integration work.
"""
import argparse
import hashlib
import json
import math
import runpy
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from art import fontcheck, ids, level, sky  # noqa: E402
import hud  # noqa: E402
import bindings  # noqa: E402

SCRIPT = ids.make('script', 1)
SCENE = ids.make('scene', 1)
E = {name: ids.entity(i) for i, name in enumerate([
    'controller', 'player', 'camera', 'exit', 'navigation', 'ground', 'walls', 'rocks', 'canyon', 'sky',
    'environment', 'sun', 'lantern', 'beacon'], start=1)}
ENEMIES = [ids.entity(20 + i) for i in range(len(level.ENEMIES))]
SAMPLES = [ids.entity(30 + i) for i in range(len(level.SAMPLES))]
ASSETS = {'canyon': (ids.asset(1), 'art/canyon.gltf'), 'surveyor': (ids.asset(2), 'art/surveyor.gltf'),
          'mite': (ids.asset(3), 'art/mite.gltf'), 'sample': (ids.asset(4), 'art/sample.gltf'),
          'sky': (ids.asset(5), 'art/sky.gltf'), 'environment': (ids.asset(6), 'art/sky_environment.png'),
          'inter': (ids.asset(10), 'fonts/Inter-Regular.ttf'), 'arabic': (ids.asset(11), 'fonts/NotoSansArabic.ttf'),
          'sc': (ids.asset(12), 'fonts/CoreSampleSC-Regular.ttf'),
          'music': (ids.asset(20), 'art/mesa_loop.wav'), 'cue_pickup': (ids.asset(21), 'art/cue_pickup.wav'),
          'cue_hurt': (ids.asset(22), 'art/cue_hurt.wav'), 'cue_won': (ids.asset(23), 'art/cue_won.wav'),
          'cue_lost': (ids.asset(24), 'art/cue_lost.wav')}
AUDIO = {'music_bus': ids.entity(40), 'effects_bus': ids.entity(41), 'music': ids.entity(42),
         'pickup': [ids.entity(43 + i) for i in range(4)], 'hurt': [ids.entity(47 + i) for i in range(3)],
         'won': ids.entity(50), 'lost': ids.entity(51)}
FONT_SOURCES = {'inter': ROOT / 'third_party/cosmic-text/fonts/Inter-Regular.ttf',
                'arabic': ROOT / 'third_party/cosmic-text/fonts/NotoSansArabic.ttf',
                'sc': HERE / 'art/fonts/CoreSampleSC-Regular.ttf'}
SETTINGS = {'speed': 3.4, 'enemySpeed': 1.7, 'jumpSpeed': 6.0}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def transform(at, rotation=(0.0, 0.0, 0.0, 1.0)):
    return {'translation': [round(v, 6) for v in at], 'rotation': [round(v, 9) for v in rotation],
            'scale': [1.0, 1.0, 1.0]}


def look_rotation(direction):
    """Unit quaternion whose local -Z points along `direction` (Y up)."""
    fx, fy, fz = direction
    n = math.sqrt(fx * fx + fy * fy + fz * fz)
    zx, zy, zz = -fx / n, -fy / n, -fz / n
    ux, uy, uz = 0.0, 1.0, 0.0
    xx, xy, xz = uy * zz - uz * zy, uz * zx - ux * zz, ux * zy - uy * zx
    m = math.sqrt(xx * xx + xy * xy + xz * xz)
    xx, xy, xz = xx / m, xy / m, xz / m
    yx, yy, yz = zy * xz - zz * xy, zz * xx - zx * xz, zx * xy - zy * xx
    trace = xx + yy + zz
    if trace > 0:
        s = math.sqrt(trace + 1.0) * 2
        return ((yz - zy) / s, (zx - xz) / s, (xy - yx) / s, 0.25 * s)
    if xx > yy and xx > zz:
        s = math.sqrt(1.0 + xx - yy - zz) * 2
        return (0.25 * s, (yx + xy) / s, (zx + xz) / s, (yz - zy) / s)
    if yy > zz:
        s = math.sqrt(1.0 + yy - xx - zz) * 2
        return ((yx + xy) / s, 0.25 * s, (zy + yz) / s, (zx - xz) / s)
    s = math.sqrt(1.0 + zz - xx - yy) * 2
    return ((zx + xz) / s, (zy + yz) / s, 0.25 * s, (xy - yx) / s)


def collider(shape, sensor=False):
    return {'shape': shape, 'density': 1000, 'friction': 0.5, 'restitution': 0, 'sensor': sensor,
            'memberships': 4294967295, 'filter': 4294967295}


KINEMATIC = {'motion': 'kinematic', 'gravity_scale': 0, 'linear_damping': 0, 'angular_damping': 0,
             'can_sleep': True, 'ccd': True}


def entity(scene, ident, name, components, parent=None):
    return {'op': 'create_entity', 'scene_id': scene, 'entity': {
        'id': ident, 'name': name, 'parent': parent, 'provenance': None, 'components': components}}


def mesh(asset, shadows=True):
    return {'mesh': ASSETS[asset][0], 'materials': [], 'cast_shadows': shadows}


def animation(seconds=0.0):
    return {'clip': 0, 'seconds': seconds, 'speed': 1.0, 'playing': True, 'looping': True}


def input_actions():
    keys = [('KeyD', [1, 0]), ('KeyA', [-1, 0]), ('KeyW', [0, -1]), ('KeyS', [0, 1]),
            ('ArrowRight', [1, 0]), ('ArrowLeft', [-1, 0]), ('ArrowUp', [0, -1]), ('ArrowDown', [0, 1])]
    return bindings.with_restart({'move': {'kind': 'axis2', 'dead_zone': 0.1, 'bindings': [
                {'type': 'key', 'code': code, 'scale': value} for code, value in keys] +
                [{'type': 'gamepad_stick', 'id': 0, 'stick': 'left', 'invert_y': False}]},
            'jump': {'kind': 'button', 'bindings': [{'type': 'key', 'code': 'Space'},
                                                    {'type': 'gamepad_button', 'id': 0, 'button': 'south'}]}})


def audio_commands(scene):
    """Music plays from the start; event cues are authored silent for the gameplay
    binding to start. Historical per-occurrence sources remain unchanged; the
    current binding explicitly restarts the first pickup/hurt source per event."""
    def source(clip, gain, bus, playing=False, looping=False, streaming=False):
        return {'AudioSource': {'clip': ASSETS[clip][0], 'gain_db': gain, 'pan': 0.0, 'rate': 1.0,
                                'start_seconds': 0.0, 'playing': playing, 'looping': looping,
                                'streaming': streaming, 'bus': AUDIO[bus], 'spatial': None}}
    cmds = [entity(scene, AUDIO['music_bus'], 'Music bus', {'AudioBus': {'gain_db': -6.0, 'parent': None}}),
            entity(scene, AUDIO['effects_bus'], 'Effects bus', {'AudioBus': {'gain_db': -2.0, 'parent': None}}),
            entity(scene, AUDIO['music'], 'Mesa loop', source('music', -3.0, 'music_bus', True, True, True))]
    for i, ident in enumerate(AUDIO['pickup']):
        cmds.append(entity(scene, ident, f'Pickup cue {i + 1}', source('cue_pickup', -4.0, 'effects_bus')))
    for i, ident in enumerate(AUDIO['hurt']):
        cmds.append(entity(scene, ident, f'Hurt cue {i + 1}', source('cue_hurt', -1.0, 'effects_bus')))
    cmds.append(entity(scene, AUDIO['won'], 'Won cue', source('cue_won', -3.0, 'effects_bus')))
    cmds.append(entity(scene, AUDIO['lost'], 'Lost cue', source('cue_lost', -3.0, 'effects_bus')))
    return cmds


def world_commands(scene):
    """The level: lights, static set, collision, navigation, actors, camera, controller."""
    colliders = level.colliders()
    cmds = [
        entity(scene, E['canyon'], 'Canyon', {'Transform': transform((0, 0, 0)), 'MeshRenderer': mesh('canyon')}),
        entity(scene, E['sky'], 'Sky dome', {'Transform': transform((0, 0, 0)),
                                             'MeshRenderer': mesh('sky', shadows=False)}),
        entity(scene, E['environment'], 'Sky light', {'Transform': transform((0, 0, 0)), 'EnvironmentLight': {
            'texture': ASSETS['environment'][0], 'intensity': 0.85, 'rotation_degrees': 0.0}}),
        entity(scene, E['sun'], 'Sun', {'Transform': transform((0, 30, 0), look_rotation(
            tuple(-c for c in sky.sun_direction()))), 'DirectionalLight': {
            'color': [1.0, 0.84, 0.66], 'intensity': 3.4, 'shadows': {'distance': 42.0}}}),
        entity(scene, E['lantern'], 'Camp lantern', {'Transform': transform(
            (level.CAMP['lamp']['at'][0], level.CAMP['lamp']['height'] + 0.02, level.CAMP['lamp']['at'][2])),
            'PointLight': {'color': [1.0, 0.7, 0.4], 'intensity': 5.0, 'range': 6.5}}),
        entity(scene, E['beacon'], 'Lift beacon', {'Transform': transform(
            (level.LIFT['at'][0], level.UPPER_Y + level.LIFT['height'] + 1.15, level.LIFT['at'][2])),
            'PointLight': {'color': [1.0, 0.88, 0.7], 'intensity': 9.0, 'range': 9.0}}),
        entity(scene, E['ground'], 'Ground collision', {'Transform': transform((0, 0, 0)), 'Collider': collider(
            {'type': 'compound', 'parts': colliders['ground']})}),
        entity(scene, E['walls'], 'Canyon wall collision', {'Transform': transform((0, 0, 0)), 'Collider': collider(
            {'type': 'compound', 'parts': colliders['walls']})}),
        entity(scene, E['rocks'], 'Obstacle collision', {'Transform': transform((0, 0, 0)), 'Collider': collider(
            {'type': 'compound', 'parts': colliders['rocks']})}),
        entity(scene, E['navigation'], 'Navigation', {'Transform': transform((0, 0, 0)), 'NavigationMesh': {
            'settings': level.navigation_settings(), 'links': [],
            'sources': [{'entity': E[k], 'geometry': 'collider'} for k in ('ground', 'walls', 'rocks')]}}),
        entity(scene, E['player'], 'Surveyor', {
            'Transform': transform(level.PLAYER_START), 'Velocity': {'linear': [0, 0, 0]},
            'RigidBody': KINEMATIC, 'Collider': collider(level.CAPSULE), 'MeshRenderer': mesh('surveyor'),
            'Animation': animation()}),
        entity(scene, E['exit'], 'Lift exit', {'Transform': transform(level.EXIT)}),
    ]
    camera_at = tuple(p + o for p, o in zip(level.PLAYER_START, level.CAMERA_OFFSET))
    pitch = math.radians(level.CAMERA_PITCH_DEGREES) / 2
    cmds.append(entity(scene, E['camera'], 'Follow camera', {
        'Transform': transform(camera_at, (math.sin(pitch), 0.0, 0.0, math.cos(pitch))),
        'Camera': {'projection': {'kind': 'perspective'}, 'fov_degrees': level.CAMERA_FOV_DEGREES,
                   'near': 0.1, 'far': 520.0},
        'AudioListener': {}}))
    for i, (ident, at) in enumerate(zip(ENEMIES, level.ENEMIES)):
        cmds.append(entity(scene, ident, f'Ember mite {i + 1}', {
            'Transform': transform(at), 'Velocity': {'linear': [0, 0, 0]}, 'RigidBody': KINEMATIC,
            'Collider': collider(level.CAPSULE), 'MeshRenderer': mesh('mite'), 'Animation': animation(0.37 * i),
            'PointLight': {'color': [1.0, 0.42, 0.12], 'intensity': 1.4, 'range': 2.6}}))
    for i, (ident, at) in enumerate(zip(SAMPLES, level.SAMPLES)):
        cmds.append(entity(scene, ident, f'Core sample {i + 1}', {
            'Transform': transform(at), 'Collider': collider({'type': 'sphere', 'radius': 0.35}, sensor=True),
            'MeshRenderer': mesh('sample', shadows=False), 'Animation': animation(0.55 * i),
            'PointLight': {'color': [0.3, 0.95, 0.9], 'intensity': 1.5, 'range': 2.8}}))
    props = {'player': E['player'], 'navigation': E['navigation'], 'exit': E['exit'], 'enemies': ENEMIES,
             'pickups': SAMPLES, 'camera': E['camera'], **SETTINGS, **bindings.roles()}
    cmds.append(entity(scene, E['controller'], 'Core Sample gameplay', {
        'Transform': transform((0, 0, 0)), 'Script': {'source': SCRIPT, 'props': props}}))
    return cmds


class Session:
    """RPC session that records every call, transaction and snapshot."""

    def __init__(self, binary, project, journal, log):
        self.Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        self.binary, self.project, self.journal, self.log = binary, project, journal, log
        self.rpc = self.Rpc(binary, project, journal)

    def call(self, method, params=None):
        result = self.rpc.call(method, params)
        self.log.append({'method': method, 'params': params if method != 'command.execute' else {
            'description': params['description'], 'commands': len(params['commands'])},
            'revision': result.get('revision') if isinstance(result, dict) else None})
        return result

    def execute(self, description, commands):
        before = self.call('project.read')
        result = self.call('command.execute', {'expected_revision': before['revision'],
                                               'description': description, 'commands': commands})
        if result['revision'] != before['revision'] + 1:
            raise RuntimeError(f'{description}: transaction was not atomic')
        return result['transaction_id']

    def close(self):
        self.rpc.close()


def run(argv, **kwargs):
    result = subprocess.run([str(a) for a in argv], text=True, capture_output=True, timeout=600, **kwargs)
    if result.returncode:
        raise RuntimeError(f'{Path(str(argv[0])).name} {argv[1]} failed: {result.stderr[-6000:]}')
    return result


def prepare_art(project_dir, art):
    target = project_dir / 'art'
    if art:
        source = Path(art).resolve(strict=True)
        manifest = json.loads((source / 'manifest.json').read_text(encoding='utf-8'))
        target.mkdir()
        for name, expected in manifest['files'].items():
            if digest(source / name) != expected:
                raise RuntimeError(f'art cache digest mismatch: {name}')
            shutil.copyfile(source / name, target / name)
        shutil.copyfile(source / 'manifest.json', target / 'manifest.json')
        return manifest
    from art import world
    return world.build(target)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--art', type=Path, help='reuse a generated art directory (manifest digests are checked)')
    parser.add_argument('--locale', choices=sorted(hud.LOCALES), default='en')
    parser.add_argument('--state', choices=sorted(hud.DOCUMENT['states']), default='playing')
    args = parser.parse_args()
    out = args.output.absolute().resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve(strict=True)
    project_dir = out / 'project'
    project_dir.mkdir()
    project, journal = project_dir / 'core.json', project_dir / 'core.journal.jsonl'
    receipt = {'format': 'incant-core-sample-presentation', 'version': 1, 'binary_sha256': digest(binary),
               'gameplay_sha256': digest(HERE / 'gameplay.ts'), 'transactions': []}
    run([binary, 'init', project, '--entities', '0', '--name', 'Core Sample'])
    manifest = prepare_art(project_dir, args.art)
    receipt['art_manifest'] = manifest
    (project_dir / 'fonts').mkdir()
    for key, source in FONT_SOURCES.items():
        shutil.copyfile(source, project_dir / ASSETS[key][1])
    total = len(level.SAMPLES)
    texts = {'en': hud.resolve(binary, out / 'l10n-en', 'en', total, run, Session)}
    if args.locale != 'en':
        texts[args.locale] = hud.resolve(binary, out / ('l10n-' + args.locale), args.locale, total, run, Session)
    chain = [(k, (project_dir / ASSETS[k][1]).read_bytes()) for k in hud.DOCUMENT['fonts']]
    receipt['fonts'] = {k: digest(project_dir / ASSETS[k][1]) for k in hud.DOCUMENT['fonts']}
    receipt['localized_text'] = texts
    receipt['glyph_coverage'] = {}
    for locale, table in texts.items():
        supplied, missing = fontcheck.coverage(hud.display_strings(table), chain)
        if missing:
            raise RuntimeError(f'{locale} HUD text has glyphs outside the font chain: {missing!r}')
        receipt['glyph_coverage'][locale] = supplied
    imports = {}
    for key, (_, path) in ASSETS.items():
        argv = [binary, 'import', project, path] + (['--texture-usage', 'color'] if path.endswith('.png') else [])
        imports[key] = json.loads(run(argv).stdout)
    receipt['imports'] = {k: {'id': v['asset']['id'], 'kind': v['asset']['kind'], 'sha256': v['asset']['sha256'],
                              'details': v.get('details')} for k, v in imports.items()}
    calls = []
    session = Session(binary, project, journal, calls)
    snapshots = []
    try:
        start = session.call('project.read')
        snapshots.append(start['project'])
        default_scene = next(iter(start['project']['scenes']))
        fonts = [ASSETS[k][0] for k in hud.DOCUMENT['fonts']]
        rekey = []
        for key, (stable, _) in ASSETS.items():
            record = dict(imports[key]['asset'])
            rekey.append({'op': 'remove_asset', 'asset_id': record['id']})
            record['id'] = stable
            rekey.append({'op': 'upsert_asset', 'asset': record})
        steps = [
            ('Assign stable Core Sample asset identities', rekey),
            ('Author Core Sample scene, script and input', [
                {'op': 'create_scene', 'scene': {'id': SCENE, 'name': 'Mesa survey', 'entities': {}}},
                {'op': 'delete_scene', 'scene_id': default_scene},
                {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'},
                 'source': (HERE / 'gameplay.ts').read_text(encoding='utf-8')},
                {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Core Sample', 'path': 'gameplay.ts'}},
                {'op': 'set_input_actions', 'actions': input_actions()},
                {'op': 'upsert_string_table', 'table': hud.TABLE},
                {'op': 'set_locale', 'settings': hud.LOCALES['en']}]),
            ('Build the mesa survey level', world_commands(SCENE)),
            ('Author music and event cues', audio_commands(SCENE)),
            ('Author the localized HUD', hud.author(SCENE, texts['en'], fonts, total)),
        ]
        if args.locale != 'en' or args.state != 'playing':
            steps.append((f'Present HUD: {args.state} in {args.locale}',
                          hud.present(SCENE, texts[args.locale], fonts, total, args.locale, args.state)))
        for description, commands in steps:
            tx = session.execute(description, commands)
            snapshots.append(session.call('project.read')['project'])
            receipt['transactions'].append({'id': tx, 'description': description, 'commands': len(commands)})
        # Undo everything back to the import state, then redo, comparing each snapshot.
        for expected in reversed(snapshots[:-1]):
            session.call('history.undo')
            if session.call('project.read')['project'] != expected:
                raise RuntimeError('Undo did not restore the recorded snapshot')
        for expected in snapshots[1:]:
            session.call('history.redo')
            if session.call('project.read')['project'] != expected:
                raise RuntimeError('Redo did not restore the recorded snapshot')
        session.call('project.save')
        final = session.call('project.read')
    finally:
        session.close()
    reopened = Session(binary, project, journal, calls)
    try:
        if reopened.call('project.read') != final:
            raise RuntimeError('Reopened project differs from the saved session')
    finally:
        reopened.close()
    validation = json.loads(run([binary, 'validate', project]).stdout)
    receipt.update(undo_redo_reopen=True, validation=validation, revision=final['revision'],
                   project_sha256=digest(project), entities=E, enemies=ENEMIES, samples=SAMPLES, script=SCRIPT,
                   scene=SCENE, assets={k: v[0] for k, v in ASSETS.items()}, settings=SETTINGS,
                   locale=args.locale, state=args.state, audio=AUDIO,
                   widgets={w['name']: hud.widget(w['index']) for w in hud.DOCUMENT['widgets']},
                   rpc_calls=len(calls))
    write_json(out / 'receipt.json', receipt)
    print(json.dumps({'output': str(out), 'project': str(project), 'revision': final['revision'],
                      'project_sha256': receipt['project_sha256']}))


if __name__ == '__main__':
    main()
