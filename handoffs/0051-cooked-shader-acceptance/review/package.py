#!/usr/bin/env python3
"""Copy the retained 0051 evidence into the packet and write its hash manifest.

CPU-only. Copies are byte-for-byte and create-new. Captures are retained from the
default side only: every cooked file is byte-identical (verified here), and both
sides stay in the ignored artifacts folder. Derived inspection crops stay ignored.
`--check` rebuilds the manifest without writing and requires it, and every
retained copy, to match what is already in the packet.
"""
import importlib.util
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PACKET = HERE.parent
EVIDENCE = PACKET / 'evidence'
OUTPUT = ROOT / 'artifacts/0051-cooked-shader-acceptance'
TOOLS = ROOT / 'artifacts/tools/0051'
spec = importlib.util.spec_from_file_location('analyze_0051', HERE / 'analyze.py')
analyze = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analyze)
NAMES = [f'{case}-{mode}' for case in analyze.CASES for mode in ('default', 'cooked')]


def copy(source, target, write):
    raw = source.read_bytes()
    # libtest ends with a blank line, which `git diff --check` rejects; drop only that.
    data = raw.rstrip(b'\n') + b'\n' if target.name.endswith('.child-log.txt') else raw
    if write:
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as out:
            out.write(data)
    else:
        assert target.read_bytes() == data, f'{target} differs from {source}'
    record = {'source': str(source.relative_to(ROOT)), 'sha256': analyze.sha(data)}
    if data != raw:
        record.update(raw_sha256=analyze.sha(raw), normalization='trailing blank line removed')
    return record


def build(write):
    plan = [(OUTPUT / 'children.json', EVIDENCE / 'helper/children.json'),
            (TOOLS / 'RELEASED.json', EVIDENCE / 'helper/RELEASED.json'),
            (OUTPUT / 'review/analysis.json', EVIDENCE / 'analysis.json')]
    # The repository ignores *.log, so retained log copies use a .txt suffix.
    plan += [(OUTPUT / name / 'child.log', EVIDENCE / f'helper/logs/{name}.child-log.txt') for name in NAMES]
    plan += [(OUTPUT / f'preflight/{name}.json', EVIDENCE / f'preflight/{name}.json') for name in NAMES]
    captures = []
    for case in analyze.CASES:
        for path in sorted((OUTPUT / f'{case}-default').glob('*.png')):
            cooked = OUTPUT / f'{case}-cooked' / path.name
            assert path.read_bytes() == cooked.read_bytes(), f'{path.name} differs between sides'
            plan.append((path, EVIDENCE / f'captures/{path.name}'))
            for mode, side in (('default', path), ('cooked', cooked)):
                data = side.read_bytes()
                width, height, rgba, _ = analyze.decode_png(data)
                captures.append({'case': case, 'mode': mode, 'file': path.name,
                                 'artifact': str(side.relative_to(ROOT)), 'width': width, 'height': height,
                                 'file_sha256': analyze.sha(data), 'rgba_sha256': analyze.sha(rgba),
                                 'retained_copy': f'evidence/captures/{path.name}' if mode == 'default' else None})
    copies = {str(target.relative_to(PACKET)): copy(source, target, write) for source, target in plan}
    derived = json.loads((OUTPUT / 'review/derived/derived-manifest.json').read_text())
    return {
        'scope': '0051 Claude appearance review: 8 helper children, 24 captures, Metal only',
        'captures': captures,
        'copies': copies,
        'derived_inspection_images_ignored': {
            'folder': 'artifacts/0051-cooked-shader-acceptance/review/derived',
            'note': 'CPU-derived nearest-neighbour crops and difference maps of the captures; '
                    'not renders, not committed, regenerable with review/views.py',
            'files': {name: {'sha256': info['sha256'], 'source': info['source']} for name, info in derived.items()},
        },
        'root_staged_files_cited_not_copied': {
            name: analyze.sha((TOOLS / name).read_bytes())
            for name in ('acp-settings-confirmation.log', 'coordination-note.md')},
    }


def main():
    check = sys.argv[1:] == ['--check']
    text = json.dumps(build(write=not check), indent=2) + '\n'
    if check:
        assert (EVIDENCE / 'manifest.json').read_text() == text, 'evidence manifest differs'
        print('evidence manifest and all retained copies reproduced exactly')
        return
    with (EVIDENCE / 'manifest.json').open('x') as out:
        out.write(text)
    print(text)


if __name__ == '__main__':
    main()
