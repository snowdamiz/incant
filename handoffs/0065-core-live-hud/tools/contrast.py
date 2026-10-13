#!/usr/bin/env python3
"""Estimate HUD label contrast in retained 0065 frames (CPU only).

Usage: python3 -B contrast.py REPORT.json [...]

For each visible label in each retained frame: WCAG contrast of the fixed theme text
colour (236, 238, 242) against the median relative luminance of every pixel in the
label's border box, plus the 90th-percentile box luminance as a bright-patch
indicator. Glyphs cover a minority of each box, so the median approximates the local
scene; this is an estimate for comparing placements, not a conformance measurement.
"""
import json
from pathlib import Path
import sys

TEXT = (236, 238, 242)
NAMES = {'04': 'health', '05': 'objective', '07': 'samples', '08': 'controls', '11': 'won', '12': 'lost'}


def linear(c):
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def luminance(rgb):
    r, g, b = (linear(v) for v in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    hi, lo = max(a, b), min(a, b)
    return (hi + 0.05) / (lo + 0.05)


def main():
    text = luminance(TEXT)
    for path in map(Path, sys.argv[1:]):
        report = json.loads(path.read_text(encoding='utf-8'))
        width, height = report['size']
        for capture in report['captures']:
            if 'rgba' not in capture:
                continue
            rgba = (path.parent / capture['rgba']).read_bytes()
            row = {'frame': f'{path.parent.name}/{capture["rgba"]}', 'labels': {}}
            for d in capture['drawn']:
                name = NAMES.get(d['id'][-2:])
                left, top, right, bottom = (round(v) for v in d['visible'])
                if name is None or right <= left or bottom <= top:
                    continue
                values = sorted(luminance(rgba[(y * width + x) * 4:(y * width + x) * 4 + 3])
                                for y in range(max(top, 0), min(bottom, height))
                                for x in range(max(left, 0), min(right, width)))
                median, p90 = values[len(values) // 2], values[int(len(values) * 0.9)]
                row['labels'][name] = {'box': [left, top, right, bottom], 'median_contrast': round(ratio(text, median), 2),
                                       'p90_box_luminance': round(p90, 3)}
            print(json.dumps(row))


if __name__ == '__main__':
    main()
