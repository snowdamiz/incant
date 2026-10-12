#!/usr/bin/env python3
"""Independent CPU reference for a 0055 window capture, and the comparison.

Usage: python3 -I -B pixels.py CAPTURE.png --page wrap|narrow --layout layout.json
       --atlas atlas.pgm [--out DIR] [--titlebar PX]

The reference uses only the CPU layout probe (layout.json, atlas.pgm), cases.json
and the published draw contract, never renderer code:
- orthographic camera, vertical_size 4.8: pixels per unit = content height / 4.8;
- 100 layout px per local unit; layout top-left at the text entity origin;
- each glyph's padded quad starts at position + origin - 1 and is (w+2, h+2);
- one bilinear atlas tap at the pixel centre (footprint <= 1.25 texel here),
  clamped to the padded rect's outer texel centres, as in text.wgsl;
- premultiplied white "over", then display = srgb(mix(backdrop, 0.9, alpha)),
  the output pass for white text over the display backdrop (20, 21, 25).
It reports per-text pixel agreement, stray ink outside every glyph quad, per
glyph captured/expected ink (a lost cluster reads ~0), captured ink extents per
line against the declared width, and ink in each interior space's core.
Guide bars are masked (they are lit meshes, not text). Rounded window corners
(transparent capture pixels) are ignored.
"""
import argparse
import json
import math
from pathlib import Path
import struct
import sys
import zlib

HERE = Path(__file__).resolve().parent
CASES = json.loads((HERE / 'cases.json').read_text(encoding='utf-8'))
BACKDROP = (20, 21, 25)


def decode(v):
    c = v / 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def encode(c):
    c = min(max(c, 0.0), 1.0)
    s = 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055
    return int(math.floor(s * 255 + 0.5))


LINEAR = [decode(v) for v in range(256)]
BG = [LINEAR[v] for v in BACKDROP]
WHITE = 0.9  # tone_map(1.0) for unlit white text radiance


def read_png(path):
    data = Path(path).read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'not a PNG'
    at, idat = 8, b''
    while at < len(data):
        size = struct.unpack('>I', data[at:at + 4])[0]
        kind, body = data[at + 4:at + 8], data[at + 8:at + 8 + size]
        at += 12 + size
        if kind == b'IHDR':
            width, height, depth, color, _, _, interlace = struct.unpack('>IIBBBBB', body)
            assert depth == 8 and color in (2, 6) and not interlace, 'needs 8-bit RGB/RGBA'
        elif kind == b'IDAT':
            idat += body
    bpp = 4 if color == 6 else 3
    raw, stride, rows, prev, at = zlib.decompress(idat), width * bpp, [], bytearray(width * bpp), 0
    for _ in range(height):
        kind, line = raw[at], bytearray(raw[at + 1:at + 1 + stride])
        at += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b, c = prev[x], prev[x - bpp] if x >= bpp else 0
            if kind == 1:
                line[x] = (line[x] + a) & 255
            elif kind == 2:
                line[x] = (line[x] + b) & 255
            elif kind == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return width, height, bpp, rows


