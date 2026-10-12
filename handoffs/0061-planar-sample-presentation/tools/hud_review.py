#!/usr/bin/env python3
"""Reproduce handoff 0061's HUD-revision evidence from a clean build.

    python3 -B hud_review.py <new-output> [--quick]

Every child process is preceded by lease.check(). Steps:
  build/        samples/planar/scene.py --play (authoring, Undo/Redo/reopen, scenarios)
  <locale>/     en, es, fr, de and pseudo. Non-en variants are scratch copies
                changed only by one public `set_locale` transaction. Each gets
                the hud_frames example built for it, its cooked data packaged by
                tools/native_package.data_package, and offscreen GPU captures of
                real replays with the HUD text and flipbook frames asserted.
  player/       en only: the ordinary project player in CPU mode with
                --ui-viewport; asserts the UI host's shaped status text.
  review.json   hashes, labels, assertions and every capture.
--quick renders only the en keyboard win (for layout iteration).
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
PLANAR = WORKTREE / 'samples/planar'
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(PLANAR))
sys.path.insert(0, str(WORKTREE / 'tools'))
import lease  # noqa: E402
import pngtool  # noqa: E402
import hud  # noqa: E402
from art import cove  # noqa: E402
from native_package import data_package, select_camera  # noqa: E402

HEADLESS = lease.TARGET / 'ci/incant_headless'
EXAMPLE = lease.TARGET / 'ci/examples/hud_frames'
PLAYER = lease.TARGET / 'ci/incant_game_player'
LOCALES = {'en': {'locale': 'en'}, 'es': {'locale': 'es'}, 'fr': {'locale': 'fr'},
           'de': {'locale': 'de'}, 'pseudo': {'locale': 'en', 'pseudo': True}}
# (recording, viewport, scale, {tick: expected state}) per run.
WIN = {2: 'start', 150: 'one', 450: 'two', 560: 'ready', 781: 'won'}
STATES = {'start': ('hud.collecting', 0, 0, 0), 'one': ('hud.collecting', 1, 0, 0),
          'two': ('hud.collecting', 2, 0, 0), 'ready': ('hud.ready', 3, 0, 1),
          'won': ('hud.won', 3, 2, 1), 'lost': ('hud.lost', 0, 1, 0),
          'locked': ('hud.collecting', 0, 0, 0)}
RUNS = {
    'en': [('keyboard-win', '640x360', 2, WIN), ('gamepad-win', '640x360', 2, {781: 'won'}),
           ('urchin-loss', '640x360', 2, {201: 'lost'}), ('locked-boat', '640x360', 2, {181: 'locked'}),
           ('keyboard-win', '640x480', 2, {2: 'start', 781: 'won'}),
           ('keyboard-win', '844x390', 3, {2: 'start', 781: 'won'})],
    **{name: [('keyboard-win', '640x360', 2, {2: 'start', 560: 'ready', 781: 'won'}),
              ('urchin-loss', '640x360', 2, {201: 'lost'})] for name in ('es', 'fr', 'de', 'pseudo')},
}
PSEUDO_PLAIN = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
PSEUDO_ACCENTED = 'áƀçďéƒğĥíĵķľɱñóþɋŕšţúṽŵẋýžÁɃÇĎÉƑĞĤÍĴĶĽṀÑÓÞɊŔŠŢÚṼŴẊÝŽ'


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(argv, log=None, **kwargs):
    lease.check()
    argv = [str(a) for a in argv]
    result = subprocess.run(argv, cwd=WORKTREE, capture_output=True, text=True, timeout=1800, **kwargs)
    lease.record({'event': 'review-child', 'argv': argv, 'exit': result.returncode})
    if log:
        Path(log).write_text(json.dumps({'argv': argv, 'exit': result.returncode,
                                         'stdout': result.stdout[-20000:],
                                         'stderr': result.stderr[-20000:]}, indent=2) + '\n')
    if result.returncode:
        raise SystemExit(f'{argv[0]} failed ({result.returncode}): {result.stderr[-3000:]}')
    return result.stdout


def pseudo(text):
    out = []
    for char in text:
        index = PSEUDO_PLAIN.find(char)
        if index < 0:
            out.append(char)
            continue
        out.append(PSEUDO_ACCENTED[index] * (2 if char in 'aeiouAEIOU' else 1))
    return ''.join(out)


def expected(key, collected, settings):
    """The exact patterns in hud.MESSAGES: plain arguments and one plural."""
    locale = settings['locale']
    pattern = hud.MESSAGES[key][locale]
    total = len(cove.PEARLS)
    head = '{total, plural, one {'
    if pattern.startswith(head):
        one, other = pattern[len(head):-2].split('} other {')
        parts = (one if total == 1 else other).split('#')
        text = (pseudo(parts[0]) if settings.get('pseudo') else parts[0]) + str(total) + \
            (pseudo(parts[1]) if settings.get('pseudo') else parts[1])
    else:
        pieces, text = pattern.replace('{total}', '\0T\0').replace('{collected}', '\0C\0').split('\0'), ''
        for piece in pieces:
            value = {'T': str(total), 'C': str(collected)}.get(piece)
            text += value if value is not None else (pseudo(piece) if settings.get('pseudo') else piece)
    return f'[!! {text} !!]' if settings.get('pseudo') else text


def variant(base, out, name, settings):
    """Scratch copy changed by one ordinary set_locale transaction."""
    copy = out / 'project'
    shutil.copytree(base, copy)
    if name == 'en':
        return copy / 'planar.json'
    Rpc = runpy.run_path(str(WORKTREE / 'tools/probes/script-source-transactions.py'))['Rpc']
    lease.check()
    rpc = Rpc(HEADLESS, copy / 'planar.json', copy / 'planar.journal.jsonl')
    try:
        state = rpc.call('project.read')
        rpc.call('command.execute', {'expected_revision': state['revision'],
                                     'description': f'Review locale: {name}',
                                     'commands': [{'op': 'set_locale', 'settings': settings}]})
        rpc.call('project.save')
    finally:
        rpc.close()
    return copy / 'planar.json'


def package(project, out, features, binary):
    # lease.cargo passes this process environment to cargo; the build script
    # cooks and embeds exactly this project.
    os.environ['INCANT_NATIVE_PROJECT'] = str(project)
    try:
        target = ['--example', 'hud_frames'] if binary == EXAMPLE else ['--bin', 'incant_game_player']
        code = lease.cargo(['build', '-p', 'incant_game_player', *target, '--features', features])
    finally:
        del os.environ['INCANT_NATIVE_PROJECT']
    if code:
        raise SystemExit(f'cargo build failed ({code})')
    sha = digest(project)
    producers = sorted((p.stat().st_mtime, p.parent) for p in
                       (lease.TARGET / 'ci/build').glob('incant_game_player-*/out/project-export.json')
                       if json.loads(p.read_text()).get('project_sha256') == sha)
    if not producers:
        raise SystemExit('no cooked output matches the project')
    producer = producers[-1][1]
    receipt = json.loads((producer / 'project-export.json').read_text())
    out.mkdir()
    camera = select_camera(receipt, cove.CAMERA, True)
    data = data_package(project.parent.resolve(), producer.resolve(), receipt, out / 'data', camera)
    shutil.copy2(binary, out / binary.name)
    return {'producer': str(producer), 'camera_hex': camera, 'data': data,
            'binary': binary.name, 'binary_sha256': digest(out / binary.name)}


def frames(pkg, recording, viewport, scale, plan, settings, out):
    ticks = ','.join(map(str, sorted(plan)))
    run([pkg / 'hud_frames', pkg / 'data', recording, viewport, scale, ticks, out],
        log=out.with_suffix('.log.json'))
    report = json.loads((out / 'report.json').read_text())
    checks = []
    for capture in report['captures']:
        key, collected, player, boat = STATES[plan[capture['tick']]]
        labels = {k: v['text'] for k, v in capture['hud']['labels'].items()}
        want = expected(key, collected, settings)
        frames_by_id = {f['id']: f['frame'] for f in capture['flipbooks']}
        ok = (labels[hud_hex(hud.STATUS)] == want and labels[hud_hex(hud.PROMPT)] == ''
              and frames_by_id[hud_hex(cove.PLAYER)] == player and frames_by_id[hud_hex(cove.BOAT)] == boat
              and capture['hud']['missing_glyphs'] == 0)
        if not ok:
            raise SystemExit(f'{out.name} tick {capture["tick"]}: {labels} {frames_by_id} want {want!r}')
        width, height = capture['size']
        raw = (out / capture['rgba']).read_bytes()
        rows = [[tuple(raw[(y * width + x) * 4:(y * width + x) * 4 + 4]) for x in range(width)]
                for y in range(height)]
        png = out / capture['rgba'].replace('.rgba', '.png')
        pngtool.write(png, rows)
        (out / capture['rgba']).unlink()  # the PNG and the recorded RGBA hash remain
        checks.append({'tick': capture['tick'], 'state': plan[capture['tick']], 'status': want,
                       'player_frame': player, 'boat_frame': boat, 'png': png.name,
                       'png_sha256': digest(png), 'rgba_sha256': capture['rgba_sha256'],
                       'glyph_quads': capture['hud']['glyph_quads']})
    return {'adapter': report['adapter'], 'backend': report['backend'], 'size': report['size'],
            'logs': [entry['message'] for entry in report['logs']], 'checks': checks}


def hud_hex(ulid):
    value = 0
    for char in ulid:
        value = value * 32 + cove.CROCKFORD.index(char)
    return f'{value:032x}'


def player_cpu(build, out):
    """The ordinary project player hosting the HUD on its CPU UI host."""
    pkg = package(build / 'project/planar.json', out, 'project,native-window', PLAYER)
    results = {}
    for name, ticks, state in (('keyboard-win', 781, 'won'), ('gamepad-win', 781, 'won'),
                               ('urchin-loss', 201, 'lost'), ('locked-boat', 181, 'locked')):
        pcm = out / f'{name}.pcm'
        report = json.loads(run([out / PLAYER.name, '--package', out / 'data', '--ui-viewport', '640x360',
                                 '--ticks', ticks, '--input-recording', build / f'{name}-input.json',
                                 '--output', pcm], log=out / f'{name}.log.json'))
        key, collected, _, _ = STATES[state]
        status = report['render_input']['ui']['sources'][hud.STATUS]
        if status != expected(key, collected, LOCALES['en']):
            raise SystemExit(f'player {name}: status {status!r}')
        pcm.unlink()
        results[name] = {'tick': report['tick'], 'status': status,
                         'ui': {k: report['render_input']['ui'][k] for k in ('widgets', 'focus')}}
    return {'package': pkg, 'runs': results}


def main():
    out = Path(sys.argv[1]).resolve()
    quick = '--quick' in sys.argv[2:]
    out.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    environment['PATH'] = str(Path(shutil.which('node')).parent) + os.pathsep + environment['PATH']
    build = out / 'build'
    run([sys.executable, '-B', PLANAR / 'scene.py', build, '--binary', HEADLESS] + ([] if quick else ['--play']),
        log=out / 'scene.log.json', env=environment)
    if quick:
        recording = out / 'keyboard-win-input.json'
        play = runpy.run_path(str(PLANAR / 'playtest.py'))
        recording.write_text(json.dumps(play['replay'](play['WIN'], 'wasd')) + '\n')
    review = {'format': 'incant-0061-hud-review', 'head': lease.git('rev-parse', 'HEAD'),
              'headless_sha256': digest(HEADLESS), 'locales': {}}
    for name, settings in LOCALES.items():
        if quick and name != 'en':
            continue
        folder = out / name
        folder.mkdir()
        project = variant(build / 'project', folder, name, settings)
        pkg = package(project, folder / 'package', 'project,render-gpu', EXAMPLE)
        captures = {}
        for recording, viewport, scale, plan in RUNS[name][:1] if quick else RUNS[name]:
            source = out / f'{recording}-input.json' if quick else build / f'{recording}-input.json'
            label = f'{recording}-{viewport}x{scale}'
            captures[label] = frames(folder / 'package', source, viewport, scale, plan, settings, folder / label)
        review['locales'][name] = {'settings': settings, 'project_sha256': digest(project),
                                   'package': pkg, 'captures': captures}
    if not quick:
        en = review['locales']['en']['captures']
        keyboard = next(c for c in en['keyboard-win-640x360x2']['checks'] if c['tick'] == 781)
        gamepad = en['gamepad-win-640x360x2']['checks'][0]
        if keyboard['rgba_sha256'] != gamepad['rgba_sha256']:
            raise SystemExit('gamepad and keyboard wins rendered different final frames')
        review['gamepad_matches_keyboard_frame'] = True
        review['player'] = player_cpu(build, out / 'player')
        review['scene_result'] = json.loads((build / 'result.json').read_text())
    (out / 'review.json').write_text(json.dumps(review, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps({'output': str(out), 'locales': list(review['locales'])}))


if __name__ == '__main__':
    main()
