#!/usr/bin/env python3
"""Replay retained Windows diagnostic data; never compile or execute a player."""
import argparse
import base64
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path, PureWindowsPath
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def digest(value):
    return hashlib.sha256(value).hexdigest()


def verify(originals=False):
    receipt = json.loads((HERE / 'hosted-success.json').read_text())
    source = ROOT / 'tools/probes/dx12-pipeline-library.py'
    assert digest(source.read_bytes()) == receipt['probe_sha256'], 'Replay requires the recorded probe version'
    spec = importlib.util.spec_from_file_location('dx12_probe', source)
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    package = receipt['portable_records']
    packed = (HERE / package['file']).read_bytes()
    assert len(packed) == package['compressed_bytes'] and digest(packed) == package['compressed_sha256']
    expanded = gzip.decompress(packed)
    assert len(expanded) == package['expanded_bytes'] and digest(expanded) == package['expanded_sha256']
    rows = json.loads(expanded)
    assert len(rows) == package['records']
    inventory = {PureWindowsPath(r['path']).as_posix(): r for r in receipt['original_inventory']}
    records = {}
    for row in rows:
        name = row['path']
        assert name not in records and name in inventory
        value = base64.b64decode(row['base64'], validate=True)
        assert len(value) == row['bytes'] == inventory[name]['bytes']
        assert digest(value) == row['sha256'] == inventory[name]['sha256']
        records[name] = value
    result = json.loads(records['result.json'])
    assert result['expected_source'] == receipt['source']
    assert result['status'] == 'passed' and result['terminal'] and result['cleanup_confirmed']
    assert len(result['calls']) == receipt['owned_calls'] == 32
    assert all(row['exit'] == 0 and row['managed_call_returned'] and row['cleanup_confirmed']
               and row['call_finished'] for row in result['calls'])
    assert all(row['cleanup_confirmed'] for row in result['owned_processes'])
    frames = []
    for mode in ('ordinary', 'cooked'):
        assert probe.validate_cpu_log(records[f'{mode}-cpu.stdout.log'].decode()) == 20
        for stage in probe.STAGES:
            log = records[f'{mode}-{stage}.stdout.log'].decode()
            assert re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', log) == ['1']
            root = f'{mode}/{stage}-case/{stage}' if stage in ('missing', 'empty') else f'{mode}/{stage}'
            if stage in ('lifetime', 'empty'):
                assert records[f'{root}/result.txt']
            elif stage in ('missing', 'corrupt'):
                assert records[f'{root}/refusal.txt']
            else:
                report = json.loads(records[f'{root}/result.json'])
                assert report['stage'] == stage and report['backend'] == 'Dx12'
                assert report['cooked'] is (mode == 'cooked')
                assert {k: report[k] for k in ('source', 'native_identity', 'adapter')} == result['native_sources'][mode]
                probe.validate_counts(report['counts_before'], stage)
                probe.validate_counts(report['counts_after'], stage, final=True)
                assert all(report['counts_after'][key] >= report['counts_before'][key] for key in probe.COUNTERS)
                raw = records[f'{root}/initial.rgba']
                assert digest(raw) == report['rgba_sha256']
                frames.append(raw)
                if stage != 'seed':
                    frames.append(records[f'{root}/refused.rgba'])
    assert len(frames) == receipt['RGBA_buffers'] == 14
    assert all(len(raw) == 160 * 96 * 4 and raw == frames[0] for raw in frames)
    assert digest(frames[0]) == result['exact_RGBA_sha256'] == receipt['exact_RGBA_sha256']
    assert all(value == 255 for value in frames[0][3::4])
    for channel in range(3):
        assert sum(frames[0][i + channel] > 150 and all(frames[0][i + channel] - frames[0][i + c] > 80
                   for c in range(3) if c != channel) for i in range(0, len(frames[0]), 4)) > 50
    ordinary, cooked = (result['native_sources'][mode] for mode in ('ordinary', 'cooked'))
    assert ordinary['source'] != cooked['source']
    assert ordinary['native_identity'] == cooked['native_identity'] and ordinary['adapter'] == cooked['adapter']
    if originals:
        external = Path(receipt['original_artifact_directory'])
        for name, row in inventory.items():
            relative = Path(name)
            assert not relative.is_absolute() and '..' not in relative.parts
            value = (external / relative).read_bytes()
            assert len(value) == row['bytes'] and digest(value) == row['sha256']
    print(f"Verified {len(rows)} portable records, 40 CPU controls, 16 native stages and 14 identical RGBA buffers; originals={originals}")


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    verify(parser.parse_args().originals)
