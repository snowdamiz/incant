#!/usr/bin/env python3
"""Pre-render contrast estimates for candidate result-state HUD placements (CPU only).

  placement.py WON_PNG LOST_PNG

WON_PNG/LOST_PNG are retained 960x540 0065 frames without a result title at the win
and loss camera positions (07 locked exit at the lift; 01 start, where the idle route
loses). The camera has a fixed vertical field of view, so a W x H viewport is the
centred (W*540/H) x 540 crop of those frames (checked with frame_profile.py match).

Each label is reduced to its Latin ink band (cap height to baseline, from Inter's
hhea metrics) and its estimated width, then mapped into the wide frame. Reports the
contrast of the fixed theme text against the median and 90th-percentile-brightest
background luminance. Planning estimates only: no frame is rendered, and any final
verdict comes from the granted capture of the real layout.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from frame_profile import decode, box  # noqa: E402

ASCENT, DESCENT, CAP = 0.96875, 0.2421875, 0.727
SIZES = {'960x540': (960, 540), '640x480': (640, 480), '420x320': (420, 320)}
# Measured 0065 widths (en) and text_width.py estimates for new strings.
WIDTH = {'pips': 72.8, 'objective': 198.8, 'samples': 130.8, 'legend': 146.8,
         'won32': 260.0, 'lost32': 227.0, 'prompt17': 233.2}


def ink(top, size, line):
    baseline = top + (line - (ASCENT + DESCENT) * size) / 2 + ASCENT * size
    return baseline - CAP * size, baseline


def layouts(width):
    """{variant: {state: [(label, x0, x1, top, size, line)]}} in viewport pixels."""
    right = width - 22
    stats = lambda top: [('pips', 22, 22 + WIDTH['pips'], top, 22, 28),
                         ('samples', right - WIDTH['samples'], right, top, 22, 28),
                         ('objective', 22, 22 + WIDTH['objective'], top + 30, 17, 22),
                         ('legend', right - WIDTH['legend'], right, top + 30, 12, 16)]
    centre = lambda key, top, size, line: (key, width / 2 - WIDTH[key] / 2, width / 2 + WIDTH[key] / 2,
                                           top, size, line)
    out = {}
    # The 0065 layout is not re-estimated here; its own measured figures stand.
    # play: the playing top bar, unchanged at 16 with the shorter legend. Boxes that
    # overlap the frames' own 0065 top-bar text are contaminated (p90 near 1).
    out['play'] = {'won': stats(16), 'lost': stats(16)}
    # A (prepared hud.json): banner above the top bar (root column_reverse, padding
    # top 8; Result padding bottom 8); title 32/40 at 8, prompt 17/22 at 48, bar at 78.
    out['A'] = {s: [centre(s + '32', 8, 32, 40), centre('prompt17', 48, 17, 22)] + stats(78)
                for s in ('won', 'lost')}
    # B (rejected): bar stays; title and prompt directly beneath it (6 px).
    out['B'] = {s: stats(16) + [centre(s + '32', 74, 32, 40), centre('prompt17', 114, 17, 22)]
                for s in ('won', 'lost')}
    return out


def main():
    frames = {'won': decode(sys.argv[1]), 'lost': decode(sys.argv[2])}
    print('variant viewport state label: median p90-bright')
    for name, (width, height) in SIZES.items():
        scale, offset = 540 / height, 480 - width / 2 * 540 / height
        for variant, states in layouts(width).items():
            for state, labels in states.items():
                _, _, rows = frames[state]
                cells = []
                for label, x0, x1, top, size, line in labels:
                    if label.startswith('objective') and state == 'won' and variant != 'play':
                        continue  # the reviewed controller clears the objective on a win
                    y0, y1 = ink(top, size, line)
                    est = box(rows, round(offset + x0 * scale), round(y0 * scale),
                              round(offset + x1 * scale), round(y1 * scale))
                    cells.append(f"{label.rstrip('0123456789')} {est['median']}/{est['p90']}")
                print(f'{variant:4} {name} {state:4} ' + ' | '.join(cells))


if __name__ == '__main__':
    main()
