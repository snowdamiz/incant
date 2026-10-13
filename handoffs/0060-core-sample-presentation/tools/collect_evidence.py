#!/usr/bin/env python3
"""collect_evidence.py TAG ART: summarize the 0060 evidence tree into the packet.

Reads ~/.codex/incant-evidence/0060 (builds/TAG-*, packages/TAG-*, renders/TAG-*,
playtest-TAG, art/ART, nav, checks, runs) and writes compact JSON into
handoffs/0060-core-sample-presentation/evidence/, then copies the curated screenshots
listed in SHOTS into screenshots/ with their digests. Read-only on the evidence tree.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
ROOT = HERE.parents[2]
S = Path(os.path.expanduser('~/.codex/incant-evidence/0060'))
PROTECTED = ['samples/core/gameplay.ts', 'samples/core/verify.py', 'samples/core/tsconfig.json']

# (screenshot name, evidence-relative source, what it shows)
SHOTS = [
    ('window-en-start.png', 'runs/w8-en/caps/w8-start.png', 'native window, 640x480 pt at 2x, start (f5)'),
    ('window-en-sample1-south.png', 'runs/w8-en/caps/w8-collect-s1.png',
     'native window, sample 1 taken walking north along its x, the corridor the old hoodoo blocked (f5)'),
    ('window-en-420x320.png', 'runs/w13-en/caps/w13-420x320.png', 'native window resized to 420x320 pt (f5)'),
    ('window-en-960x540.png', 'runs/w3-en/caps/w3-960x540.png',
     'native window resized to 960x540 pt (f4: same scene, 13 pt hint and 32 pt gap)'),
    ('window-ar-start.png', 'runs/w9-ar/caps/w9-start.png', 'native window, Arabic (RTL) HUD (f5)'),
    ('window-zh-Hans-start.png', 'runs/w10-zh/caps/w10-start.png', 'native window, Simplified Chinese HUD (f5)'),
    ('window-en-XA-start.png', 'runs/w11-pseudo/caps/w11-start.png', 'native window, pseudo-locale HUD (f5)'),
    ('window-lost.png', 'runs/w12-lost/caps/w12-lost.png', 'native window, lost state (f5)'),
    ('window-before-fix-sample1-west.png', 'runs/w1-en/caps/w1-collect-s1.png',
     'before the layout fix (f3): the wash hoodoo fills the frame at sample 1'),
    ('window-before-fix-420x320.png', 'runs/w3-en/caps/w3-420x320.png',
     'before the HUD fit fix (f4): the controls hint wraps and orphans "jump" at 420x320'),
    ('offscreen-route-pocket-s2.png', 'renders/{tag}-en-playing-route-playing/route-pocket-s2.png',
     'planned route tick 500: sample 2 in the pocket'),
    ('offscreen-route-shelf-s3.png', 'renders/{tag}-en-playing-route-playing/route-shelf-s3.png',
     'planned route tick 1250: sample 3 on the east shelf'),
    ('offscreen-route-plateau-s4.png', 'renders/{tag}-en-playing-route-playing/route-plateau-s4.png',
     'planned route tick 1830: sample 4 on the plateau'),
    ('offscreen-collected.png', 'renders/{tag}-en-collected-route-collected/route-all-collected.png',
     'collected HUD state at tick 1905'),
    ('offscreen-won.png', 'renders/{tag}-en-won-route-won/route-won.png', 'won HUD state at the lift, tick 2110'),
]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def write(name, value):
    out = PACKET / 'evidence' / name
    out.parent.mkdir(exist_ok=True)
    out.write_text(json.dumps(value, indent=1, ensure_ascii=False) + '\n', encoding='utf-8')


def png_size(path):
    data = Path(path).read_bytes()[16:24]
    return [int.from_bytes(data[:4], 'big'), int.from_bytes(data[4:], 'big')]


def sources():
    changed = git('status', '--porcelain', '--untracked-files=all', '--', 'samples/core',
                  'handoffs/0060-core-sample-presentation').splitlines()
    files = {}
    for line in changed:
        path = line[3:]
        if path.startswith('handoffs/0060-core-sample-presentation/') and \
                path.split('/')[2] in ('evidence', 'screenshots', 'result.md'):
            continue
        files[path] = {'status': line[:2].strip(), 'sha256': sha(ROOT / path)}
    protected = {}
    for path in PROTECTED:
        head = git('rev-parse', f'HEAD:{path}').strip()
        work = git('hash-object', path).strip()
        protected[path] = {'sha256': sha(ROOT / path), 'git_blob': work, 'matches_head': head == work}
    return {'head': git('rev-parse', 'HEAD').strip(), 'files': dict(sorted(files.items())),
            'protected_unchanged': protected}


def builds(tag):
    out, common = {}, None
    for receipt_path in sorted(S.glob(f'builds/{tag}-*/receipt.json')):
        r = load(receipt_path)
        name = receipt_path.parent.name
        out[name] = {'locale': r['locale'], 'state': r['state'], 'project_sha256': r['project_sha256'],
                     'revision': r['revision'], 'rpc_calls': r['rpc_calls'],
                     'transactions': r['transactions'], 'undo_redo_reopen': r['undo_redo_reopen'],
                     'validation': r['validation'], 'glyph_coverage': r['glyph_coverage'],
                     'localized_text': r['localized_text']}
        if common is None:
            common = {k: r[k] for k in ('binary_sha256', 'gameplay_sha256', 'entities', 'enemies', 'samples',
                                        'script', 'scene', 'assets', 'settings', 'audio', 'widgets', 'fonts')}
            common['imports'] = {k: {'id': v['id'], 'kind': v['kind'], 'sha256': v['sha256']}
                                 for k, v in r['imports'].items()}
    return {'common': common, 'variants': out}


def packages(tag):
    out = {}
    for path in sorted(S.glob(f'packages/{tag}-*/package.json')):
        p = load(path)
        out[path.parent.name] = {
            'project_sha256': p['project_sha256'], 'player_sha256': p['player_sha256'],
            'player_bytes': p['player_bytes'], 'data_bytes': p['data']['bytes'],
            'data_files': len(p['data']['files']), 'startup_frames': p['startup']['frames'],
            'startup_logs': [e['message'] for e in p['startup']['logs']['entries']]}
        command, profile, features, limits = p['command'], p['profile'], p['features'], p['limits']
    return {'command': [Path(c).name if c.startswith('/') else c for c in command], 'profile': profile,
            'features': features, 'limits': limits, 'packages': out}


def offscreen(tag):
    captures, adapter = [], None
    for review in sorted(S.glob(f'renders/{tag}-*/review.json')):
        d = load(review)
        adapter = {'adapter': d['adapter'], 'backend': d['backend'], 'target_format': d['target_format']}
        for case in d['cases']:
            for c in case['captures']:
                png = review.parent / c['capture']
                captures.append({'run': review.parent.name, 'guest': d['guest'], 'capture': c['capture'],
                                 'file_sha256': sha(png), 'rgba_sha256': c['png_rgba_sha256'],
                                 'size_px': c['size'], 'scale': case['scale'], 'tick': c['tick'],
                                 'scene': c.get('scene'), 'ui': c['ui']})
            logs = [f"{e['tick']} {e['message']}" for e in case['guest_logs'] if not e['message'].startswith('core.route')]
            if logs:
                captures[-1]['run_logs'] = logs
    return {'route': 'incant_render_gpu_host HostedOfflineGameSession + GpuRenderConsumer, package data, '
                     'cooked program from the package producer', **(adapter or {}), 'captures': captures}


def window():
    runs = []
    images = (S / 'images.log').read_text(encoding='utf-8').splitlines()
    processes = (S / 'processes.log').read_text(encoding='utf-8').splitlines()
    for run_dir in sorted((S / 'runs').iterdir()):
        exit_path = run_dir / 'exit.json'
        if not exit_path.exists():
            continue
        e = load(exit_path)
        report = e['report'] or {}
        launched = next((l for l in processes if f' launched {run_dir.name} ' in l), '')
        opened = next((l for l in processes if f' {run_dir.name} pid ' in l and ' window ' in l), '')
        notes = run_dir / 'notes.txt'
        caps = []
        for png in sorted((run_dir / 'caps').glob('*.png')):
            line = next((l for l in images if str(png) in l), '')
            caps.append({'file': png.name, 'sha256': sha(png), 'size_px': png_size(png),
                         'logged': line.split(' capture ')[0] if line else None,
                         'window': json.loads(line[line.rfind('{'):]) if '{' in line else None})
        runs.append({'run': run_dir.name,
                     'package': re.search(r'packages/([^/]+)/player', launched).group(1) if launched else None,
                     'player_sha256': re.search(r'sha256 ([0-9a-f]{64})', launched).group(1) if launched else None,
                     'opened': opened.split(' window ', 1)[1] if opened else None,
                     'exit_status': e['exit_status'], 'signal': e['signal'], 'ended_at': e['ended_at'],
                     'new_crash_reports': e['new_crash_reports'],
                     'report': {k: report.get(k) for k in ('mode', 'adapter', 'backend', 'tick', 'accepted_frames',
                                                           'dropped_wall_ticks', 'surface_reconfigurations',
                                                           'surface_recreations', 'surface_skipped', 'on_screen', 'ui')},
                     'notes': notes.read_text(encoding='utf-8').splitlines() if notes.exists() else [],
                     'captures': caps})
    bursts = (S / 'bursts.log').read_text(encoding='utf-8').splitlines()
    return {'route': 'ordinary ci-profile window player: incant_game_player --package data --window',
            'capture': 'screencapture -o -x -l <window id> (window only, no shadow)',
            'bursts': {'started': sum(' burst start' in l for l in bursts),
                       'exit_0': sum(' burst end, exit 0' in l for l in bursts),
                       'yielded_to_user_input': sum(' burst end, exit 7' in l for l in bursts)},
            'runs': runs}


def navigation():
    out = {}
    for name in ('sweep-old-settings-full.txt', 'sweep-chosen-settings-full.txt', 'sweep-v13-full.txt'):
        path = S / 'nav' / name
        if not path.exists():
            continue
        text = path.read_text(encoding='utf-8')
        out[name] = {'settings': re.findall(r'^settings .*$', text, re.M)[-1:],
                     'polygons': re.findall(r'^polygons=.*$', text, re.M)[-1:],
                     'summary': re.findall(r'^queries .*$', text, re.M)[-1:]}
    return out


def checks():
    tests = {}
    for log in sorted((S / 'checks').glob('test-*.txt')):
        text = log.read_text(encoding='utf-8', errors='replace')
        tests[log.stem] = {'result': re.findall(r'^test result: .*$', text, re.M),
                           'ran': re.findall(r'^\s*Running (\S+)', text, re.M)}
    l10n = {p.stem: load(p) for p in sorted((S / 'checks').glob('l10n-*.json'))}
    extra = load(S / 'checks' / 'checks.json') if (S / 'checks' / 'checks.json').exists() else {}
    refusals = {name: (S / 'checks' / f'{name}.err').read_text(encoding='utf-8').strip()
                for name in ('native-play-ui', 'native-play-ui-camera') if (S / 'checks' / f'{name}.err').exists()}
    audio = load(S / 'checks' / 'audio-pcm.json') if (S / 'checks' / 'audio-pcm.json').exists() else None
    if audio:
        audio['pcm'] = str(Path(audio['pcm']).relative_to(S))
        audio['loop'] = str(Path(audio['loop']).relative_to(S))
    return {'render_tests': tests, 'crates_unchanged_by_tests':
            (S / 'checks/crates-status-before.txt').read_bytes() == (S / 'checks/crates-status-after.txt').read_bytes(),
            'localization_check': l10n, 'native_play_refusals': refusals, 'audio_mixer_output': audio, **extra}


def screenshots(tag):
    target = PACKET / 'screenshots'
    target.mkdir(exist_ok=True)
    index = []
    for name, source, what in SHOTS:
        path = S / source.format(tag=tag)
        if not path.exists():
            print('missing screenshot source', path, file=sys.stderr)
            continue
        shutil.copyfile(path, target / name)
        index.append({'screenshot': f'screenshots/{name}', 'sha256': sha(target / name),
                      'size_px': png_size(path), 'source': str(path.relative_to(S)), 'shows': what})
    return index


def main():
    tag, art = sys.argv[1], sys.argv[2]
    manifest = load(S / 'art' / art / 'manifest.json')
    write('sources.json', sources())
    write('art.json', {'command': f'cd samples/core && python3 -B -m art.world OUT ({art})', **manifest})
    write('builds.json', builds(tag))
    write('packages.json', packages(tag))
    write('playtest.json', load(S / f'playtest-{tag}' / 'playtest.json'))
    write('offscreen.json', offscreen(tag))
    write('window.json', window())
    write('navigation.json', navigation())
    write('checks.json', checks())
    write('screenshots.json', screenshots(tag))
    print('evidence written to', PACKET / 'evidence')


if __name__ == '__main__':
    main()
