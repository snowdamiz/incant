#!/usr/bin/env python3
"""Prove pixels.py can fail: feed it known defects and require each to be caught.

Usage: python3 -I -B check_mutations.py CAPTURE.png LAYOUT_JSON ATLAS_PGM SCRATCH_DIR
CAPTURE is the startup-size wrap page. Mutations (scratch copies only):
  lost-glyph     capture with the 'a' of "and" (mixed300) painted as backdrop
  dropped-space  layout as in the 0054 defect: the space before "and" removed
                 and the rest of that line shifted left by its advance
  wrong-break    layout with 'ש' moved from line 2 to the end of line 1
  extra-glyph    layout without the 'W' of "Wrapped": real ink becomes stray
The unmutated inputs must pass the same thresholds.
"""
import copy
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import pixels  # noqa: E402


def run(capture, layout, atlas):
    out = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'pixels.py'), str(capture), '--page', 'wrap',
                          '--layout', str(layout), '--atlas', str(atlas)], capture_output=True, text=True, check=True)
    report = json.loads(out.stdout)
    text = report['texts']['mixed300']
    return {'max_abs': text['max_abs_code_value'], 'over2': text['pixels_over_2'],
            'ratio_min': text['glyph_ink_ratio_min'], 'stray': report['outside_quads']['non_backdrop']}


def passes(r):
    return r['max_abs'] <= 3 and r['stray'] == 0 and r['ratio_min'][0] >= 0.9


def main():
    capture, layout_path, atlas, scratch = map(Path, sys.argv[1:5])
    scratch.mkdir(parents=True, exist_ok=True)
    layout = json.loads(layout_path.read_text(encoding='utf-8'))
    text = next(t for t in layout['texts'] if t['key'] == 'mixed300')
    glyphs = text['glyphs']
    results = {'unmutated': run(capture, layout_path, atlas)}

    def mutated_layout(name, edit):
        data = copy.deepcopy(layout)
        edit(next(t for t in data['texts'] if t['key'] == 'mixed300'))
        path = scratch / f'{name}.json'
        path.write_text(json.dumps(data, ensure_ascii=False), encoding='utf-8')
        results[name] = run(capture, path, atlas)

    # lost-glyph: paint the padded quad of the 'a' in "and" with the backdrop colour.
    a = next(i for i, g in enumerate(glyphs) if g['text'] == 'a' and g['y'] > 40)
    g = glyphs[a]
    width, height, bpp, rows = pixels.read_png(capture)
    left, top = 24 + g['x'] + g['origin'][0] - 1, 64 + 24 + g['y'] + g['origin'][1] - 1
    box = (int(left), int(top), int(left + g['rect'][2] + 3), int(top + g['rect'][3] + 3))
    rows = [bytearray(r) for r in rows]
    for y in range(box[1], box[3]):
        for x in range(box[0], box[2]):
            rows[y][x * bpp:x * bpp + 3] = bytes(pixels.BACKDROP)
    lost = scratch / 'lost-glyph.png'
    pixels.write_png(lost, width, height, lambda x, y: bytes(rows[y][x * bpp:(x + 1) * bpp]), 6 if bpp == 4 else 2)
    results['lost-glyph'] = run(lost, layout_path, atlas)

    def dropped_space(t):
        line = t['lines'][1]
        g0, g1 = line['glyphs']
        space = next(i for i in range(g0, g1) if t['glyphs'][i]['text'] == ' ')
        advance = t['glyphs'][space]['advance']
        for i in range(space + 1, g1):
            t['glyphs'][i]['x'] -= advance
        del t['glyphs'][space]
        line['glyphs'][1] -= 1
        for later in t['lines'][2:]:
            later['glyphs'] = [later['glyphs'][0] - 1, later['glyphs'][1] - 1]
    mutated_layout('dropped-space', dropped_space)

    def wrong_break(t):
        first, second = t['lines'][0], t['lines'][1]
        shin = next(i for i in range(*second['glyphs']) if t['glyphs'][i]['text'] == 'ש')
        g = t['glyphs'][shin]
        g['x'] = first['width']
        g['y'] = first['baseline']
    mutated_layout('wrong-break', wrong_break)

    def extra_glyph(t):
        w = next(i for i, g in enumerate(t['glyphs']) if g['text'] == 'W')
        t['glyphs'][w]['rect'] = None
    mutated_layout('extra-glyph', extra_glyph)

    verdict = {name: ('pass' if passes(r) else 'caught') for name, r in results.items()}
    ok = verdict['unmutated'] == 'pass' and all(v == 'caught' for k, v in verdict.items() if k != 'unmutated')
    print(json.dumps({'results': results, 'verdict': verdict, 'checker_discriminates': ok},
                     indent=1, ensure_ascii=False))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