def write_png(path, width, height, pixel, color=2):
    """8-bit RGB (color 2) or RGBA (color 6); pixel(x, y) returns 3 or 4 bytes."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(pixel(x, y))

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
    Path(path).write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, color, 0, 0, 0))
                           + chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + chunk(b'IEND', b''))


def read_pgm(path):
    data = Path(path).read_bytes()
    magic, size, depth, pixels = data.split(b'\n', 3)
    width, height = map(int, size.split())
    assert magic == b'P5' and depth == b'255' and len(pixels) == width * height
    return width, height, pixels


def titlebar(width, rows, bpp):
    """First row where both content edges show the display backdrop for 8 rows."""
    def backdrop(y, x):
        p = rows[y][x * bpp:x * bpp + 3]
        return all(abs(p[i] - BACKDROP[i]) <= 2 for i in range(3))
    for y in range(len(rows) - 8):
        if all(backdrop(y + j, 6) and backdrop(y + j, width - 7) for j in range(8)):
            return y
    raise ValueError('no content backdrop found')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture')
    parser.add_argument('--page', required=True, choices=sorted(CASES['pages']))
    parser.add_argument('--layout', required=True)
    parser.add_argument('--atlas', required=True)
    parser.add_argument('--out')
    parser.add_argument('--titlebar', type=int)
    args = parser.parse_args()
    cap_w, cap_h, bpp, rows = read_png(args.capture)
    top = args.titlebar if args.titlebar is not None else titlebar(cap_w, rows, bpp)
    W, H = cap_w, cap_h - top
    ppu = H / CASES['vertical_size']
    k = ppu / 100
    cam_x, cam_y, _ = CASES['pages'][args.page]['camera']
    atlas_w, _, atlas = read_pgm(args.atlas)
    # A clamped tap at a padded rect's last texel centre has zero weight on the next texel,
    # which can lie one row past the probe's cropped extent; keep that read in bounds.
    atlas = atlas + bytes(atlas_w + 1)
    layout = {t['key']: t for t in json.loads(Path(args.layout).read_text(encoding='utf-8'))['texts']}
    unit = CASES['frame'][1] / CASES['vertical_size']
    alpha = [0.0] * (W * H)       # composite expected coverage
    total = [0.0] * (W * H)       # sum of glyph contributions (for attribution)
    guide = bytearray(W * H)
    owner = {}                    # pixel -> text key whose quads cover it
    contributions = {}            # (key, glyph) -> [(pixel, a)]
    maps = {}

    for case in CASES['texts']:
        if case['page'] != args.page:
            continue
        key = case['key']
        left, ttop = case['at']
        gx = cam_x + (left - CASES['frame'][0] / 2) / unit
        gy = cam_y + (CASES['frame'][1] / 2 - ttop) / unit
        theta = math.radians(case.get('rotate_degrees', 0))
        s = case.get('scale', 1) * k
        c, n = math.cos(theta), math.sin(theta)
        ox, oy = W / 2 + (gx - cam_x) * ppu, H / 2 - (gy - cam_y) * ppu
        # frame = O + s * [[c, n], [-n, c]] (X, Y); inverse is the transpose / s.
        to_frame = lambda X, Y, ox=ox, oy=oy, s=s, c=c, n=n: (ox + s * (c * X + n * Y), oy + s * (-n * X + c * Y))
        to_layout = lambda fx, fy, ox=ox, oy=oy, s=s, c=c, n=n: (
            (c * (fx - ox) - n * (fy - oy)) / s, (n * (fx - ox) + c * (fy - oy)) / s)
        footprint = 1 / s
        assert math.ceil(footprint - 0.25) <= 1, f'{key}: multi-tap footprint {footprint}'
        maps[key] = (to_frame, to_layout)

        def pixels_of(rect):
            l, t, r, b = rect
            corners = [to_frame(x, y) for x in (l, r) for y in (t, b)]
            x0 = max(0, math.floor(min(p[0] for p in corners)) - 1)
            x1 = min(W, math.ceil(max(p[0] for p in corners)) + 1)
            y0 = max(0, math.floor(min(p[1] for p in corners)) - 1)
            y1 = min(H, math.ceil(max(p[1] for p in corners)) + 1)
            for py in range(y0, y1):
                for px in range(x0, x1):
                    X, Y = to_layout(px + 0.5, py + 0.5)
                    yield px, py, X, Y

        text = layout[key]
        for gi, g in enumerate(text['glyphs']):
            if g['rect'] is None:
                continue
            ax, ay, gw, gh = g['rect']
            l, t = g['x'] + g['origin'][0] - 1, g['y'] + g['origin'][1] - 1
            r, b = l + gw + 2, t + gh + 2
            u_lo, u_hi, v_lo, v_hi = ax - 0.5, ax + gw + 0.5, ay - 0.5, ay + gh + 0.5
            out = []
            for px, py, X, Y in pixels_of((l, t, r, b)):
                if not (l <= X < r and t <= Y < b):
                    continue
                u = min(max(ax - 1 + (X - l), u_lo), u_hi) - 0.5
                v = min(max(ay - 1 + (Y - t), v_lo), v_hi) - 0.5
                x0, y0 = math.floor(u), math.floor(v)
                fu, fv = u - x0, v - y0
                at = y0 * atlas_w + x0
                a = ((atlas[at] * (1 - fu) + atlas[at + 1] * fu) * (1 - fv)
                     + (atlas[at + atlas_w] * (1 - fu) + atlas[at + atlas_w + 1] * fu) * fv) / 255
                i = py * W + px
                owner.setdefault(i, key)
                if a > 0:
                    alpha[i] = 1 - (1 - alpha[i]) * (1 - a)
                    total[i] += a
                    out.append((i, a))
            contributions[(key, gi)] = out
        if case['guides']:
            height = text['size'][1]
            for x0g in (-3, case['width'] + 1):
                l, t, r, b = x0g - 1, -1, x0g + 3, height + 1   # 1 px safety margin
                for px, py, X, Y in pixels_of((l, t, r, b)):
                    if l <= X < r and t <= Y < b:
                        guide[py * W + px] = 1

    def captured(i):
        px, py = i % W, i // W
        row = rows[py + top]
        o = px * bpp
        if bpp == 4 and row[o + 3] != 255:
            return None
        return row[o:o + 3]

    def coverage(p):
        return sum((LINEAR[p[ch]] - BG[ch]) / (WHITE - BG[ch]) for ch in range(3)) / 3

    expected = lambda a: [encode((1 - a) * BG[ch] + a * WHITE) for ch in range(3)]
    report = {'capture': str(args.capture), 'page': args.page, 'capture_size': [cap_w, cap_h],
              'titlebar_px': top, 'content_px': [W, H], 'layout_px_per_frame_px': 1 / k,
              'pixels_per_unit': ppu, 'texts': {}}
    stats = {}
    for i, key in owner.items():
        if guide[i]:
            continue
        p = captured(i)
        if p is None:
            continue
        e = expected(alpha[i])
        d = max(abs(p[ch] - e[ch]) for ch in range(3))
        st = stats.setdefault(key, {'pixels': 0, 'max_abs': 0, 'over2': 0, 'sum_abs': 0})
        st['pixels'] += 1
        st['max_abs'] = max(st['max_abs'], d)
        st['over2'] += d > 2
        st['sum_abs'] += d
    stray, stray_box, checked = 0, None, 0
    for i in range(W * H):
        if i in owner or guide[i]:
            continue
        p = captured(i)
        if p is None:
            continue
        checked += 1
        if max(abs(p[ch] - BACKDROP[ch]) for ch in range(3)) > 2:
            stray += 1
            px, py = i % W, i // W
            stray_box = [px, py, px, py] if stray_box is None else [
                min(stray_box[0], px), min(stray_box[1], py), max(stray_box[2], px), max(stray_box[3], py)]
    report['outside_quads'] = {'pixels_checked': checked, 'non_backdrop': stray, 'bbox': stray_box}

    for case in CASES['texts']:
        key = case['key']
        if case['page'] != args.page:
            continue
        text, (to_frame, to_layout) = layout[key], maps[key]
        st = stats.get(key, {'pixels': 0, 'max_abs': 0, 'over2': 0, 'sum_abs': 0})
        glyph_ratio = []
        for gi, g in enumerate(text['glyphs']):
            parts = contributions.get((key, gi))
            if not parts:
                continue
            # Captured and reference composite coverage are attributed to overlapping glyphs
            # (joins, kerning) by the same shares, so a present glyph reads 1 and a lost one ~0.
            want, got, own = 0.0, 0.0, sum(a for _, a in parts)
            for i, a in parts:
                p = captured(i)
                if p is not None and not guide[i]:
                    got += coverage(p) * a / total[i]
                    want += alpha[i] * a / total[i]
            if own > 1.0 and want > 0.5:
                glyph_ratio.append((round(got / want, 3), g['text'], gi))
        lines = []
        for li, line in enumerate(text['lines']):
            band = (line['top'] + 4, line['top'] + line['height'] - 4)
            xs = []
            g0, g1 = line['glyphs']
            for gi in range(g0, g1):
                for i, a in contributions.get((key, gi), []):
                    p = captured(i)
                    if p is None or guide[i] or coverage(p) < 0.25:
                        continue
                    X, Y = to_layout(i % W + 0.5, i // W + 0.5)
                    if band[0] <= Y <= band[1]:
                        xs.append(X)
            spaces = []
            for gi in range(g0, g1):
                g = text['glyphs'][gi]
                if g['text'].strip() or gi in (g0, g1 - 1):
                    continue
                core = (g['x'] + 0.3 * g['advance'], g['x'] + 0.7 * g['advance'])
                peak, peak_expected = 0.0, 0.0
                fx0, fy0 = to_frame(core[0], line['top'] + 6)
                fx1, fy1 = to_frame(core[1], line['top'] + line['height'] - 6)
                for py in range(max(0, math.floor(min(fy0, fy1))), min(H, math.ceil(max(fy0, fy1)))):
                    for px in range(max(0, math.floor(min(fx0, fx1))), min(W, math.ceil(max(fx0, fx1)))):
                        X, Y = to_layout(px + 0.5, py + 0.5)
                        if core[0] <= X <= core[1] and line['top'] + 6 <= Y <= line['top'] + line['height'] - 6:
                            p = captured(py * W + px)
                            if p is not None:
                                peak = max(peak, coverage(p))
                                peak_expected = max(peak_expected, alpha[py * W + px])
                spaces.append({'source': g['source'], 'advance': round(g['advance'], 3),
                               'core_peak_captured': round(peak, 3), 'core_peak_expected': round(peak_expected, 3)})
            lines.append({'logical': line['logical'], 'layout_width': round(line['width'], 3),
                          'pen': [round(v, 3) for v in line['pen']] if line['pen'] else None,
                          'captured_ink_x': [round(min(xs), 2), round(max(xs), 2)] if xs else None,
                          'over_width': line['over_width'], 'interior_spaces': spaces})
        report['texts'][key] = {
            'width': case['width'], 'wrap': case['wrap'], 'pixels_compared': st['pixels'],
            'max_abs_code_value': st['max_abs'], 'pixels_over_2': st['over2'],
            'mean_abs': round(st['sum_abs'] / max(st['pixels'], 1), 4),
            'glyphs_checked': len(glyph_ratio),
            'glyph_ink_ratio_min': min(glyph_ratio) if glyph_ratio else None,
            'glyph_ink_ratio_max': max(glyph_ratio) if glyph_ratio else None,
            'lines': lines}
    text = json.dumps(report, indent=1, ensure_ascii=False)
    if args.out:
        out = Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        stem = Path(args.capture).stem
        (out / f'{stem}-pixels.json').write_text(text + '\n', encoding='utf-8')

        def reference(x, y):
            i = y * W + x
            if guide[i]:
                return bytes((96, 96, 96))
            return bytes(expected(alpha[i]))

        def difference(x, y):
            i = y * W + x
            p = captured(i)
            if p is None:
                return bytes((0, 0, 0))
            if guide[i]:
                return bytes((40, 40, 40))
            d = max(abs(p[ch] - e) for ch, e in enumerate(expected(alpha[i])))
            return bytes((min(255, d * 40), min(255, d * 40), 0)) if d else bytes((0, 0, 48 if i in owner else 0))
        write_png(out / f'{stem}-reference.png', W, H, reference)
        write_png(out / f'{stem}-diff.png', W, H, difference)
    print(text)


if __name__ == '__main__':
    sys.exit(main())
