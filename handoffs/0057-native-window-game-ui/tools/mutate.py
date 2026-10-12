#!/usr/bin/env python3
"""One deliberate renderer mistake at a time against the UI draw tests (0057).

Usage: python3 -I -B mutate.py OUT_JSON

Each mutation replaces one unique anchor, runs the UI draw tests (GPU tests
included, ci profile, -j2, the leased target), then restores the exact bytes
and verifies the file hash before the next mutation. The unmutated suite runs
first and last. Commit or keep a copy of sources before running; never edit
sources while it runs. Refuses without the 0057 lease or after its release.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path.home() / '.codex/incant-evidence/0057'
TARGET = Path.home() / '.codex/incant-native-target'
SHADER = 'crates/incant_render/src/ui/ui.wgsl'
GEOMETRY = 'crates/incant_render/src/ui/geometry.rs'
MUTATIONS = [
    ('glyph pen one physical pixel right', SHADER,
     'let pen = snap(physical(quad.pen));', 'let pen = snap(physical(quad.pen)) + vec2f(1.0, 0.0);'),
    ('quarter-texel sampling shift', SHADER,
     'let at = clamp(input.texel + dx * offset.x + dy * offset.y, low, high);',
     'let at = clamp(input.texel + dx * offset.x + dy * offset.y + vec2f(0.25, 0.0), low, high);'),
    ('no clip intersection', SHADER,
     'let low = max(p0, c0);\n    let high = max(min(p1, c1), low);',
     'let low = p0;\n    let high = max(p1, low);'),
    ('90 % opacity', SHADER,
     'let alpha = input.color.a * coverage;', 'let alpha = 0.9 * input.color.a * coverage;'),
    ('straight instead of premultiplied output', SHADER,
     'return vec4f(color * alpha, alpha);', 'return vec4f(color, alpha);'),
    ('no sRGB decode on sRGB targets', SHADER,
     'if frame.encode.x == 0.0 {\n        color = decode(color);\n    }', 'if frame.encode.x == 2.0 {\n        color = decode(color);\n    }'),
    ('label shadow without its offset', GEOMETRY,
     'glyphs(atlas, widget, shadow, theme::LABEL_SHADOW_OFFSET, out)?;', 'glyphs(atlas, widget, shadow, [0.; 2], out)?;'),
    ('caret one logical pixel right', GEOMETRY,
     'let half = theme::CARET_WIDTH * 0.5;\n    let rect = translate(\n        [caret.x - half, caret.top, caret.x + half, caret.bottom],',
     'let half = theme::CARET_WIDTH * 0.5;\n    let rect = translate(\n        [caret.x + 1. - half, caret.top, caret.x + 1. + half, caret.bottom],'),
]


def leased():
    lease = json.loads((EVIDENCE / 'lease.json').read_text(encoding='utf-8'))
    if lease.get('packet') != '0057-native-window-game-ui' or (EVIDENCE / 'released').exists():
        sys.exit('no active 0057 lease; refusing')


def run_tests(log):
    env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET))
    command = [str(ROOT / 'tools/cargo'), 'test', '--locked', '--offline', '-j2', '--profile', 'ci',
               '-p', 'incant_render', '--lib', '--', '--include-ignored', '--test-threads=1', 'ui::']
    with open(log, 'w', encoding='utf-8') as out:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=out, stderr=subprocess.STDOUT, timeout=3600)
    text = Path(log).read_text(encoding='utf-8')
    failed = sorted({line.split()[1] for line in text.splitlines()
                     if line.startswith('test ') and line.rstrip().endswith('FAILED')})
    return result.returncode, failed


def main():
    leased()
    out = Path(sys.argv[1])
    logs = EVIDENCE / 'logs' / 'mutations'
    logs.mkdir(parents=True, exist_ok=True)
    records = []
    code, failed = run_tests(logs / 'unmutated-before.txt')
    records.append({'mutation': 'none (before)', 'exit': code, 'failed': failed})
    for index, (name, path, old, new) in enumerate(MUTATIONS):
        file = ROOT / path
        original = file.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode('utf-8')
        if text.count(old) != 1:
            sys.exit(f'anchor for {name!r} is not unique in {path}')
        try:
            file.write_text(text.replace(old, new), encoding='utf-8')
            code, failed = run_tests(logs / f'mutation-{index}.txt')
        finally:
            file.write_bytes(original)
        if hashlib.sha256(file.read_bytes()).hexdigest() != digest:
            sys.exit(f'restore of {path} failed')
        records.append({'mutation': name, 'file': path, 'exit': code, 'failed': failed, 'caught': code != 0})
        print(name, 'caught' if code else 'NOT CAUGHT', failed, flush=True)
    code, failed = run_tests(logs / 'unmutated-after.txt')
    records.append({'mutation': 'none (after)', 'exit': code, 'failed': failed})
    out.write_text(json.dumps(records, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
