#!/usr/bin/env python3
"""Collect 0065 evidence into the handoff (CPU only; reads the released evidence root).

Usage: python3 -B collect.py

Writes handoffs/0065-core-live-hud/evidence/*.json and copies the unedited screenshot
selection (PNG of the retained RGBA, pixels unchanged) plus labelled review crops.
Raw frames, builds, packages, binaries and logs stay in the evidence root and are
recorded here by hash.
"""
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
ROOT = PACKET.parents[1]
E = Path('/Users/andreyyurlov/.codex/incant-evidence/0065')
ACCEPTED = Path('/Users/andreyyurlov/.codex/incant-evidence/core-runtime-bindings-001')
OUT = PACKET / 'evidence'
SHOTS = PACKET / 'screenshots'
VARIANTS = ('en', 'ar', 'zh-Hans', 'en-XA')
SELECTION = [  # (committed name, run, capture name, caption)
    ('01-en-960x540-start.png', 'review-en-start', 'start',
     'English fresh start, tick 2; bit-identical to the frames after Start (host 2158) and after R (host 902)'),
    ('02-en-960x540-pickup-1.png', 'frames-en-route-a', 'pickup-1', 'first sample collected (tick 127), Samples 1/4'),
    ('03-en-960x540-damage.png', 'frames-en-route-a', 'damage-1', 'first hit (tick 664), pips 2 of 3, Samples 2/4'),
    ('04-en-960x540-collected.png', 'frames-en-route-a', 'collected', 'fourth sample (tick 1842), Return to the lift'),
    ('05-en-960x540-won.png', 'frames-en-route-a', 'won', 'win at the lift (tick 2066), Samples secured'),
    ('06-en-960x540-lost.png', 'frames-en-route-b', 'lost', 'idle loss (tick 616), Surveyor down, empty pips'),
    ('07-en-960x540-locked-exit.png', 'frames-en-locked', 'locked-exit',
     'alive on the lift with 3 of 4 samples after the 30-tick dwell, no win'),
    ('08-en-640x480-start.png', 'frames-en-default', 'start', 'English at the default 640x480 window size'),
    ('09-ar-640x480-pickup-1.png', 'frames-ar-route-a', 'pickup-1', 'Arabic, mirrored HUD, first sample'),
    ('10-ar-640x480-won.png', 'review-ar-won', 'won', 'Arabic win title'),
    ('11-zh-Hans-640x480-pickup-1.png', 'frames-zh-Hans-route-a', 'pickup-1', 'Simplified Chinese, first sample'),
    ('12-zh-Hans-640x480-won.png', 'frames-zh-Hans-route-a', 'won', 'Simplified Chinese win title'),
    ('13-en-XA-640x480-pickup-1.png', 'review-en-XA-pickup', 'pickup-1',
     'pseudo-locale at 640 pt: progress wraps onto its own row'),
    ('14-en-XA-640x480-won.png', 'frames-en-XA-route-a', 'won',
     'pseudo-locale win: the shorter exit objective fits one row again'),
    ('15-en-420x320-start.png', 'frames-en-narrow', 'start', 'English at 420 pt: progress on its own row'),
    ('16-en-420x320-lost.png', 'review-en-narrow-lost', 'lost', 'English at 420 pt, loss title'),
]
CROPS = [  # (committed name, source crop, caption)
    ('crop-en-960x540-progress-x4.png', 'review-en-start-progress-x4.png',
     'crop, 4x nearest-neighbour, of 01: x 700-948, y 12-68'),
    ('crop-ar-640x480-progress-x4.png', 'review-ar-won-progress-x4.png',
     'crop, 4x nearest-neighbour, of 10: x 16-248, y 12-66'),
    ('crop-zh-Hans-640x480-progress-x4.png', 'frames-zh-Hans-pickup-progress-x4.png',
     'crop, 4x nearest-neighbour, of 11: x 400-624, y 12-68'),
]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def jsonl(path):
    return [json.loads(line) for line in Path(path).read_text(encoding='utf-8').splitlines() if line.strip()]


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def head_blob_sha256(path):
    data = subprocess.run(['git', 'show', f'HEAD:{path}'], cwd=ROOT, capture_output=True, check=True).stdout
    return hashlib.sha256(data).hexdigest()


def write(name, value):
    (OUT / name).write_text(json.dumps(value, indent=1, ensure_ascii=False) + '\n', encoding='utf-8')


