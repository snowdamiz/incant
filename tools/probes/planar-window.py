#!/usr/bin/env python3
"""Explicit Pearl Cove preparation, same-package CPU baseline and window stages.

This never builds a player. A separately reviewed ordinary shipping export must
precede baseline/window, and an exclusive native-window grant precedes window.
Numerical report parity does not establish pixels, FPS or a phase gate.
"""
import argparse
from contextlib import contextmanager, nullcontext
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import runpy
import shutil
import signal
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process
from native_package import ulid_hex

ORDER = ('keyboard-win', 'urchin-loss', 'shore-and-cliff')
TICKS = (781, 201, 221)
FLOOR = 6 * 1024**3
LOG_CAP = 64 * 1024**2


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(name):
    path = PurePosixPath(name)
    require(name and '\\' not in name and ':' not in name and not path.is_absolute()
            and '..' not in path.parts and str(path) == name, 'Relative engine-owned input path required')
    return path


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + '\n')


def fresh(path):
    path = path.absolute()
    require(not path.exists() and not path.is_symlink(), 'Choose a fresh output directory')
    path.mkdir(parents=False)
    return path


def inventory(root):
    return {p.relative_to(root).as_posix(): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def generate(output):
    """Data only: delegate unchanged route/observation generation to the sample."""
    play = runpy.run_path(str(ROOT / 'samples/planar/playtest.py'))
    hud = runpy.run_path(str(ROOT / 'samples/planar/hud.py'))
    output = fresh(output)
    rows = []
    for name, ticks in zip(ORDER, TICKS):
        route = 'win' if name == 'keyboard-win' else name
        recording = play['replay'](play['ROUTES'][route], 'wasd')
        require(recording['ticks'] == ticks, 'Existing sample route changed; review new bounds')
        observation = play['observe'](name, ticks)
        write(output / (name + '-input.json'), recording)
        write(output / (name + '-observe.json'), observation)
        rows.append({'name': name, 'ticks': ticks,
                     'recording_sha256': digest(output / (name + '-input.json')),
                     'observation_sha256': digest(output / (name + '-observe.json'))})
    source_paths = sorted((ROOT / 'samples/planar').rglob('*.py')) + [ROOT / 'samples/planar/gameplay.ts']
    manifest = {'format': 'incant-pearl-cove-recordings', 'version': 1, 'cases': rows,
                'camera': play['cove'].CAMERA, 'player': play['cove'].PLAYER,
                'pearls': play['cove'].PEARLS, 'boat': play['cove'].BOAT,
                'hud_status': hud['STATUS'], 'hud_prompt': hud['PROMPT'],
                'sample_sources': {p.relative_to(ROOT).as_posix(): digest(p) for p in source_paths},
                'route_changes': False, 'native_execution': False}
    write(output / 'recordings.json', manifest)
    return manifest


def recordings(path):
    manifest = json.loads((path / 'recordings.json').read_text())
    require(manifest['format'] == 'incant-pearl-cove-recordings'
            and type(manifest['version']) is int and manifest['version'] == 1,
            'Unknown recording manifest')
    require(manifest['route_changes'] is False and manifest['native_execution'] is False, 'Data-only unchanged route manifest')
    require([(r['name'], r['ticks']) for r in manifest['cases']] == list(zip(ORDER, TICKS))
            and all(type(r['ticks']) is int for r in manifest['cases']), 'Exact three unchanged routes')
    for row in manifest['cases']:
        for suffix, key in (('-input.json', 'recording_sha256'), ('-observe.json', 'observation_sha256')):
            require(digest(path / (row['name'] + suffix)) == row[key], 'Recording/observation identity')
    for name, expected in manifest['sample_sources'].items():
        relative(name)
        require(name.startswith('samples/planar/'), 'Unexpected sample source namespace')
        require(digest(ROOT / name) == expected, 'Authored sample source changed')
    return manifest


def package(path, manifest, selected=None):
    receipt = json.loads((path / 'export.json').read_text())
    require(receipt['format'] == 'incant-native-export' and type(receipt['version']) is int and receipt['version'] == 1
            and receipt['mode'] == 'window' and receipt['camera'] == ulid_hex(manifest['camera'])
            and receipt['startup_source_parsing'] is False and receipt['build_storage_removed_before_startup'] is True,
            'Reviewed ordinary source-free window export required')
    player = path / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    require(digest(player) == receipt['player_sha256'], 'Exported executable identity')
    for name, expected in receipt['data']['files'].items():
        relative(name)
        require(digest(path / 'player/data' / name) == expected, 'Exported data identity')
    if selected is not None:
        require(digest(selected) == receipt['player_sha256'], 'Selected bundle executable differs from export')
        player = selected
    return player, receipt


def check_cpu(row, report, manifest):
    require(type(report['tick']) is int and report['tick'] == row['ticks'], 'Exact same-package CPU ticks')
    observed = report['observations']
    require(observed['passed'] is True and observed['checks']
            and all(c['passed'] is True for c in observed['checks']), 'Actual CPU observation checks')
    player = observed['entities'][manifest['player']]
    x, y, z = player['translation']
    require(all(type(v) in (int, float) and math.isfinite(v) for v in (x, y, z)), 'Finite numeric XY pose required')
    require(z == 0, 'Player left XY plane')
    logs = report['logs']
    require(type(logs['dropped']) is int and logs['dropped'] == 0, 'Dropped gameplay logs')
    messages = [r['message'] for r in logs['entries']]
    picked = [m.split()[1] for m in messages if m.startswith('planar.pickup ')]
    won = sum(m.startswith('planar.won ') for m in messages)
    lost = sum(m.startswith('planar.lost ') for m in messages)
    if row['name'] == 'keyboard-win':
        require(picked == manifest['pearls'] and (won, lost) == (1, 0)
                and abs(x - 10.5) < .05 and 2.25 < y < 2.75, 'Actual pearl collection and boat win')
        status = 'Sailing home with 3 pearls'
    elif row['name'] == 'urchin-loss':
        require(not picked and (won, lost) == (0, 1) and abs(x - 13.5) < .05 and 4.1 < y < 4.25,
                'Actual urchin loss')
        status = 'Stung by a sea urchin'
    else:
        require(not picked and (won, lost) == (0, 0) and abs(x - 6.5) < .05 and abs(y - 9.7) < .05,
                'Actual shore/cliff collision')
        status = 'Pearls 0 of 3'
    render = report['render_input']
    require(render['mode'] == 'cpu-only' and render['gpu_submitted'] is False
            and type(render['sprite_count']) is int and render['sprite_count'] > 0,
            'Authored sprites admitted without GPU submission; tile pixels remain separate')
    ui = render['ui']
    require(ui['viewport'] == [640, 360] and ui['widgets'] == 3 and type(ui['glyphs']) is int and ui['glyphs'] > 0
            and ui['sources'][manifest['hud_status']] == status and ui['sources'][manifest['hud_prompt']] == '',
            'Actual canonical live Pearl Cove HUD')
    frames = {tuple(r['id']): r for r in report['flipbooks']}
    for owner, frame in ((manifest['player'], {'keyboard-win': 2, 'urchin-loss': 1, 'shore-and-cliff': 0}[row['name']]),
                         (manifest['boat'], int(row['name'] == 'keyboard-win'))):
        state = frames[tuple(bytes.fromhex(ulid_hex(owner)))]
        require(type(state['frame']) is int and state['frame'] == frame and state['playing'] is False,
                'Actual authored actor/boat state flipbooks')


def check_window(row, report, baseline, manifest):
    require(report['mode'] == 'native-window' and report['on_screen'] is True and report['backend'] == 'Metal',
            'Actual Mac Metal window required')
    require(type(report['host_steps']) is int and type(report['tick']) is int
            and report['host_steps'] == report['tick'] == row['ticks'], 'Complete accepted host steps')
    recording = report['input_recording']
    require(recording['sha256'] == row['recording_sha256'] and recording['completed'] is True
            and recording['requested_host_steps'] == recording['accepted_host_steps'] == row['ticks']
            and recording['physical_devices'] is False and recording['native_focus_and_surface_required'] is True,
            'Recorded physical events through real native eligibility')
    require(report['observations'] == baseline['observations'] and report['observations']['passed'] is True,
            'Exact same-package CPU/window entity and physics observations')
    require(type(report['accepted_frames']) is int and report['accepted_frames'] >= row['ticks']
            and report['ui']['widgets'] == 3 and type(report['ui']['glyph_quads']) is int and report['ui']['glyph_quads'] > 0,
            'Native scene/HUD submissions; pixel appearance remains separate')


@contextmanager
def bounded(seconds, output):
    """Latch signals and cancel only inside managed ownership/safe checkpoints."""
    require(threading.current_thread() is threading.main_thread() and hasattr(signal, 'setitimer'), 'POSIX main thread required')
    require(signal.getitimer(signal.ITIMER_REAL) == (0., 0.), 'Another alarm is active')
    numbers = (signal.SIGALRM, signal.SIGTERM, signal.SIGINT)
    previous = {n: signal.getsignal(n) for n in numbers}
    deadline, cause = time.monotonic() + seconds, None
    def latch(number, *_):
        nonlocal cause
        if cause is None:
            cause = TimeoutError('Pearl Cove execution deadline') if number == signal.SIGALRM else InterruptedError('Pearl Cove cancelled')
            signal.setitimer(signal.ITIMER_REAL, 0)
    def check():
        if cause is None and time.monotonic() >= deadline:
            latch(signal.SIGALRM)
        if cause is not None:
            raise cause
        require(shutil.disk_usage(output).free >= FLOOR, 'Six GiB free floor reached')
        require(sum(p.stat().st_size for p in output.glob('*.raw.log')) <= LOG_CAP, 'Bounded raw output exceeded')
    try:
        for n in numbers:
            signal.signal(n, latch)
        signal.setitimer(signal.ITIMER_REAL, seconds)
        yield check
        check()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for n, handler in previous.items():
            signal.signal(n, handler)


@contextmanager
def readiness(argv, output):
    """Observe exact fresh-fixture argv; never opens, raises or relaunches an app."""
    stop, records, errors = threading.Event(), [], []
    expected = ' '.join(argv)
    def observe():
        try:
            until = time.monotonic() + 20
            while not stop.is_set() and time.monotonic() < until:
                result = subprocess.run(['ps', '-axo', 'pid=,command='], capture_output=True, text=True, timeout=2)
                require(result.returncode == 0, 'Owned-child process inventory failed')
                matches = []
                for line in result.stdout.splitlines():
                    pid, _, command = line.strip().partition(' ')
                    if command.lstrip() == expected:
                        matches.append(int(pid))
                require(len(matches) <= 1, 'Ambiguous exact app argv; no activation permitted')
                if matches:
                    row = {'stage': 'owned_child_live', 'pid': matches[0], 'argv': argv,
                           'created_utc': datetime.now(timezone.utc).isoformat()}
                    records.append(row); write(output / 'owned-live.json', row)
                    print(json.dumps(row), flush=True)
                    return
                stop.wait(.05)
        except BaseException as error:
            errors.append(repr(error))
    thread = threading.Thread(target=observe, name='pearl-cove-owned-readiness')
    thread.start()
    try:
        yield records
    finally:
        stop.set(); thread.join()
        write(output / 'readiness.json', {'records': records, 'errors': errors, 'monitor_joined': True})


def run_stage(argv, output, name, timeout, check, ledger, env=None, cwd=ROOT, observe_live=False):
    row = {'argv': list(map(str, argv)), 'timeout_seconds': timeout,
           'started_utc': datetime.now(timezone.utc).isoformat(), 'reaped': False}
    ledger.append(row)
    out, err = output / (name + '.stdout.raw.log'), output / (name + '.stderr.raw.log')
    try:
        with (readiness(row['argv'], output) if observe_live else nullcontext([])) as live:
            with out.open('xb') as stdout, err.open('xb') as stderr:
                result = native_process.run(row['argv'], cwd=cwd, env=env, timeout=timeout,
                                            stdout=stdout, stderr=stderr, cancel=check)
        row.update(exit_code=result.returncode, reaped=True)
        require(result.returncode == 0, 'Child failed; see retained raw output')
        if observe_live:
            require(len(live) == 1 and not json.loads((output / 'readiness.json').read_text())['errors'],
                    'Exact owned live PID was not observed; no window activation claim')
        check()
        return out
    except BaseException as error:
        row['error'] = repr(error)
        raise
    finally:
        row.update(finished_utc=datetime.now(timezone.utc).isoformat(),
                   logs=[{'path': p.name, 'bytes': p.stat().st_size, 'sha256': digest(p)} for p in (out, err) if p.exists()])
        write(output / 'processes.json', ledger)


def execute(args):
    for field in ('package', 'recordings', 'baseline', 'player'):
        value = getattr(args, field, None)
        if value is not None:
            setattr(args, field, value.resolve(strict=True))
    output = fresh(args.output)
    ledger, result = [], {'passed': False, 'stage': args.stage, 'pixel_capture': False,
                          'appearance_review': False, 'performance_or_phase_gate': False}
    try:
        with bounded(600 if args.stage == 'author' else 240, output) as check:
            if args.stage == 'author':
                run_stage([sys.executable, '-B', ROOT / 'samples/planar/scene.py', output / 'authoring',
                           '--binary', args.headless.resolve(strict=True)], output, 'author', 300, check, ledger)
            else:
                manifest = recordings(args.recordings)
                recording_files = inventory(args.recordings)
                player, exported = package(args.package, manifest, getattr(args, 'player', None))
                before = inventory(args.package / 'player')
                if args.stage == 'window':
                    require(sys.platform == 'darwin', 'This proof is Mac-only')
                    baseline = json.loads((args.baseline / 'result.json').read_text())
                    require(baseline['passed'] is True and baseline['stage'] == 'baseline'
                            and baseline['package_files'] == before
                            and baseline['recordings_sha256'] == digest(args.recordings / 'recordings.json'),
                            'Exact same-package CPU baseline required')
                env = {k: v for k, v in os.environ.items() if k in ('HOME', 'USER', 'LOGNAME', 'LANG', 'LC_ALL')}
                env.update(PATH=str(player.parent), TMPDIR=str(output), RAYON_NUM_THREADS='2')
                for row in manifest['cases']:
                    if args.stage == 'window' and row['name'] != args.case:
                        continue
                    name = row['name']
                    argv = [player, '--package', args.package / 'player/data']
                    argv += ['--window', '--stop-after', str(row['ticks'])] if args.stage == 'window' else [
                        '--ticks', str(row['ticks']), '--ui-viewport', '640x360', '--output', output / (name + '.pcm')]
                    if args.stage == 'window' and args.window_state_log:
                        argv += ['--window-state-log', output / 'eligibility.jsonl']
                    argv += ['--input-recording', args.recordings / (name + '-input.json'),
                             '--observe', args.recordings / (name + '-observe.json')]
                    raw = run_stage(argv, output, name, 90, check, ledger, env, player.parent, args.stage == 'window')
                    report = json.loads(raw.read_text())
                    if args.stage == 'baseline':
                        check_cpu(row, report, manifest)
                        require(type(report['frames']) is int and report['frames'] == row['ticks'] * 800
                                and (output / (name + '.pcm')).stat().st_size == row['ticks'] * 800 * 2 * 4,
                                'Exact bounded48kHz stereo float PCM clock')
                        require(digest(output / (name + '.pcm')) == report['pcm_sha256'], 'Retained actual PCM identity')
                    else:
                        reference = json.loads((args.baseline / (name + '.json')).read_text())
                        check_cpu(row, reference, manifest)
                        check_window(row, report, reference, manifest)
                    write(output / (name + '.json'), report)
                require(inventory(args.package / 'player') == before, 'Runtime changed package inputs')
                require(inventory(args.recordings) == recording_files, 'Runtime changed recording/observation inputs')
                result.update(package_files=before, player_sha256=exported['player_sha256'],
                              recordings_sha256=digest(args.recordings / 'recordings.json'))
                if args.stage == 'window':
                    result['case'] = args.case
            result.update(passed=True, owned_children_reaped=all(r['reaped'] for r in ledger))
    except BaseException as error:
        result['error'] = repr(error)
        raise
    finally:
        if args.stage == 'window' and args.window_state_log:
            log = output / 'eligibility.jsonl'
            result['window_state_log'] = {'requested': True, 'present': log.is_file()}
            if log.is_file():
                result['window_state_log'].update(path=log.name, bytes=log.stat().st_size, sha256=digest(log))
        write(output / 'result.json', result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest='stage', required=True)
    p = modes.add_parser('recordings'); p.add_argument('output', type=Path)
    p = modes.add_parser('author'); p.add_argument('output', type=Path); p.add_argument('--headless', type=Path, required=True)
    for mode in ('baseline', 'window'):
        p = modes.add_parser(mode); p.add_argument('package', type=Path); p.add_argument('recordings', type=Path)
        p.add_argument('output', type=Path)
        if mode == 'window':
            p.add_argument('--window-state-log', action='store_true', help='Explicit bounded eligibility log in this fresh output; diagnostic overhead, not a timing run')
            p.add_argument('--case', choices=ORDER, required=True, help='One explicitly coordinated route; never launches the next app automatically')
            p.add_argument('--baseline', type=Path, required=True)
            p.add_argument('--player', type=Path, help='Optional unique bundle copy; must hash-match the actual export')
    args = parser.parse_args()
    generate(args.output) if args.stage == 'recordings' else execute(args)


if __name__ == '__main__':
    main()
