#!/usr/bin/env python3
"""Copy the 0055 scratch results into the handoff and record every identity.

Usage: python3 -I -B collect.py
Reads /tmp/incant0055 (captures, crops, analyses, layout probe, fixture,
packages, logs) and writes handoffs/0055-bidi-wrap-review/{screenshots,
evidence,fixture}. Logs are committed as .txt (the repository ignores *.log)
with trailing whitespace and final blank lines removed for `git diff --check`;
the raw log hashes are recorded in evidence/identities.json.
"""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
ROOT = PACKET.parents[1]
S = Path('/tmp/incant0055').resolve()


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def text_copy(source, target, start=None):
    lines = Path(source).read_text(encoding='utf-8').splitlines()
    if start is not None:
        lines = lines[next(i for i, line in enumerate(lines) if start in line):]
    lines = [line.rstrip() for line in lines]
    while lines and not lines[-1]:
        lines.pop()
    Path(target).write_text('\n'.join(lines) + '\n', encoding='utf-8')
    return {'raw': str(source), 'raw_sha256': sha(source), 'committed': target.name, 'committed_sha256': sha(target)}


def plane_sha(pgm):
    """The probe's cropped atlas placed in the native 2048 x 2048 R8 plane (zeros elsewhere)."""
    magic, size, depth, pixels = Path(pgm).read_bytes().split(b'\n', 3)
    width, height = map(int, size.split())
    plane = bytearray(2048 * 2048)
    for y in range(height):
        plane[y * 2048:y * 2048 + width] = pixels[y * width:(y + 1) * width]
    return hashlib.sha256(plane).hexdigest()


def png_size(path):
    data = Path(path).read_bytes()[16:24]
    return [int.from_bytes(data[:4], 'big'), int.from_bytes(data[4:], 'big')]