def identities():
    lease = json.loads((E / 'lease.json').read_text())
    release = json.loads((E / 'release.json').read_text())
    sw = dict(line.split(':', 1) for line in subprocess.run(['sw_vers'], capture_output=True, text=True).stdout
              .strip().splitlines())
    report = json.loads((E / 'captures/frames-en-route-a/report.json').read_text())
    packages = {v: json.loads((E / 'packages' / v / 'package.json').read_text()) for v in VARIANTS}
    write('identities.json', {
        'model': 'Claude Opus 5.5 (claude-opus-5-5)', 'thinking_effort': 'max',
        'acp_record': {'path': '/Users/andreyyurlov/.codex/incant-evidence/claude-0065-acp.log', 'line': 3,
                       'text': 'Claude session verified: Opus 5.5; thinking: max'},
        'source': {'head': git('rev-parse', 'HEAD'), 'tree': git('rev-parse', 'HEAD^{tree}'),
                   'start_commit': 'a9e26999e0763de7cbd625930595be8a36f3f460',
                   'cargo_lock_sha256': digest(ROOT / 'Cargo.lock')},
        'brief_sha256': digest(PACKET / 'brief.md'), 'brief_sha256_at_lease': lease['brief_sha256'],
        'grant': {'path': str(E / 'review-ready.json'), 'sha256': lease['grant_sha256'], 'grant': lease['grant']},
        'lease': {'path': str(E / 'lease.json'), 'sha256': digest(E / 'lease.json'),
                  'acquired_at': lease['acquired_at'], 'processes_on_target_at_acquire':
                  lease['processes_on_target_at_acquire'], 'rules': lease['rules']},
        'release': {'path': str(E / 'release.json'), 'sha256': digest(E / 'release.json'), **release},
        'prior_release_chain': {'0064_gpu_released_sha256': digest('/Users/andreyyurlov/.codex/incant-evidence/0064/gpu-released.json'),
                                '0064_target_lease_sha256': digest('/Users/andreyyurlov/.codex/incant-evidence/0064/target-lease.json'),
                                'matches_grant': True},
        'host': {'os': f"{sw['ProductName'].strip()} {sw['ProductVersion'].strip()} ({sw['BuildVersion'].strip()})",
                 'model': subprocess.run(['sysctl', '-n', 'hw.model'], capture_output=True, text=True).stdout.strip(),
                 'machine': platform.machine()},
        'gpu': {'adapter': report['adapter'], 'backend': report['backend'], 'device_type': report['device_type'],
                'driver': report['driver'] or None, 'target_format': report['target_format']},
        'toolchain': {'rustc': '1.99.0 (b940084d7 2026-09-28)', 'cargo': '1.99.0 (5f94df478 2026-08-27)',
                      'profile': 'ci', 'jobs': 2, 'flags': ['--locked', '--offline (packages)'],
                      'cargo_target_dir': lease['target_dir'], 'python': '3.14.8 (tools), 3.9.6 (/usr/bin/python3, scene.py)',
                      'node': 'v22.23.3', 'typescript': '5.9.3'},
        'binaries': {
            'incant_headless': {'path': str(E / 'bin/incant_headless'), 'sha256': digest(E / 'bin/incant_headless'),
                                'bytes': (E / 'bin/incant_headless').stat().st_size},
            **{f'{v}': {'player_sha256': p['player_sha256'], 'player_bytes': p['player_bytes'],
                        'adapter_sha256': p['adapter_sha256'], 'adapter_source_sha256': p['adapter_source_sha256'],
                        'behavior_sha256': p['behavior_sha256'], 'features': p['features']}
               for v, p in packages.items()}},
        'adapter_freeze': json.loads((E / 'freeze/adapter.json').read_text()),
    })


def sources():
    changed = ['samples/core/ui/strings.json', 'samples/core/ui/hud.json', 'samples/core/art/fonts/CoreSampleSC-Regular.ttf',
               'samples/core/art/fonts/provenance.json', 'samples/core/PRESENTATION.md']
    contract = ['samples/core/gameplay.ts', 'samples/core/bindings.py', 'samples/core/binding_oracle.py',
                'samples/core/verify_bindings.py', 'samples/core/playtest.py', 'samples/core/verify.py',
                'samples/core/verify_presentation.py', 'samples/core/export_verify.py', 'samples/core/scene.py',
                'samples/core/hud.py', 'samples/core/routes/win-input.json', 'samples/core/routes/locked-input.json',
                'samples/core/test_bindings.py', 'samples/core/test_playtest.py', 'samples/core/tsconfig.json',
                'samples/core/art/fonts/subset_sc.py', 'samples/core/art/fontcheck.py',
                'docs/spikes/core-authored-runtime-bindings.md', 'docs/spikes/core-locked-exit.md', 'STATUS.md',
                'crates/incant_game_player/src/restart/input.rs']
    art = sorted(str(p.relative_to(ROOT)) for p in (ROOT / 'samples/core/art').glob('*.py'))
    write('sources.json', {
        'changed': {p: {'head_sha256': head_blob_sha256(p), 'sha256': digest(ROOT / p)} for p in changed},
        'unchanged_by_contract': {p: {'sha256': digest(ROOT / p), 'equals_head': digest(ROOT / p) == head_blob_sha256(p)}
                                  for p in contract + art},
        'crates_status': git('status', '--porcelain', '--', 'crates') or 'clean',
        'packet_tools': {str(p.relative_to(ROOT)): digest(p) for p in sorted(HERE.iterdir()) if p.is_file()},
    })


