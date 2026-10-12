#!/usr/bin/env python3
"""Reproduce handoff 0061's offscreen evidence from a clean build.

    python3 -B review.py <new-output>

Every child process is preceded by guard.check() (lease valid, not released).
  build/     samples/planar/scene.py --play (authoring, Undo/Redo/reopen, scenarios)
  package/   ci-profile project player and its cooked data (tools/package.py)
  renders/   incant_headless screenshot: authoring renderer, several output sizes
  native/    native_frames: retained native runtime frames with flipbook controls
  previews/  stills of observed gameplay states, reconstructed on scratch copies
             through public RPC commands (positions/pickups from native-play)
  review.json  hashes, parity results and labels for every output
No window, screen capture, device or performance measurement is involved.
"""
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
WORKTREE = HERE.parents[2]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(WORKTREE / 'samples/planar'))
import guard  # noqa: E402
import pngtool  # noqa: E402
from art import cove  # noqa: E402

HEADLESS = guard.TARGET / 'ci/incant_headless'
NATIVE_FRAMES = guard.TARGET / 'ci/examples/native_frames'
# incant_headless screenshot is capped at 1920x1080, so phone shapes are
# aspect-matched (2532:1170 -> 1920x887); device scales are computed separately.
SIZES = {
    'logical-320x180': (320, 180), 'desktop-16x9-1280x720': (1280, 720),
    'desktop-16x9-1920x1080': (1920, 1080), 'window-4x3-1280x960': (1280, 960),
    'phone-landscape-aspect-1920x887': (1920, 887),
}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(argv, log=None, **kwargs):
    guard.check()
    argv = [str(a) for a in argv]
    result = subprocess.run(argv, cwd=WORKTREE, capture_output=True, text=True, timeout=1800, **kwargs)
    guard.record({'event': 'review-child', 'argv': argv, 'exit': result.returncode})
    if log:
        Path(log).write_text(json.dumps({'argv': argv, 'exit': result.returncode,
                                         'stdout': result.stdout[-20000:],
                                         'stderr': result.stderr[-20000:]}, indent=2) + '\n')
    if result.returncode:
        raise SystemExit(f'{argv[0]} failed ({result.returncode}): {result.stderr[-3000:]}')
    return result.stdout


def screenshot(project, path, size):
    out = run([HEADLESS, 'screenshot', project, path, '--camera', cove.CAMERA,
               '--width', size[0], '--height', size[1]])
    return json.loads(out)


def native(package, name, request, out):
    data = package / 'player/data'
    (out / f'{name}-request.json').write_text(json.dumps(request, indent=2) + '\n')
    camera = json.loads((package / 'package.json').read_text())['camera_hex']
    run([NATIVE_FRAMES, data / 'world.scene', data / 'models', data / 'textures', camera,
         out / f'{name}-request.json', out / name], log=out / f'{name}.log.json')
    return json.loads((out / name / 'report.json').read_text())


def hex_id(ulid):
    value = 0
    for char in ulid:
        value = value * 32 + cove.CROCKFORD.index(char)
    return f'{value:032x}'


def observe(build, out, name, ticks, present):
    """Truncated keyboard-win run: the real controller's state at `ticks`. The
    plan asserts which pearls remain, so native-play fails if the route drifts."""
    plan = {'format': 'incant-native-observation', 'version': 1, 'entities': [cove.PLAYER],
            'checks': [{'name': 'pearl-' + p, 'tick': ticks,
                        'expect': {'type': 'exists', 'entity': p, 'value': p in present}}
                       for p in cove.PEARLS]}
    path = out / f'{name}-observe.json'
    path.write_text(json.dumps(plan, indent=2) + '\n')
    report = json.loads(run([HEADLESS, 'native-play', build / 'project/planar.json', '--camera', cove.CAMERA,
                             '--ticks', ticks, '--input-replay', build / 'keyboard-win-input.json',
                             '--observe', path]))
    (out / f'{name}-observation.json').write_text(json.dumps(report, indent=2) + '\n')
    return report['observations']['entities'][cove.PLAYER]['translation'], present, report


