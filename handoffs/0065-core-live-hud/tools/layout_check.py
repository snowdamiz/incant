#!/usr/bin/env python3
"""Summarize and check recorded HUD geometry from 0065 adapter reports (CPU only).

Usage: python3 -B layout_check.py REPORT.json [...]

Checks every capture's drawn labels: each visible label is one line, nothing is
missing a glyph, visible labels stay inside the viewport padding and do not overlap,
status and progress keep their start/end edges (mirrored for RTL), and when the
progress column sits on its own row it is below the status column. Exits 1 on failure.
"""
import json
from pathlib import Path
import sys

W = {'01CSWD00000000000000000004': 'health', '01CSWD00000000000000000005': 'objective',
     '01CSWD00000000000000000007': 'samples', '01CSWD00000000000000000008': 'controls',
     '01CSWD00000000000000000011': 'won', '01CSWD00000000000000000012': 'lost'}
PAD = [22, 16, 22, 16]  # ui/hud.json root padding: left, top, right, bottom


def check(capture, rtl):
    view = capture['viewport']
    labels = {W[d['id']]: d for d in capture['drawn'] if d['id'] in W}
    visible = {k: d for k, d in labels.items() if d['visible'][2] > d['visible'][0]}
    failures = []
    for name, d in visible.items():
        b = d['bounds']
        if len(d['lines']) != 1:
            failures.append(f'{name} has {len(d["lines"])} lines')
        if d['missing_clusters']:
            failures.append(f'{name} misses glyphs')
        if b[0] < PAD[0] - 0.01 or b[2] > view[0] - PAD[2] + 0.01 or b[1] < PAD[1] - 0.01:
            failures.append(f'{name} leaves the padded viewport')
    names = sorted(visible)
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            p, q = visible[a]['bounds'], visible[b]['bounds']
            if p[0] < q[2] and q[0] < p[2] and p[1] < q[3] and q[1] < p[3]:
                failures.append(f'{a} overlaps {b}')
    start, end = (view[0] - PAD[2], PAD[0]) if rtl else (PAD[0], view[0] - PAD[2])
    status_edge = 2 if rtl else 0
    progress_edge = 0 if rtl else 2
    for name in ('health', 'objective'):
        if name in visible and abs(visible[name]['bounds'][status_edge] - start) > 0.01:
            failures.append(f'{name} off the start edge')
    for name in ('samples', 'controls'):
        if name in visible and abs(visible[name]['bounds'][progress_edge] - end) > 0.01:
            failures.append(f'{name} off the end edge')
    stacked = visible['samples']['bounds'][1] >= visible['objective']['bounds'][3] - 0.01
    title = next((k for k in ('won', 'lost') if k in visible), None)
    return {'host_step': capture['host_step'], 'name': capture['name'], 'generation': capture['generation'],
            'simulation_tick': capture['simulation_tick'], 'viewport': view, 'stacked': stacked, 'title': title,
            'title_box': visible[title]['bounds'] if title else None,
            'widths': {k: round(d['bounds'][2] - d['bounds'][0], 1) for k, d in visible.items()},
            'texts': {k: d['source'] for k, d in visible.items()},
            'missing_glyphs': capture['ui']['missing_glyphs'], 'failures': failures}


def main():
    ok = True
    for path in sys.argv[1:]:
        report = json.loads(Path(path).read_text(encoding='utf-8'))
        for capture in report['captures']:
            # RTL presentation mirrors the top bar; the controls paragraph resolves RTL.
            rtl = any(d['id'] == '01CSWD00000000000000000008' and d['lines'] and d['lines'][0]['rtl']
                      for d in capture['drawn'])
            row = check(capture, rtl)
            ok &= not row['failures'] and row['missing_glyphs'] == 0
            print(json.dumps({'report': Path(path).parent.name, **row}, ensure_ascii=False))
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