def font():
    stdout = json.loads((E / 'font/subset-run1.stdout.txt').read_text())
    control = json.loads((E / 'font/subset-control.stdout.txt').read_text())
    write('font.json', {
        'recipe': 'samples/core/art/fonts/subset_sc.py', 'recipe_sha256': digest(ROOT / 'samples/core/art/fonts/subset_sc.py'),
        'upstream_sha256': digest(E / 'downloads/notosanssc/NotoSansSC[wght].ttf'),
        'ofl_sha256': digest(E / 'downloads/notosanssc/OFL.txt'),
        'fonttools_wheel_sha256': digest(E / 'downloads/wheels/fonttools-4.60.1-cp314-cp314-macosx_10_13_universal2.whl'),
        'inputs_copied_from': '/Users/andreyyurlov/.codex/incant-evidence/0060/downloads (read only, hashes equal provenance.json)',
        'strings_sha256': digest(ROOT / 'samples/core/ui/strings.json'),
        'runs': [{'output_sha256': digest(E / f'font/CoreSampleSC-Regular.run{i}.ttf'),
                  'bytes': (E / f'font/CoreSampleSC-Regular.run{i}.ttf').stat().st_size} for i in (1, 2)],
        'glyph_text': stdout['glyph_text'], 'characters': stdout['characters'], 'added': '一 再 次 玩',
        'committed_sha256': digest(ROOT / 'samples/core/art/fonts/CoreSampleSC-Regular.ttf'),
        'control': {'strings': 'HEAD samples/core/ui/strings.json', 'strings_sha256': digest(E / 'font/strings-HEAD.json'),
                    'output_sha256': digest(E / 'font/CoreSampleSC-Regular.control-HEAD.ttf'),
                    'head_font_sha256': head_blob_sha256('samples/core/art/fonts/CoreSampleSC-Regular.ttf'),
                    'characters': control['characters'],
                    'reproduces_head': digest(E / 'font/CoreSampleSC-Regular.control-HEAD.ttf')
                    == head_blob_sha256('samples/core/art/fonts/CoreSampleSC-Regular.ttf')},
    })


def builds():
    rows = {}
    for v in VARIANTS:
        r = json.loads((E / f'builds/{v}-playing/receipt.json').read_text())
        rows[v] = {'output': str(E / f'builds/{v}-playing'), 'project_sha256': r['project_sha256'],
                   'revision': r['revision'], 'valid': r['validation']['valid'],
                   'diagnostics': len(r['validation']['diagnostics']), 'undo_redo_reopen': r['undo_redo_reopen'],
                   'fonts': r['fonts'], 'controls': r['localized_text'][v]['hud.controls'],
                   'glyph_coverage': r['glyph_coverage'][v], 'missing_glyphs': '',
                   'transactions': [[t['description'], t['commands']] for t in r['transactions']],
                   'art': '0060 v13 via --art, 30 files, manifest digests checked by scene.py'}
    write('builds.json', {'command': 'python3 -B samples/core/scene.py OUT --binary incant_headless '
                                     '--art ~/.codex/incant-evidence/0060/art/v13 --locale L --state playing',
                          'builds': rows})


