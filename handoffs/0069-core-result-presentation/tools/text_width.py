#!/usr/bin/env python3
"""Unshaped advance-width estimate of HUD strings over the sample font chain (CPU only).

  text_width.py [--pseudo] SIZE TEXT [TEXT ...]

Sums each character's horizontal advance from the first font in the chain whose cmap
maps it (Inter, Noto Sans Arabic, the SC subset), at SIZE logical pixels. There is no
shaping, kerning or Arabic joining, so this is a planning estimate only; layout
acceptance comes from the engine's own measured widths in a granted run. --pseudo
first applies a transcription of incant_localization's en-XA transform (format.rs
`pseudo` plus the catalog's "[!! " / " !!]" wrapper) to plain-text messages.
"""
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CHAIN = [ROOT / 'third_party/cosmic-text/fonts/Inter-Regular.ttf',
         ROOT / 'third_party/cosmic-text/fonts/NotoSansArabic.ttf',
         ROOT / 'samples/core/art/fonts/CoreSampleSC-Regular.ttf']


def tables(data):
    count = struct.unpack('>H', data[4:6])[0]
    out = {}
    for i in range(count):
        tag, _, offset, length = struct.unpack('>4sIII', data[12 + 16 * i:28 + 16 * i])
        out[tag.decode('latin1')] = (offset, length)
    return out


def cmap(data, table):
    offset = table['cmap'][0]
    mapping = {}
    for i in range(struct.unpack('>H', data[offset + 2:offset + 4])[0]):
        _, _, sub = struct.unpack('>HHI', data[offset + 4 + 8 * i:offset + 12 + 8 * i])
        start = offset + sub
        kind = struct.unpack('>H', data[start:start + 2])[0]
        if kind == 4:
            seg = struct.unpack('>H', data[start + 6:start + 8])[0] // 2
            ends = struct.unpack('>%dH' % seg, data[start + 14:start + 14 + 2 * seg])
            starts = struct.unpack('>%dH' % seg, data[start + 16 + 2 * seg:start + 16 + 4 * seg])
            deltas = struct.unpack('>%dh' % seg, data[start + 16 + 4 * seg:start + 16 + 6 * seg])
            ranges_at = start + 16 + 6 * seg
            ranges = struct.unpack('>%dH' % seg, data[ranges_at:ranges_at + 2 * seg])
            for k in range(seg):
                for code in range(starts[k], ends[k] + 1):
                    if code == 0xFFFF:
                        continue
                    if ranges[k] == 0:
                        glyph = (code + deltas[k]) & 0xFFFF
                    else:
                        at = ranges_at + 2 * k + ranges[k] + 2 * (code - starts[k])
                        glyph = struct.unpack('>H', data[at:at + 2])[0]
                        glyph = (glyph + deltas[k]) & 0xFFFF if glyph else 0
                    if glyph:
                        mapping.setdefault(code, glyph)
        elif kind == 12:
            for g in range(struct.unpack('>I', data[start + 12:start + 16])[0]):
                first, last, glyph = struct.unpack('>III', data[start + 16 + 12 * g:start + 28 + 12 * g])
                for code in range(first, last + 1):
                    mapping.setdefault(code, glyph + code - first)
    return mapping


def font(path):
    data = path.read_bytes()
    table = tables(data)
    units = struct.unpack('>H', data[table['head'][0] + 18:table['head'][0] + 20])[0]
    metrics = struct.unpack('>H', data[table['hhea'][0] + 34:table['hhea'][0] + 36])[0]
    at = table['hmtx'][0]
    advances = [struct.unpack('>H', data[at + 4 * i:at + 4 * i + 2])[0] for i in range(metrics)]
    return cmap(data, table), advances, units


def width(text, size, fonts):
    total = 0.0
    for ch in text:
        for mapping, advances, units in fonts:
            glyph = mapping.get(ord(ch))
            if glyph is not None:
                total += advances[min(glyph, len(advances) - 1)] * size / units
                break
    return total


PLAIN = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
ACCENTED = 'áƀçďéƒğĥíĵķľɱñóþɋŕšţúṽŵẋýžÁɃÇĎÉƑĞĤÍĴĶĽṀÑÓÞɊŔŠŢÚṼŴẊÝŽ'


def pseudo(text):
    out = []
    for c in text:
        i = PLAIN.find(c)
        out.append(c if i < 0 else ACCENTED[i] * (2 if c in 'aeiouAEIOU' else 1))
    return '[!! ' + ''.join(out) + ' !!]'


def main():
    args = sys.argv[1:]
    transform = pseudo if args[:1] == ['--pseudo'] else (lambda text: text)
    args = args[1:] if args[:1] == ['--pseudo'] else args
    size, texts = float(args[0]), [transform(t) for t in args[1:]]
    fonts = [font(p) for p in CHAIN]
    for text in texts:
        print(f'{width(text, size, fonts):7.1f}  {text}')


if __name__ == '__main__':
    main()
