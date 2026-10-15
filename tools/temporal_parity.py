"""Offline comparison of bounded fixed motion/history numeric test outputs.

No renderer, native process, window, capture, or tolerance inference. The shader
test's existing analytical tolerances remain separate from exact cross-mode bytes.
"""
import json
import math
from pathlib import Path
import struct
import sys

from native_package import read, require, sha256

FORMATS = {'r32uint': 4, 'depth32float': 4, 'rgba8srgb': 4, 'rgba16float': 8, 'rgba32float': 16}


def _object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'Duplicate numerical summary key')
        result[key] = value
    return result


def expected(test):
    if test == 'temporal-resolve':
        return {f'mode{i:02}-resolved': (8, 8, 'rgba16float') for i in range(14)}
    require(test == 'motion-owner', 'Unknown numerical test')
    return {f'scale{scale}-frame{frame}-{name}': (32 if name.endswith('color') else scale * 2,
            32 if name.endswith('color') else scale * 2, format)
        for scale in (16, 8) for frame in range(3)
        for name, format in [('on-owner', 'r32uint'), ('on-motion', 'rgba32float'),
            ('on-depth', 'depth32float'), ('on-color', 'rgba8srgb'),
            ('off-motion', 'rgba32float'), ('off-depth', 'depth32float'), ('off-color', 'rgba8srgb')]}


def load(directory, test, cooked):
    summary = json.loads(read(Path(directory) / 'summary.json', 65536), object_pairs_hook=_object)
    require(set(summary) == {'format', 'test', 'cooked_temporal', 'complete', 'records', 'bytes', 'comparison'}, 'Unknown numerical summary fields')
    require(summary['format'] == 'incant-fixed-temporal-parity-v1' and summary['test'] == test
            and summary['complete'] is True and summary['cooked_temporal'] is cooked, 'Incomplete/wrong actual route')
    wanted = expected(test)
    rows = summary['records']
    require(isinstance(rows, list) and len(rows) == len(wanted), 'Missing numerical records')
    result = {}
    for row in rows:
        require(isinstance(row, dict) and set(row) == {'name', 'file', 'width', 'height', 'format', 'bytes', 'sha256'}, 'Unknown numerical record fields')
        name = row['name']
        require(name in wanted and name not in result, 'Unknown/duplicate numerical record')
        width, height, format = wanted[name]
        require(type(row['width']) is int and type(row['height']) is int and
                (row['width'], row['height'], row['format']) == (width, height, format), 'Numeric shape/format changed')
        require(row['file'] == name + '.raw', 'Numeric path changed')
        raw = read(Path(directory) / row['file'], 128 * 128 * 16)
        require(type(row['bytes']) is int and len(raw) == row['bytes'] == width * height * FORMATS[format]
                and sha256(raw) == row['sha256'], 'Raw identity changed')
        result[name] = raw
    require(set(result) == set(wanted) and type(summary['bytes']) is int
            and summary['bytes'] == sum(map(len, result.values())) <= 2 * 1024**2, 'Numerical membership/size changed')
    return result


def analyze_owner(raw):
    moved = []
    for scale in (16, 8):
        for frame in range(3):
            prefix = f'scale{scale}-frame{frame}-'
            for kind in ('motion', 'depth', 'color'):
                require(raw[prefix + 'on-' + kind] == raw[prefix + 'off-' + kind], 'Owner attachment changed prior output')
            ids = [p[0] for p in struct.iter_unpack('<I', raw[prefix + 'on-owner'])]
            require({i & 0x7fffffff for i in ids} == set(range(6)), 'Owner/MASK/deformed fixture coverage missing')
            flagged = {i & 0x7fffffff for i in ids if i & 0x80000000}
            require(len(flagged) == (5 if frame == 0 else 2 if frame == 2 else 0), 'Owner change chronology changed')
            motion = list(struct.iter_unpack('<4f', raw[prefix + 'on-motion']))
            require(all(all(math.isfinite(x) for x in p) and p[3] in (0., 1.) for p in motion), 'Invalid motion output')
            if frame == 0:
                require(all(p == (0., 0., 0., 0.) for p in motion), 'First frame accepted prior motion')
            elif frame == 1:
                require(any(p[3] == 1. for p in motion) and all(abs(p[0]) <= 1e-6 and abs(p[1]) <= 1e-6 for p in motion), 'Stationary history missing/changed')
            else:
                count = sum(p[3] == 1. and (abs(p[0]) > 1e-6 or abs(p[1]) > 1e-6) for p in motion)
                require(count > 0, 'Moved/deformed fixture produced no motion')
                moved.append(count)
            depth = struct.iter_unpack('<f', raw[prefix + 'on-depth'])
            require(all(math.isfinite(p[0]) and 0. <= p[0] <= 1. for p in depth), 'Invalid depth output')
    return moved


def analyze_resolve(raw):
    for mode in range(14):
        values = list(struct.iter_unpack('<4e', raw[f'mode{mode:02}-resolved']))
        require(all(all(math.isfinite(x) for x in p) for p in values), 'Nonfinite temporal output')
        expected = ((0.375, 0.375, 0.375, 1.) if mode in (0, 1, 8, 9, 11, 13) else
            (0., 0., 0., 0.) if mode == 4 else (0.2, 0.1, 0., 0.5) if mode == 5 else
            (2., 0.5, 100., 1.) if mode == 7 else
            (0.2 + 0.03 * 3.375 + 0.02 * 2.625, 0.25, 0.5, 1.) if mode == 12 else (0.5, 0.5, 0.5, 1.))
        require(all(abs(a - b) < 0.001 for a, b in zip(values[3 * 8 + 3], expected)), 'Existing fourteen-mode analytical oracle failed')


def compare(owner_default, owner_cooked, resolve_default, resolve_cooked):
    pairs = [(load(owner_default, 'motion-owner', False), load(owner_cooked, 'motion-owner', True)),
             (load(resolve_default, 'temporal-resolve', False), load(resolve_cooked, 'temporal-resolve', True))]
    for a, b in pairs:
        require(a == b, 'Default/cooked raw mismatch; exact tolerance unchanged')
    moved = analyze_owner(pairs[0][0])
    analyze_resolve(pairs[1][0])
    return {'status': 'pass', 'comparison': 'exact raw bytes', 'tolerance_codes': 0,
            'owner_records_each': 42, 'resolve_records_each': 14, 'nonzero_motion_pixels': moved,
            'temporal_analytical_absolute_tolerance': 0.001, 'appearance_or_performance_gate': False}


if __name__ == '__main__':
    require(len(sys.argv) == 5, 'Expected default/cooked owner then default/cooked resolve directories')
    print(json.dumps(compare(*map(Path, sys.argv[1:])), indent=2))
