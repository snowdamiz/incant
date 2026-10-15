#!/usr/bin/env python3
"""Explicit Core locale authoring, same-package CPU and native-window stages.

This does not build a player or grant a window run. Each locale needs its own
reviewed ordinary export: embedded script bindings include the whole scene hash.
Report parity/PCM does not establish pixels, audible quality, FPS or a phase gate.
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
import struct
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process
from native_package import ulid_hex

LOCALES = {'ar': {'fallbacks': [], 'locale': 'ar', 'pseudo': False},
           'zh-Hans': {'fallbacks': [], 'locale': 'zh-Hans', 'pseudo': False},
           'en-XA': {'fallbacks': [], 'locale': 'en', 'pseudo': True}}
ORDER = ('lose-short', 'win', 'lose-restart-win')
TICKS = (900, 2156, 3056)
CAMERA = '01CSEN00000000000000000003'
TICK_BYTES = 6400  # 48 kHz stereo f32 at 60 Hz.
FLOOR = 6 * 1024**3


def require(value, message):
    if not value:
        raise ValueError(message)


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


def locale_settings(value, locale):
    return (isinstance(value, dict) and value == LOCALES[locale]
            and type(value.get('pseudo')) is bool)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    def invalid(value):
        raise ValueError('Nonfinite JSON: ' + value)
    return json.loads(Path(path).read_text(), parse_constant=invalid)


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + '\n')


def fresh(path):
    path = path.absolute()
    require(not path.exists() and not path.is_symlink(), 'Choose a fresh output directory')
    path.mkdir(parents=False)
    return path


def relative(name):
    path = PurePosixPath(name)
    require(name and '\\' not in name and ':' not in name and not path.is_absolute()
            and '..' not in path.parts and str(path) == name, 'Relative engine-owned path required')


def inventory(root):
    return {p.relative_to(root).as_posix(): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def authoring(path, locale):
    require(locale in LOCALES, 'Unknown qualification locale')
    receipt = read(path / 'receipt.json')
    project = path / 'project/core.json'
    require(receipt['format'] == 'incant-core-sample-presentation'
            and type(receipt['version']) is int and receipt['version'] == 1
            and receipt['locale'] == locale and receipt['state'] == 'playing'
            and receipt['undo_redo_reopen'] is True and receipt['validation']['valid'] is True,
            'Completed public playing-locale authoring required')
    require(digest(project) == receipt['project_sha256']
            and digest(path / 'project/gameplay.ts') == receipt['gameplay_sha256'] == digest(ROOT / 'samples/core/gameplay.ts'),
            'Authoring project/current gameplay identity')
    require(locale_settings(read(project)['settings']['localization'], locale), 'Exact locale/pseudo project settings')
    require(receipt['entities']['camera'] == CAMERA and len(receipt['samples']) == 4
            and len(receipt['widgets']) == 13 and len(receipt['localized_text'][locale]) > 0,
            'Existing complete Core scene/HUD required')
    return receipt


def generate(author, locale, output):
    """Data only; unchanged sample helpers supply routes, plans and localized labels.

    The sample has absolute art/hud imports, so call this CLI in a fresh Python
    interpreter instead of importing unrelated samples into a shared test process.
    """
    receipt = authoring(author, locale)
    sys.path.insert(0, str(ROOT / 'samples/core'))
    oracle = runpy.run_path(str(ROOT / 'samples/core/binding_oracle.py'))
    recordings, _ = oracle['restart_cases']()
    lose, win = recordings['lose-short'], recordings['win']
    values = [lose, win, oracle['restarted'](lose, win)]
    plans = [oracle['plan'](receipt, 'lose', lose), oracle['plan'](receipt, 'win', win),
             oracle['restart_plan'](receipt, 'lose-short', 'win', lose, win)]
    output = fresh(output)
    rows = []
    for name, ticks, recording, plan in zip(ORDER, TICKS, values, plans):
        require(recording['ticks'] == ticks, 'Existing route changed; review bounds')
        write(output / (name + '-input.json'), recording)
        write(output / (name + '-observe.json'), plan)
        health, count, phase = (0, 0, 'lost') if name == 'lose-short' else (2, 4, 'won')
        rows.append({'name': name, 'host_steps': ticks, 'simulation_tick': 2156 if name == ORDER[2] else ticks,
                     'recording_sha256': digest(output / (name + '-input.json')),
                     'observation_sha256': digest(output / (name + '-observe.json')),
                     'labels': oracle['labels'](receipt['localized_text'][locale], health, count, phase, receipt['hud'])})
    sources = [p for p in sorted((ROOT / 'samples/core').rglob('*'))
               if p.is_file() and p.suffix in ('.py', '.ts', '.json')]
    manifest = {'format': 'incant-core-locale-recordings', 'version': 1, 'locale': locale,
                'locale_settings': LOCALES[locale], 'camera': CAMERA, 'hud': receipt['hud'],
                'samples': receipt['samples'], 'author_receipt_sha256': digest(author / 'receipt.json'),
                'project_sha256': receipt['project_sha256'], 'cases': rows,
                'sample_sources': {p.relative_to(ROOT).as_posix(): digest(p) for p in sources},
                'route_changes': False, 'native_execution': False}
    write(output / 'recordings.json', manifest)
    return manifest


def recordings(path, locale):
    manifest = read(path / 'recordings.json')
    require(manifest['format'] == 'incant-core-locale-recordings'
            and type(manifest['version']) is int and manifest['version'] == 1
            and manifest['route_changes'] is False and manifest['native_execution'] is False,
            'Known data-only recording manifest required')
    require(locale in LOCALES and manifest['locale'] == locale
            and locale_settings(manifest['locale_settings'], locale) and manifest['camera'] == CAMERA,
            'Exact locale/pseudo recording identity')
    require([(r['name'], r['host_steps']) for r in manifest['cases']] == list(zip(ORDER, TICKS))
            and all(integer(r['host_steps']) and integer(r['simulation_tick']) for r in manifest['cases']),
            'Exact three unchanged routes')
    for row in manifest['cases']:
        require(row['simulation_tick'] == (2156 if row['name'] == ORDER[2] else row['host_steps']), 'Reset simulation clock')
        for suffix, key in (('-input.json', 'recording_sha256'), ('-observe.json', 'observation_sha256')):
            require(digest(path / (row['name'] + suffix)) == row[key], 'Recording/observation identity')
        value = read(path / (row['name'] + '-input.json'))
        require(value['format'] == 'incant-input' and type(value['version']) is int and value['version'] == 1
                and value['tick_rate'] == 60 and value['start_tick'] == 0 and value['ticks'] == row['host_steps'],
                'Unchanged physical recording clock')
        require(all(set(f) == {'tick', 'events'} and all(e['type'] in
                    ('key', 'gamepad_connected', 'gamepad_disconnected', 'gamepad_axis', 'gamepad_button')
                    for e in f['events']) for f in value['frames']), 'No UI/viewport/focus injection')
    for name, expected in manifest['sample_sources'].items():
        relative(name)
        require(name.startswith('samples/core/') and digest(ROOT / name) == expected, 'Current authored sample source identity')
    return manifest


def package(path, manifest, selected=None):
    receipt = read(path / 'export.json')
    require(receipt['format'] == 'incant-native-export' and type(receipt['version']) is int and receipt['version'] == 1
            and receipt['mode'] == 'window' and receipt['camera'] == ulid_hex(CAMERA)
            and receipt['project_sha256'] == manifest['project_sha256']
            and 'profiling' not in receipt
            and receipt['startup_source_parsing'] is False and receipt['build_storage_removed_before_startup'] is True,
            'Exact locale ordinary source-free window export required')
    player = path / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    require(digest(player) == receipt['player_sha256'], 'Exported executable identity')
    for name, expected in receipt['data']['files'].items():
        relative(name)
        require(digest(path / 'player/data' / name) == expected, 'Exported data identity')
    if selected is not None:
        require(digest(selected) == receipt['player_sha256'], 'Selected app differs from export')
        player = selected
    return player, receipt


def observations(report, plan):
    observed = report['observations']
    require(observed['passed'] is True and integer(observed['tick']) and observed['tick_rate'] == 60
            and set(observed['entities']) == set(plan['entities']), 'Complete actual observation entity set')
    require(len(observed['checks']) == len(plan['checks']) and len(plan['checks']) > 0,
            'No discarded observation checks')
    # Plan::new canonicalizes requests by (tick,name) before executing them.
    # Sort only the expected requests; never sort away a malformed result order.
    expected_checks = sorted(plan['checks'], key=lambda c: (c['tick'], c['name']))
    for actual, expected in zip(observed['checks'], expected_checks):
        require(all(actual[k] == expected[k] for k in ('name', 'tick', 'expect'))
                and integer(actual['tick']) and actual['passed'] is True, 'Exact successful observation plan')
        if expected['expect']['type'] == 'exists':
            require(actual['actual'] is expected['expect']['value'], 'Actual boolean existence result')


def check_cpu(row, report, manifest, plan):
    require(integer(report['tick']) and report['tick'] == row['simulation_tick']
            and integer(report['frames']) and report['frames'] == row['host_steps'] * 800, 'CPU simulation/PCM clocks')
    observations(report, plan)
    require(report['observations']['tick'] == row['simulation_tick'], 'Observation simulation clock')
    logs = report['logs']
    require(integer(logs['dropped']) and logs['dropped'] == 0, 'Dropped gameplay logs')
    entries = logs['entries']
    require(all(integer(r['tick'], 1) for r in entries), 'Integer gameplay log clocks')
    pickups = [r for r in entries if r['message'].startswith('core.pickup ')]
    terminal = [r for r in entries if r['message'].startswith(('core.won ', 'core.lost '))]
    won = row['name'] != 'lose-short'
    require([r['message'].split()[1] for r in pickups] == (manifest['samples'] if won else [])
            and [r['tick'] for r in pickups] == ([127, 508, 1266, 1842] if won else []), 'Actual complete sample collection')
    require(len(terminal) == 1 and terminal[0]['message'].startswith('core.won ' if won else 'core.lost ')
            and (terminal[0]['tick'] == 2066 if won else terminal[0]['tick'] <= 900), 'Actual terminal gameplay event')
    damage = [r for r in entries if r['message'].startswith('core.damage ')]
    require(damage and int(damage[-1]['message'].rsplit('=', 1)[1]) == (2 if won else 0), 'Actual final damaged health')
    hud = manifest['hud']
    for role in ('won', 'lost', 'prompt'):
        shown = role == 'prompt' or (role == 'won') == won
        require(report['observations']['entities'][hud[role]]['parent'] == hud['result' if shown else 'inactive'],
                'Actual result/prompt parenting')
    render = report['render_input']
    ui = render['ui']
    require(render['mode'] == 'cpu-only' and render['gpu_submitted'] is False
            and integer(render['model_count'], 1) and render['camera'] == ulid_hex(CAMERA), 'Real CPU scene/HUD input')
    require(integer(ui['widgets']) and ui['widgets'] == 13 and integer(ui['glyphs'], 1)
            and ui['viewport'] == [640, 480] and ui['focus'] is None and ui['sources'] == row['labels'],
            'Exact canonical localized logical HUD')
    if row['name'] == ORDER[2]:
        expected = {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156,
                    'restarts': [{'host_step': 900, 'completed_generation': 1, 'simulation_tick': 900, 'output_frames': '720000'}]}
        require(report['session'] == expected and all(integer(report['session'][k]) for k in ('generation', 'host_tick', 'simulation_tick')),
                'Exact whole-session restart receipt')
    else:
        require('session' not in report, 'Unexpected standalone session reset')


def check_restart(reports, pcm):
    loss, win, reset = (reports[n] for n in ORDER)
    for key in ('logs', 'render_input', 'flipbooks', 'sources'):
        require(reset[key] == win[key], 'Fresh winning generation parity: ' + key)
    require(reset['observations']['entities'] == win['observations']['entities'], 'Fresh generation entity parity')
    require(pcm[ORDER[2]] == pcm[ORDER[0]] + pcm[ORDER[1]], 'Exact loss + fresh win PCM after reset')
    require(not any(pcm[ORDER[0]][-60 * TICK_BYTES:]), 'Actual loss result cue drained')


def check_window(row, report, baseline, manifest, plan):
    require(report['mode'] == 'native-window' and report['on_screen'] is True and report['backend'] == 'Metal', 'Actual Mac Metal window')
    require(integer(report['host_steps']) and report['host_steps'] == 3056
            and integer(report['tick']) and report['tick'] == 2156, 'Complete composite with reset simulation clock')
    recording = report['input_recording']
    require(recording['sha256'] == row['recording_sha256'] and recording['completed'] is True
            and recording['source'] == 'recorded-physical-input'
            and all(integer(recording[k]) and recording[k] == 3056 for k in ('requested_host_steps', 'accepted_host_steps'))
            and recording['physical_devices'] is False and recording['native_focus_and_surface_required'] is True,
            'Complete physical replay under actual native eligibility')
    observations(report, plan)
    require(report['observations'] == baseline['observations'], 'Exact same-package CPU/window observations')
    refresh = report['presentation_refreshes']
    session = report['session']
    require(integer(refresh) and all(integer(session[k]) for k in ('generation', 'host_tick', 'simulation_tick', 'accepted_frames'))
            and session == {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156, 'accepted_frames': 3056 + refresh},
            'Cumulative session presentation receipt')
    # The replacement consumer owns only generation 2. Refreshes may straddle
    # generations; no comparison to cumulative 3056 host steps is valid here.
    require(integer(report['accepted_frames']) and 2156 <= report['accepted_frames'] <= 2157 + refresh,
            'Current-generation accepted frames')
    ui = report['ui']
    require(integer(ui['widgets']) and ui['widgets'] == 7 and integer(ui['glyph_quads'], 1)
            and len(ui['viewport']) == 2 and all(type(v) in (int, float) and math.isfinite(v) and v >= 16 for v in ui['viewport'])
            and ui['focus'] is None, 'Seven native text inputs; pixels/text appearance reviewed separately')


@contextmanager
def bounded(seconds, output, byte_cap, log_cap):
    """Cooperative execution deadline; owned cleanup and blocked IO may finish later."""
    require(threading.current_thread() is threading.main_thread() and hasattr(signal, 'setitimer'), 'POSIX main thread required')
    require(signal.getitimer(signal.ITIMER_REAL) == (0., 0.), 'Another alarm is active')
    numbers = (signal.SIGALRM, signal.SIGTERM, signal.SIGINT)
    previous = {n: signal.getsignal(n) for n in numbers}
    deadline, cause, sampled = time.monotonic() + seconds, None, float('-inf')
    def latch(number, *_):
        nonlocal cause
        if cause is None:
            cause = TimeoutError('Core locale execution deadline') if number == signal.SIGALRM else InterruptedError('Core locale cancelled')
            signal.setitimer(signal.ITIMER_REAL, 0)
    def check():
        nonlocal sampled
        now = time.monotonic()
        if cause is None and now >= deadline:
            latch(signal.SIGALRM)
        if cause is not None:
            raise cause
        if now - sampled >= 1:
            sampled = now
            paths = [p for p in output.rglob('*') if p.is_file()]
            require(shutil.disk_usage(output).free >= FLOOR, 'Six GiB free floor reached')
            require(sum(p.stat().st_size for p in paths) <= byte_cap, 'Bounded stage storage exceeded')
            require(sum(p.stat().st_size for p in paths if p.name.endswith('.raw.log')) <= log_cap, 'Bounded raw output exceeded')
    try:
        for n in numbers:
            signal.signal(n, latch)
        signal.setitimer(signal.ITIMER_REAL, seconds)
        yield check
        sampled = float('-inf')
        check()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for n, handler in previous.items():
            signal.signal(n, handler)


@contextmanager
def readiness(argv, output):
    """Observe exact owned argv; never activates, opens or relaunches any app."""
    stop, records, errors = threading.Event(), [], []
    expected = ' '.join(argv)
    def observe():
        try:
            until = time.monotonic() + 20
            while not stop.is_set() and time.monotonic() < until:
                found = subprocess.run(['ps', '-axo', 'pid=,pgid=,command='], capture_output=True, text=True, timeout=2)
                require(found.returncode == 0, 'Owned-child process inventory failed')
                matches = [line.split(None, 2) for line in found.stdout.splitlines() if len(line.split(None, 2)) == 3
                           and line.split(None, 2)[2] == expected]
                require(len(matches) <= 1, 'Ambiguous exact app argv')
                if matches:
                    pid, pgid, _ = matches[0]
                    require(pid == pgid, 'App must own its managed process group')
                    row = {'stage': 'owned_child_live', 'pid': int(pid), 'pgid': int(pgid), 'argv': argv,
                           'created_utc': datetime.now(timezone.utc).isoformat()}
                    records.append(row); write(output / 'owned-live.json', row)
                    print(json.dumps(row), flush=True)
                    return
                stop.wait(.05)
        except BaseException as error:
            errors.append(repr(error))
    thread = threading.Thread(target=observe, name='core-locale-owned-readiness')
    thread.start()
    try:
        yield records
    finally:
        stop.set(); thread.join()
        write(output / 'readiness.json', {'records': records, 'errors': errors, 'monitor_joined': True})


def run_stage(argv, output, name, timeout, check, ledger, env=None, cwd=ROOT, live=False):
    argv = list(map(str, argv))
    row = {'argv': argv, 'timeout_seconds': timeout, 'started_utc': datetime.now(timezone.utc).isoformat(),
           'managed_call_finished': False}
    ledger.append(row)
    out, err = (output / (name + suffix + '.raw.log') for suffix in ('.stdout', '.stderr'))
    try:
        with (readiness(argv, output) if live else nullcontext([])) as children:
            with out.open('xb') as stdout, err.open('xb') as stderr:
                try:
                    result = native_process.run(argv, cwd=cwd, env=env, timeout=timeout, stdout=stdout, stderr=stderr, cancel=check)
                finally:
                    # The helper guarantees owned cleanup before every return or
                    # exception, including timeouts/cancellation/decoding errors.
                    row['managed_call_finished'] = True
        row['exit_code'] = result.returncode
        require(result.returncode == 0, 'Child failed; retained raw output')
        if live:
            require(len(children) == 1 and not read(output / 'readiness.json')['errors'], 'Exact owned live PID not observed')
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
    for field in ('package', 'recordings', 'baseline', 'player', 'headless', 'art'):
        value = getattr(args, field, None)
        if value is not None:
            setattr(args, field, value.resolve(strict=True))
    output = fresh(args.output)
    ledger, result = [], {'passed': False, 'stage': args.stage, 'locale': args.locale,
                          'appearance_review': False, 'audible_quality': False, 'performance_or_phase_gate': False}
    try:
        seconds, byte_cap, log_cap = (900, 1024**3, 64*1024**2) if args.stage == 'author' else (
            (300, 48*1024**2, 4*1024**2) if args.stage == 'baseline' else (180, 8*1024**2, 4*1024**2))
        with bounded(seconds, output, byte_cap, log_cap) as check:
            if args.stage == 'author':
                run_stage([sys.executable, '-B', ROOT / 'samples/core/scene.py', output / 'authoring',
                           '--binary', args.headless, '--art', args.art, '--locale', args.locale, '--state', 'playing'],
                          output, 'author', 600, check, ledger)
                receipt = authoring(output / 'authoring', args.locale)
                result.update(author_receipt_sha256=digest(output / 'authoring/receipt.json'), project_sha256=receipt['project_sha256'])
            else:
                manifest = recordings(args.recordings, args.locale)
                input_files = inventory(args.recordings)
                player, exported = package(args.package, manifest, getattr(args, 'player', None))
                before = inventory(args.package / 'player')
                if args.stage == 'window':
                    baseline = read(args.baseline / 'result.json')
                    require(sys.platform == 'darwin' and baseline['passed'] is True and baseline['stage'] == 'baseline'
                            and baseline['locale'] == args.locale and baseline['package_files'] == before
                            and baseline['recordings_sha256'] == digest(args.recordings / 'recordings.json'),
                            'Exact same-locale same-package CPU baseline required')
                env = {k: v for k, v in os.environ.items() if k in ('HOME', 'USER', 'LOGNAME', 'LANG', 'LC_ALL')}
                env.update(PATH=str(player.parent), TMPDIR=str(output), RAYON_NUM_THREADS='2')
                reports, pcm = {}, {}
                for row in manifest['cases']:
                    name = row['name']
                    if args.stage == 'window' and name != ORDER[2]:
                        continue
                    plan = read(args.recordings / (name + '-observe.json'))
                    argv = [player, '--package', args.package / 'player/data']
                    argv += ['--window', '--stop-after', str(row['host_steps']), '--window-state-log', output / 'eligibility.jsonl'] if args.stage == 'window' else [
                        '--ticks', str(row['host_steps']), '--ui-viewport', '640x480', '--output', output / (name + '.pcm')]
                    argv += ['--input-recording', args.recordings / (name + '-input.json'), '--observe', args.recordings / (name + '-observe.json')]
                    raw = run_stage(argv, output, name, 120 if args.stage == 'window' else 90, check, ledger,
                                    env, player.parent, args.stage == 'window')
                    report = read(raw)
                    if args.stage == 'baseline':
                        check_cpu(row, report, manifest, plan)
                        data = (output / (name + '.pcm')).read_bytes()
                        require(len(data) == row['host_steps'] * TICK_BYTES and all(math.isfinite(v[0]) for v in struct.iter_unpack('<f', data))
                                and digest(output / (name + '.pcm')) == report['pcm_sha256'], 'Exact finite PCM clock/hash')
                        reports[name], pcm[name] = report, data
                    else:
                        reference = read(args.baseline / (name + '.json'))
                        check_cpu(row, reference, manifest, plan)
                        check_window(row, report, reference, manifest, plan)
                    write(output / (name + '.json'), report)
                if args.stage == 'baseline':
                    check_restart(reports, pcm)
                    result['winning_gameplay_events'] = [r for r in reports['win']['logs']['entries']
                        if r['message'].startswith(('core.damage ', 'core.won '))]
                require(inventory(args.package / 'player') == before and inventory(args.recordings) == input_files, 'Runtime changed package/recordings')
                result.update(package_files=before, player_sha256=exported['player_sha256'],
                              recordings_sha256=digest(args.recordings / 'recordings.json'))
            result['passed'] = True
    except BaseException as error:
        result['error'] = repr(error)
        raise
    finally:
        result['managed_calls_finished'] = all(r['managed_call_finished'] for r in ledger)
        if args.stage == 'window':
            log = output / 'eligibility.jsonl'
            result['window_state_log'] = {'present': log.is_file()}
            if log.is_file():
                result['window_state_log'].update(bytes=log.stat().st_size, sha256=digest(log))
        write(output / 'result.json', result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest='stage', required=True)
    for mode in ('recordings', 'author', 'baseline', 'window'):
        p = modes.add_parser(mode)
        p.add_argument('--locale', choices=LOCALES, required=True)
        if mode == 'recordings':
            p.add_argument('author', type=Path)
        elif mode == 'author':
            p.add_argument('--headless', type=Path, required=True)
            p.add_argument('--art', type=Path, required=True)
        else:
            p.add_argument('package', type=Path); p.add_argument('recordings', type=Path)
            if mode == 'window':
                p.add_argument('--baseline', type=Path, required=True)
                p.add_argument('--player', type=Path, help='Unique app copy must match the actual export')
        p.add_argument('output', type=Path)
    args = parser.parse_args()
    generate(args.author.resolve(strict=True), args.locale, args.output) if args.stage == 'recordings' else execute(args)


if __name__ == '__main__':
    main()
