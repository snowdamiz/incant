#!/usr/bin/env python3
"""Background luminance of retained 0065 frames, for placing the 0069 HUD (CPU only).

  frame_profile.py rows PNG [X0 X1]        median luminance/contrast per 4-px band
  frame_profile.py boxes PNG L,T,R,B ...    contrast estimates for candidate boxes
  frame_profile.py match WIDE NARROW        test that NARROW is WIDE's centre crop

Reads existing unedited PNGs only; it renders and captures nothing. Contrast is the
WCAG ratio of the fixed theme text colour (236, 238, 242) against the median, 25th
and 10th-percentile *brightest* luminance of a box: an estimate for comparing
placements, never a conformance measurement. Existing HUD text inside a box biases
its estimate upwards; callers choose boxes away from drawn labels.
"""
import struct
import sys
import zlib

TEXT = (236, 238, 242)


def decode(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', path
    at, idat, header = 8, b'', None
    while at < len(data):
        length, kind = struct.unpack('>I4s', data[at:at + 8])
        body = data[at + 8:at + 8 + length]
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
        at += 12 + length
    width, height, depth, colour, _, _, interlace = header
    assert depth == 8 and colour in (2, 6) and not interlace, (path, header)
    channels = 4 if colour == 6 else 3
    raw, stride = zlib.decompress(idat), width * channels
    rows, previous = [], bytearray(stride)
    for y in range(height):
        kind, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b, c = previous[i], previous[i - channels] if i >= channels else 0
            if kind == 1: line[i] = (line[i] + a) & 255
            elif kind == 2: line[i] = (line[i] + b) & 255
            elif kind == 3: line[i] = (line[i] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append([tuple(line[x * channels:x * channels + 3]) for x in range(width)])
        previous = line
    return width, height, rows


def linear(c):
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


LUT = [linear(v) for v in range(256)]


def luminance(rgb):
    return 0.2126 * LUT[rgb[0]] + 0.7152 * LUT[rgb[1]] + 0.0722 * LUT[rgb[2]]


TEXT_L = luminance(TEXT)


def contrast(value):
    return (max(TEXT_L, value) + 0.05) / (min(TEXT_L, value) + 0.05)


def stats(values):
    values = sorted(values)
    pick = lambda q: values[min(len(values) - 1, int(len(values) * q))]
    return {'median': round(contrast(pick(0.5)), 2), 'p75': round(contrast(pick(0.75)), 2),
            'p90': round(contrast(pick(0.9)), 2)}


def box(rows, left, top, right, bottom):
    return stats([luminance(rows[y][x]) for y in range(max(top, 0), min(bottom, len(rows)))
                  for x in range(max(left, 0), min(right, len(rows[0])))])


def main():
    mode, path, *rest = sys.argv[1:]
    if mode == 'match':
        wide, narrow = decode(path), decode(rest[0])
        (ww, wh, wr), (nw, nh, nr) = wide, narrow
        scale = wh / nh
        offset = (ww - nw * scale) / 2
        errors = []
        for y in range(0, nh, 3):
            for x in range(0, nw, 3):
                sx, sy = int(offset + (x + .5) * scale), int((y + .5) * scale)
                errors.append(abs(luminance(nr[y][x]) - luminance(wr[sy][sx])))
        errors.sort()
        print({'scale': round(scale, 4), 'x_offset': round(offset, 1),
               'median_abs_luminance_error': round(errors[len(errors) // 2], 4),
               'p90_abs_luminance_error': round(errors[int(len(errors) * .9)], 4)})
        return
    width, height, rows = decode(path)
    if mode == 'rows':
        x0, x1 = (int(v) for v in rest) if rest else (0, width)
        for top in range(0, height, 4):
            print(top, stats([luminance(rows[y][x]) for y in range(top, min(top + 4, height))
                              for x in range(x0, x1)]))
    elif mode == 'boxes':
        for spec in rest:
            print(spec, box(rows, *(int(v) for v in spec.split(','))))


if __name__ == '__main__':
    main()