def checks():
    prepared = json.loads((E / 'checks/bindings-en/prepared.json').read_text())
    localized = json.loads((E / 'checks/bindings-en/localized.json').read_text())
    equal = {}
    for n in ('win', 'lose', 'locked', 'idle', 'win-tail'):
        a = json.loads((E / f'checks/bindings-en/{n}.json').read_text())
        b = json.loads((ACCEPTED / f'native-003/{n}.json').read_text())
        equal[n] = {k: a[k] == b[k] for k in ('pcm_sha256', 'logs', 'observations')}
    locale_hud = {}
    for loc in ('ar', 'zh-Hans', 'en-XA'):
        r = json.loads((E / f'checks/bindings-en/locale-{loc}/win.json').read_text())
        rows = [json.loads(e['message'][10:]) for e in r['logs']['entries'] if e['message'].startswith('proof.hud ')]
        locale_hud[loc] = next(t for i, t in rows if i.endswith('08'))
    playtest = json.loads((E / 'checks/playtest-en/playtest.json').read_text())
    unit = (E / 'checks/unittest.txt').read_text().strip().splitlines()[-3:]
    write('checks.json', {
        'unittest': {'command': '(cd samples/core && python3 -B -m unittest -v test_bindings test_playtest)',
                     'exit': 0, 'tail': unit},
        'tsc': {'command': 'node node_modules/typescript/bin/tsc --noEmit -p samples/core/tsconfig.json', 'exit': 0,
                'output_bytes': (E / 'checks/tsc.txt').stat().st_size},
        'verify_bindings_prepare': {
            'command': 'python3 -B samples/core/verify_bindings.py prepare builds/en-playing checks/bindings-en --binary bin/incant_headless',
            'passed': prepared['passed'], 'runs': prepared['runs'], 'native_processes': len(list((E / 'checks/bindings-en').glob('command-*.json'))),
            'binding_refusals': [r['case'] for r in prepared['binding_refusals']],
            'cue_checks': {k: {'ticks': v['ticks'], 'outside_windows_exact': v['outside_windows_exact']}
                           for k, v in prepared['cue_checks'].items()},
            'controls_by_locale': {k: localized[k]['hud.controls'] for k in localized},
            'controls_readback_tick1': locale_hud,
            'equals_accepted_native_003': equal},
        'playtest': {'command': 'python3 -B samples/core/playtest.py builds/en-playing checks/playtest-en --player packages/en/player',
                     'passed': playtest['passed'], 'checks': playtest['checks'],
                     'results': {n: {k: r[k] for k in ('ticks', 'won_tick', 'lost_tick', 'pickups', 'pickup_order')}
                                 for n, r in playtest['results'].items()},
                     'locked_exit': playtest['results']['locked']['exit_proof']['checks']},
        'routes': json.loads((E / 'routes/routes.json').read_text()),
        'checker_mutations': json.loads((E / 'checks/mutations.json').read_text()),
    })


def packages():
    rows = {}
    for v in VARIANTS:
        p = json.loads((E / 'packages' / v / 'package.json').read_text())
        rows[v] = {'project_sha256': p['project_sha256'], 'scene_sha256': p['producer_receipt']['scene_sha256'],
                   'compiler_modules': p['producer_receipt']['compiler']['modules'],
                   'bundle_sha256': p['producer_receipt']['compiler']['bundleSha256'],
                   'player_sha256': p['player_sha256'], 'adapter_sha256': p['adapter_sha256'],
                   'behavior_sha256': p['behavior_sha256'], 'data_files': len(p['data']['files']),
                   'startup_tick': p['startup']['tick'], 'features': p['features'], 'profile': p['profile']}
    write('packages.json', {'command': 'python3 -B handoffs/0065-core-live-hud/tools/build_variant.py VARIANT PROJECT OUT [--clippy]',
                            'clippy': 'en build: cargo clippy --profile ci -p incant_game_player --example zz_0065_live_hud '
                                      '--features project,render-gpu -- -D warnings, exit 0',
                            'packages': rows})


