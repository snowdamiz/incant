#!/usr/bin/env python3
"""Prove route_check.py and layout_check.py detect real defects (CPU only).

Usage: python3 -B mutate_checks.py ACCEPTED LOCALIZED REPORT.json SCRATCH

Copies one passing restart report into SCRATCH/<mutation>/<run>/report.json with one
defect each, then requires the matching checker to fail (exit 1) for every mutation
and to pass for the unmutated copy. Source captures are never modified.
"""
import copy
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
CONTROLS = '01CSWD00000000000000000008'
WON = '01CSWD00000000000000000011'


def first(report, name):
    return next(c for c in report['captures'] if c['name'] == name)


def drawn(capture, owner):
    return next(d for d in capture['drawn'] if d['id'] == owner)


MUTATIONS = {
    'route': {
        'label-text': lambda r: first(r, 'pickup-1')['widgets'][CONTROLS].update(text='WASD move · Space jump'),
        'won-parent': lambda r: first(r, 'won')['widgets'][WON].update(parent='01CSWD00000000000000000010'),
        'log-tick': lambda r: r['generation_logs'][0]['entries'][3].update(tick=665),
        'dropped-log': lambda r: r['generation_logs'][1]['entries'].pop(),
        'pcm': lambda r: r['pcm'].update(sha256='0' * 64),
        'restart-differs': lambda r: drawn(first(r, 'after-start'), CONTROLS).update(source='stale'),
    },
    'layout': {
        'two-lines': lambda r: drawn(first(r, 'start'), CONTROLS)['lines'].append(
            copy.deepcopy(drawn(first(r, 'start'), CONTROLS)['lines'][0])),
        'off-edge': lambda r: drawn(first(r, 'start'), CONTROLS)['bounds'].__setitem__(2, 900.0),
        'missing-glyph': lambda r: drawn(first(r, 'won'), WON).update(missing_clusters=1),
        'overlap': lambda r: drawn(first(r, 'start'), CONTROLS)['bounds'].__setitem__(1, 20.0),
    },
}


def main():
    accepted, localized, source, scratch = sys.argv[1], sys.argv[2], Path(sys.argv[3]), Path(sys.argv[4])
    scratch.mkdir(parents=True, exist_ok=False)
    run = source.parent.name
    original = json.loads(source.read_text(encoding='utf-8'))
    results = {}
    for checker, mutations in [('route', {'none': lambda r: None, **MUTATIONS['route']}),
                               ('layout', {'none': lambda r: None, **MUTATIONS['layout']})]:
        for name, mutate in mutations.items():
            report = copy.deepcopy(original)
            mutate(report)
            target = scratch / f'{checker}-{name}' / run / 'report.json'
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(report), encoding='utf-8')
            argv = ([sys.executable, '-B', HERE / 'route_check.py', accepted, localized, target] if checker == 'route'
                    else [sys.executable, '-B', HERE / 'layout_check.py', target])
            status = subprocess.run(argv, capture_output=True, text=True).returncode
            results[f'{checker}-{name}'] = {'exit': status, 'detected': status == 1, 'expected': name != 'none'}
    ok = all(r['detected'] == r['expected'] for r in results.values())
    print(json.dumps({'passed': ok, 'source_report': str(source), 'results': results}, indent=1))
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
