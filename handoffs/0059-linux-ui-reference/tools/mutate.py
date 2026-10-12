#!/usr/bin/env python3
"""One deliberate mistake at a time against the UI draw tests (0059).

Usage: python3 -I -B mutate.py OUT_JSON

Each mutation replaces one unique anchor, runs the UI draw tests (GPU tests
included) through tools/cargo.sh, then restores the exact bytes and verifies
the file hash before the next one. The unmutated suite runs first and last.
Commit or keep a copy of sources first; never edit sources while it runs.
The runner refuses without the 0059 lease or after its release.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
RUNNER = ROOT / 'handoffs/0059-linux-ui-reference/tools/cargo.sh'
LOGS = Path.home() / '.codex/incant-evidence/0059/logs'
SHADER = 'crates/incant_render/src/ui/ui.wgsl'
TESTS = 'crates/incant_render/src/ui_draw_tests.rs'
UNORM = 'frame.encode.x == 1.0'
MUTATIONS = [
    ('unorm-linear', 'UNORM targets blend decoded (linear-light) colours', SHADER,
     'if frame.encode.x == 0.0 {', 'if frame.encode.x >= 0.0 {'),
    ('unorm-colour-98', 'UNORM colour 2 % dark', SHADER,
     'var color = input.color.rgb;', f'var color = input.color.rgb * select(1.0, 0.98, {UNORM});'),
    ('unorm-colour-99', 'UNORM colour 1 % dark', SHADER,
     'var color = input.color.rgb;', f'var color = input.color.rgb * select(1.0, 0.99, {UNORM});'),
    ('unorm-opacity-97', 'UNORM opacity 97 %', SHADER,
     'let alpha = input.color.a * coverage;', f'let alpha = input.color.a * coverage * select(1.0, 0.97, {UNORM});'),
    ('pen-right', 'glyph pen one physical pixel right', SHADER,
     'let pen = snap(physical(quad.pen));', 'let pen = snap(physical(quad.pen)) + vec2f(1.0, 0.0);'),
    ('no-clip', 'no clip intersection', SHADER,
     'let low = max(p0, c0);\n    let high = max(min(p1, c1), low);', 'let low = p0;\n    let high = max(p1, low);'),
    ('reference-float-only', 'reference without the UNORM fixed-point admission (checkpoint 12)', TESTS,
     'pixel[c] = if model == Model::Portable && !srgb {', 'pixel[c] = if false && !srgb {'),
]
ARGS = ['test', '-p', 'incant_render', '--profile', 'ci', '--locked', '-j2', '--lib', 'ui::draw_tests',
        '--', '--include-ignored', '--test-threads=1', '--nocapture']


def run(label):
    code = subprocess.run([str(RUNNER), label, *ARGS], cwd=ROOT).returncode
    text = (LOGS / f'{label}.log').read_text(encoding='utf-8')
    failed = sorted({line.split()[1] for line in text.splitlines()
                     if line.startswith('test ') and line.rstrip().endswith('FAILED')})
    panics = [line.strip() for line in text.splitlines() if re.match(r'\s*assertion|.*rejected by|^scale 1 Rgba8Unorm', line)]
    return code, failed, panics[:12]


def main():
    out = Path(sys.argv[1])
    records = []
    code, failed, _ = run('mutation-0-unmutated-before')
    records.append({'mutation': 'none (before)', 'exit': code, 'failed': failed})
    if code != 0:
        sys.exit('unmutated suite fails; refusing to mutate')
    for index, (slug, name, path, old, new) in enumerate(MUTATIONS, start=1):
        file = ROOT / path
        original = file.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode('utf-8')
        if text.count(old) != 1:
            sys.exit(f'anchor for {name!r} is not unique in {path}')
        try:
            file.write_text(text.replace(old, new), encoding='utf-8')
            code, failed, lines = run(f'mutation-{index}-{slug}')
        finally:
            file.write_bytes(original)
        assert hashlib.sha256(file.read_bytes()).hexdigest() == digest, f'{path} not restored'
        records.append({'mutation': name, 'file': path, 'anchor': old, 'replacement': new,
                        'exit': code, 'failed': failed, 'caught': code != 0, 'lines': lines,
                        'restored_sha256': digest})
        print(f'{name}: exit {code}, failed {failed}', flush=True)
    code, failed, _ = run('mutation-9-unmutated-after')
    records.append({'mutation': 'none (after)', 'exit': code, 'failed': failed})
    out.write_text(json.dumps(records, indent=1) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