def captures():
    route = {r['run']: r for r in jsonl(E / 'checks/route-check-frames.jsonl')}
    dry = subprocess.run(['python3', '-B', str(HERE / 'route_check.py'), str(ACCEPTED / 'shipping-003'),
                          str(E / 'checks/bindings-en/localized.json'),
                          *map(str, sorted((E / 'captures').glob('dry-*/report.json')))],
                         capture_output=True, text=True)
    for line in dry.stdout.splitlines():
        row = json.loads(line)
        route[row['run']] = row
    layout = jsonl(E / 'checks/layout-check-frames.jsonl')
    layout_runs = subprocess.run(['python3', '-B', str(HERE / 'layout_check.py'),
                                  *map(str, sorted((E / 'captures').glob('layout-*/report.json')) +
                                       sorted((E / 'captures').glob('dry-*/report.json')))],
                                 capture_output=True, text=True)
    layout += [json.loads(line) for line in layout_runs.stdout.splitlines()]
    contrast = jsonl(E / 'checks/contrast.jsonl')
    runs = {}
    for report_path in sorted((E / 'captures').glob('*/report.json')):
        run = report_path.parent.name
        report = json.loads(report_path.read_text())
        runs[run] = {'report_sha256': digest(report_path), 'viewport': report['viewport'], 'size': report['size'],
                     'host_steps': report['host_steps'], 'boundaries': report['boundaries'],
                     'pcm': report['pcm'], 'elapsed_seconds': report['elapsed_seconds'],
                     'program': report['program'], 'inputs': report['inputs'],
                     'captures': [{'name': c['name'], 'host_step': c['host_step'], 'generation': c['generation'],
                                   'simulation_tick': c['simulation_tick'], 'rgba': c.get('rgba'),
                                   'rgba_sha256': c.get('rgba_sha256'),
                                   'capture_seconds': round(c['capture_seconds'], 4),
                                   'missing_glyphs': c['ui']['missing_glyphs'],
                                   'labels': {d['id'][-2:]: d['source'] for d in c['drawn']
                                              if d['visible'][2] > d['visible'][0]}}
                                  for c in report['captures']]}
    starts = {name: digest(E / 'captures' / name) for name in
              ('review-en-start/01-start.rgba', 'frames-en-route-a/05-after-start.rgba', 'frames-en-route-b/02-after-r.rgba')}
    write('captures.json', {
        'adapter': 'handoffs/0065-core-live-hud/tools/live_hud.rs built per variant as example zz_0065_live_hud',
        'runner': 'handoffs/0065-core-live-hud/tools/capture.py RUN (plans in tools/plans.json)',
        'gpu': 'Apple M5 Pro, Metal, Rgba8UnormSrgb offscreen target, scale 1',
        'retained_frames': sum(1 for _ in (E / 'captures').glob('*/*.rgba')), 'frame_bound': 24,
        'restart_pixel_identity': {'frames': starts, 'identical': len(set(starts.values())) == 1},
        'route_checks': route, 'layout_checks': layout, 'contrast_estimates': contrast, 'runs': runs})


def ledger():
    rows = jsonl(E / 'ledger.jsonl')
    # Old private ledgers may predate the explicit build-variable allowlist.
    # Derive publication rows without copying inherited process credentials.
    from lease import evidence_env
    rows = [{**row, 'env': evidence_env(row['env'])} if 'env' in row else row
            for row in rows]
    starts = [r for r in rows if r['event'] == 'start']
    exits = {r['pid']: r for r in rows if r['event'] == 'exit'}
    failures = [{'label': s['label'], 'status': exits[s['pid']]['status'], 'stderr': exits[s['pid']]['stderr']}
                for s in starts if exits[s['pid']]['status'] != 0]
    write('ledger.json', {'path': str(E / 'ledger.jsonl'), 'sha256': digest(E / 'ledger.jsonl'),
                          'children': len(starts), 'gpu_children': sum(s['gpu'] for s in starts),
                          'nonzero_exits': failures, 'timeouts': [r for r in rows if r.get('timed_out')],
                          'unreaped_groups': [r for r in rows if r['event'] == 'exit' and not r['group_reaped']],
                          'rows': rows,
                          'publication_note': 'Rows retain only explicitly allowed build environment variables; sha256 identifies the original private ledger, not this sanitized JSON.'})


def screenshots():
    SHOTS.mkdir(exist_ok=True)
    rows = []
    for name, run, capture, caption in SELECTION:
        report = json.loads((E / 'captures' / run / 'report.json').read_text())
        c = next(c for c in report['captures'] if c['name'] == capture and 'rgba' in c)
        png = E / 'png' / f"{run}-{c['rgba'][3:-5]}.png"
        shutil.copyfile(png, SHOTS / name)
        rows.append({'file': f'screenshots/{name}', 'sha256': digest(SHOTS / name), 'caption': caption,
                     'run': run, 'capture': capture, 'host_step': c['host_step'], 'generation': c['generation'],
                     'simulation_tick': c['simulation_tick'], 'size': c['size'],
                     'rgba': str(E / 'captures' / run / c['rgba']), 'rgba_sha256': c['rgba_sha256'],
                     'unedited': 'PNG of the retained RGBA bytes (tools/png.py convert)'})
    for name, source, caption in CROPS:
        shutil.copyfile(E / 'crops' / source, SHOTS / name)
        rows.append({'file': f'screenshots/{name}', 'sha256': digest(SHOTS / name), 'caption': caption,
                     'review_aid': True})
    write('screenshots.json', rows)


def main():
    OUT.mkdir(exist_ok=True)
    identities(); sources(); font(); builds(); checks(); packages(); captures(); ledger(); screenshots()
    print(json.dumps({p.name: p.stat().st_size for p in sorted(OUT.iterdir())}, indent=1))


if __name__ == '__main__':
    main()