def preview(build, out, name, translation, present, frames):
    """Copy the project, move the player, remove collected pearls and select
    hook frames with ordinary RPC transactions, then render one still."""
    copy = out / name
    shutil.copytree(build / 'project', copy)
    project = copy / 'planar.json'
    Rpc = runpy.run_path(str(WORKTREE / 'tools/probes/script-source-transactions.py'))['Rpc']
    guard.check()
    rpc = Rpc(HEADLESS, project, copy / 'planar.journal.jsonl')
    try:
        state = rpc.call('project.read')
        scene = next(iter(state['project']['scenes']))
        entities = state['project']['scenes'][scene]['entities']
        player = dict(entities[cove.PLAYER]['components']['Transform'])
        player['translation'] = translation
        commands = [{'op': 'set_component', 'scene_id': scene, 'entity_id': cove.PLAYER,
                     'component': 'Transform', 'value': player}]
        commands += [{'op': 'delete_entity', 'scene_id': scene, 'entity_id': p}
                     for p in cove.PEARLS if p not in present]
        for entity, frame in frames.items():
            # A flipbook's first frame overrides the sprite region, so the still
            # drops the flipbook on this scratch copy and selects the frame.
            sprite = dict(entities[entity]['components']['SpriteRenderer'])
            sprite['region'] = {'frame': cove.frame_id(frame)}
            commands += [{'op': 'remove_component', 'scene_id': scene, 'entity_id': entity,
                          'component': 'SpriteFlipbook'},
                         {'op': 'set_component', 'scene_id': scene, 'entity_id': entity,
                          'component': 'SpriteRenderer', 'value': sprite}]
        rpc.call('command.execute', {'expected_revision': state['revision'],
                                     'description': f'Preview state: {name}', 'commands': commands})
        rpc.call('project.save')
    finally:
        rpc.close()
    path = out / f'{name}-1280x720.png'
    screenshot(project, path, (1280, 720))
    return {'png': path.name, 'translation': translation, 'pearls_present': present, 'frames': frames,
            'label': 'authoring-renderer still of an observed state; not a live frame'}


def main():
    out = Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    node = shutil.which('node')
    environment['PATH'] = str(Path(node).parent) + os.pathsep + environment['PATH']
    build, package = out / 'build', out / 'package'
    run([sys.executable, '-B', WORKTREE / 'samples/planar/scene.py', build, '--binary', HEADLESS, '--play'],
        log=out / 'scene.log.json', env=environment)
    project = build / 'project/planar.json'
    run([sys.executable, '-B', HERE / 'package.py', project, package, '--camera', cove.CAMERA],
        log=out / 'package.log.json', env=environment)
    review = {'format': 'incant-0061-review', 'head': guard.git('rev-parse', 'HEAD'),
              'headless_sha256': digest(HEADLESS), 'native_frames_sha256': digest(NATIVE_FRAMES),
              'scene_result': json.loads((build / 'result.json').read_text()),
              'package': json.loads((package / 'package.json').read_text())['data']}
    renders = out / 'renders'
    renders.mkdir()
    review['renders'] = {}
    for name, size in SIZES.items():
        info = screenshot(project, renders / f'{name}.png', size)
        review['renders'][name] = {'size': size, 'adapter': info['adapter'], 'geometry': info['geometry'],
                                   'png_sha256': digest(renders / f'{name}.png')}
    natives = out / 'native'
    natives.mkdir()
    pearls, urchins = [hex_id(p) for p in cove.PEARLS], [hex_id(u) for u in cove.URCHINS]
    review['native'] = {
        'motion': native(package, 'motion', {'width': 960, 'height': 540,
                                             'ticks': list(range(0, 64, 2))}, natives),
        'hooks': native(package, 'hooks', {'width': 960, 'height': 540, 'ticks': [0, 1, 2], 'controls': [
            {'tick': 1, 'entity': hex_id(cove.PLAYER), 'action': {'kind': 'seek', 'frame': 1}},
            {'tick': 2, 'entity': hex_id(cove.PLAYER), 'action': {'kind': 'seek', 'frame': 2}},
            {'tick': 2, 'entity': hex_id(cove.BOAT), 'action': {'kind': 'seek', 'frame': 1}}]}, natives),
    }
    review['native']['ids'] = {'pearls': pearls, 'urchins': urchins}
    screenshot(project, natives / 'authoring-960x540.png', (960, 540))
    a = pngtool.read(natives / 'authoring-960x540.png')
    b = pngtool.read(natives / 'motion/tick-00000.png')
    differing = sum(p != q for ra, rb in zip(a, b) for p, q in zip(ra, rb))
    review['parity'] = {'authoring': 'native/authoring-960x540.png', 'native': 'native/motion/tick-00000.png',
                        'differing_pixels': differing, 'pixels': len(a) * len(a[0])}
    previews = out / 'previews'
    previews.mkdir()
    # Tick 450 is just after the east-alcove pearl (collected near tick 428).
    at, present, _ = observe(build, previews, 'collecting', 450, [cove.PEARLS[2]])
    review['previews'] = {'collecting': preview(build, previews, 'collecting', at, present, {})}
    win = review['scene_result']['scenarios']['keyboard-win']['final_translation'] + [0.0]
    loss = review['scene_result']['scenarios']['urchin-loss']['final_translation'] + [0.0]
    review['previews']['won-hooks'] = preview(build, previews, 'won-hooks', win, [], {
        cove.PLAYER: 'beachcomber/2', cove.BOAT: 'boat/1'})
    review['previews']['lost-hook'] = preview(build, previews, 'lost-hook', loss, list(cove.PEARLS), {
        cove.PLAYER: 'beachcomber/1'})
    review['files'] = {str(p.relative_to(out)): digest(p) for p in sorted(out.rglob('*'))
                       if p.is_file() and '.incant' not in p.parts and p.suffix in ('.png', '.json')}
    (out / 'review.json').write_text(json.dumps(review, indent=2) + '\n')
    print(json.dumps({'output': str(out), 'parity_differing_pixels': differing}))


if __name__ == '__main__':
    main()