def main():
    shots, evidence, fixture = PACKET / 'screenshots', PACKET / 'evidence', PACKET / 'fixture'
    for directory in (shots, evidence, fixture):
        directory.mkdir(exist_ok=True)
    captures = {}
    log = {Path(m.group(1)).name: json.loads(m.group(3)) | {'window': int(m.group(2)), 'at': m.group(0)[:20]}
           for m in re.finditer(r'\S+ capture (\S+) window (\d+) (\{.*\})', (S / 'images.log').read_text())}
    for png in sorted((S / 'captures').glob('*.png')):
        shutil.copy2(png, shots / png.name)
        captures[png.name] = {'sha256': sha(png), 'px': png_size(png), 'window_frame_pt': log[png.name],
                              'kind': 'actual window-only capture (screencapture -o -x -l), Display P3 tagged'}
    crops = {}
    for png in sorted((S / 'crops').glob('*.png')):
        shutil.copy2(png, shots / png.name)
        crops[png.name] = {'sha256': sha(png), 'px': png_size(png), 'source': png.name.split('-')[0] + '-1-default.png',
                           'kind': 'nearest-neighbour crop of an actual capture (pngcrop.py, colour chunks kept)'}
    analyses = {}
    for name in ('wrap-1-default', 'wrap-2-large', 'narrow-1-default', 'narrow-2-large'):
        target = evidence / f'pixels-{name}.json'
        shutil.copy2(S / 'analysis' / f'{name}-pixels.json', target)
        analyses[target.name] = sha(target)
    for name in ('wrap-1-default', 'narrow-1-default'):
        for kind in ('reference', 'diff'):
            target = evidence / f'{kind}-{name}.png'
            shutil.copy2(S / 'analysis' / f'{name}-{kind}.png', target)
            analyses[target.name] = sha(target)
    shutil.copy2(S / 'checker-mutations.json', evidence / 'checker-mutations.json')
    shutil.copy2(S / 'layout/layout.json', evidence / 'layout.json')
    shutil.copy2(S / 'layout/atlas.pgm', evidence / 'atlas.pgm')
    shutil.copy2(S / 'pkg/package.json', evidence / 'package.json')
    shutil.copy2(S / 'fixture/fixture.json', fixture / 'fixture.json')
    shutil.copy2(S / 'fixture/authoring/project.json', fixture / 'project.json')
    logs = {'probe': text_copy(S / 'logs/probe.log', evidence / 'probe.txt', start='running 1 test')}
    for name in ('test-cpu-text', 'test-render-gpu', 'test-gpu-host', 'test-player-window'):
        logs[name] = text_copy(S / 'logs' / f'{name}.log', evidence / f'{name}.txt')
    for name in ('processes', 'bursts', 'images'):
        logs[name] = text_copy(S / f'{name}.log', evidence / f'{name}.txt')
    reports = {name: json.loads((S / 'logs' / f'player-{name}.log').read_text()) for name in ('wrap', 'narrow')}
    (evidence / 'player-reports.txt').write_text(
        ''.join(f'{name}: {json.dumps(report, sort_keys=True)}\n' for name, report in reports.items()), encoding='utf-8')
    package = json.loads((S / 'pkg/package.json').read_text())
    fixture_record = json.loads((S / 'fixture/fixture.json').read_text())
    fonts = {p.name: p.stat().st_size for p in sorted((S / 'pkg/wrap/player/data/fonts').glob('*.ifont'))}
    identities = {
        'source': {'head': git('rev-parse', 'HEAD'), 'tree': git('rev-parse', 'HEAD^{tree}'),
                   'branch': git('rev-parse', '--abbrev-ref', 'HEAD'),
                   'clean_outside_packet': git('status', '--porcelain', '--', '.',
                                               ':(exclude)handoffs/0055-bidi-wrap-review') == '',
                   'files': {p: sha(ROOT / p) for p in (
                       'Cargo.lock', 'Cargo.toml', 'third_party/cosmic-text/src/shape.rs',
                       'third_party/cosmic-text/INCANT_PATCH.diff', 'crates/incant_text/tests/bidi_wrap.rs',
                       'crates/incant_render/src/text/text.wgsl', 'crates/incant_render/src/output.wgsl')},
                   'vendored_patch_check': 'diff -u of the 4 changed files between ~/.cargo/registry/src/*/'
                                           'cosmic-text-0.14.2 (read only) and third_party/cosmic-text equals '
                                           'INCANT_PATCH.diff (whitespace-normalized); no other source file differs'},
        'host': {'os': 'macOS 26.6 (25G72)', 'machine': 'Apple M5 Pro, Mac17,9', 'gpu_backend': 'Metal',
                 'rustc': 'rustc 1.99.0 (b940084d7 2026-09-28), aarch64-apple-darwin'},
        'binaries': {
            'incant_headless': {'profile': 'ci', 'sha256': sha(S / 'bin/incant_headless'),
                                'use': 'fixture init/import/RPC only (CPU)'},
            'incant_game_player': {'profile': package['profile'], 'features': package['features'],
                                   'sha256': package['player_sha256'], 'bytes': package['player_bytes'],
                                   'signed': False, 'built': package['built_executable'],
                                   'copies': {k: v['player'] for k, v in package['packages'].items()}}},
        'fixture': {'project_sha256': fixture_record['project_sha256'], 'fixture_json_sha256': sha(S / 'fixture/fixture.json'),
                    'journal_sha256': sha(S / 'fixture/authoring/journal.jsonl'),
                    'project_journal_sha256': sha(S / 'fixture/authoring/project.journal.jsonl'),
                    'cases_sha256': sha(HERE / 'cases.json'), 'font_files': fixture_record['font_files'],
                    'packaged_font_payloads': fonts, 'cameras': fixture_record['cameras']},
        'packages': {k: {'camera': v['camera'], 'data_files': v['data']['files'],
                         'startup_text': {x: y for x, y in v['startup']['render_input'].items() if 'text' in x}}
                     for k, v in package['packages'].items()},
        'layout_probe': {'layout_json_sha256': sha(S / 'layout/layout.json'), 'atlas_pgm_sha256': sha(S / 'layout/atlas.pgm'),
                         'native_startup_atlas_sha256': package['packages']['wrap']['startup']['render_input']['text_atlas_sha256'],
                         'probe_atlas_as_2048_plane_sha256': plane_sha(S / 'layout/atlas.pgm'),
                         'recheck_after_rustfmt': 'byte-identical layout.json and atlas.pgm'},
        'captures': captures, 'crops': crops, 'analyses': analyses, 'logs': logs, 'player_reports': reports,
    }
    (evidence / 'identities.json').write_text(json.dumps(identities, indent=1, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps({'captures': len(captures), 'crops': len(crops), 'analyses': len(analyses)}))


if __name__ == '__main__':
    main()
