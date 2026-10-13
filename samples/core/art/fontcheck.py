"""Glyph coverage of an ordered font chain (TrueType/OpenType cmap formats 4 and 12).

Used by the builder to refuse HUD text the packaged fallback chain cannot shape:
every non-space character of every display string must be mapped by some font.
"""
import struct


def codepoints(data):
    count = struct.unpack('>H', data[4:6])[0]
    tables = {}
    for i in range(count):
        tag, _, offset, length = struct.unpack('>4sIII', data[12 + 16 * i:28 + 16 * i])
        tables[tag] = (offset, length)
    offset, _ = tables[b'cmap']
    subtables = struct.unpack('>H', data[offset + 2:offset + 4])[0]
    points = set()
    for i in range(subtables):
        _, _, sub = struct.unpack('>HHI', data[offset + 4 + 8 * i:offset + 12 + 8 * i])
        start = offset + sub
        fmt = struct.unpack('>H', data[start:start + 2])[0]
        if fmt == 4:
            seg2 = struct.unpack('>H', data[start + 6:start + 8])[0]
            ends = struct.unpack('>%dH' % (seg2 // 2), data[start + 14:start + 14 + seg2])
            starts = struct.unpack('>%dH' % (seg2 // 2), data[start + 16 + seg2:start + 16 + 2 * seg2])
            for s, e in zip(starts, ends):
                if s != 0xFFFF:
                    points.update(range(s, e + 1))
        elif fmt == 12:
            groups = struct.unpack('>I', data[start + 12:start + 16])[0]
            for g in range(groups):
                s, e, _ = struct.unpack('>III', data[start + 16 + 12 * g:start + 28 + 12 * g])
                points.update(range(s, e + 1))
    return points


def coverage(strings, chain):
    """chain: [(name, bytes)]. Returns ({font name: chars it supplies}, [uncovered chars])."""
    maps = [(name, codepoints(data)) for name, data in chain]
    supplied, missing = {name: set() for name, _ in maps}, set()
    for text in strings:
        for ch in text:
            if ch.isspace() or ord(ch) in (0x200E, 0x200F):
                continue
            for name, points in maps:
                if ord(ch) in points:
                    supplied[name].add(ch)
                    break
            else:
                missing.add(ch)
    return {k: ''.join(sorted(v)) for k, v in supplied.items()}, ''.join(sorted(missing))
