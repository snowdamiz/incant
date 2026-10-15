#!/usr/bin/env python3
"""Replay retained native Arabic results without launching a player or capture."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(value):
    return hashlib.sha256(value).hexdigest()


def image_metadata(content):
    # Capture bytes may be JPEG even when the retained path ends with .png.
    if content.startswith(b'\x89PNG\r\n\x1a\n') and content[12:16] == b'IHDR':
        return 'image/png', int.from_bytes(content[16:20], 'big'), int.from_bytes(content[20:24], 'big')
    require(content.startswith(b'\xff\xd8'), 'Known capture encoding')
    position = 2
    while position < len(content):
        require(content[position] == 255, 'JPEG marker prefix')
        while position < len(content) and content[position] == 255:
            position += 1
        marker = content[position]
        position += 1
        require(marker not in (0xD9, 0xDA), 'JPEG frame header before scan')
        length = int.from_bytes(content[position:position + 2], 'big')
        require(length >= 2 and position + length <= len(content), 'Bounded JPEG segment')
        if marker in (0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF):
            require(length >= 8, 'JPEG frame dimensions')
            height = int.from_bytes(content[position + 3:position + 5], 'big')
            width = int.from_bytes(content[position + 5:position + 7], 'big')
            require(width > 0 and height > 0, 'Positive dimensions')
            return 'image/jpeg', width, height
        position += length
    raise ValueError('Missing image dimensions')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--images', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(manifest['format'] == 'incant-core-arabic-corrected-window-v1', 'Known format')
    packed = (HERE / 'records.json.gz').read_bytes()
    require(sha(packed) == manifest['archive_sha256'], 'Archive hash')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        raw = stream.read(8 * 1024**2 + 1)
    require(len(raw) <= 8 * 1024**2, 'Finite archive')
    data = {}
    for row in json.loads(raw)['records']:
        name = row['name']
        require(name not in data and not Path(name).is_absolute()
                and '..' not in Path(name).parts, 'Unique relative record')
        content = base64.b64decode(row['base64'], validate=True)
        require(len(content) == row['bytes'] and sha(content) == row['sha256'], 'Record hash')
        data[name] = content
    require(len(data) == manifest['records'], 'Record count')
    read = lambda name: json.loads(data[name])
    baseline = read('baseline.json')
    observation_plan = read('lose-restart-win-observe.json')
    recording = read('recordings.json')['cases'][-1]
    require(manifest['producing_source'] == '069bbe54dc529511138867dfd4fbe50507a38b6b',
            'Historical producing source retained')
    for attempt in ('002', '003', '004'):
        prefix = f'window-{attempt}/'
        plan_data = data[f'native-window-{attempt}/plan.json']
        plan = json.loads(plan_data)
        result = read(prefix + 'result.json')
        require(result['terminal'] is True and result['plan_sha256'] == sha(plan_data)
                and result['appearance_review'] is False, 'Terminal scoped result')
        require(plan['source'] == manifest['producing_source'] and plan['stop_after'] == 3056
                and plan['native_child_seconds'] == 120, 'Unchanged source and replay bounds')
        grant = read(f'native-window-{attempt}/root-window-grant.json')
        require(grant['plan_sha256'] == sha(plan_data)
                and grant['exclusive_GPU_window_CUA'] is True, 'Matching execution grant')
        life = read(prefix + 'lifecycle.json')
        require(life['cleanup_confirmed'] is True and not life['cleanup_errors']
                and len(life['calls']) == len(life['cleanup_records']) == 1
                and life['cleanup_records'][0]['original_stop_returned'] is True,
                'Owned child cleanup retained')
        preflight = read(prefix + 'os-lock-preflight.json')
        require(preflight['admitted'] is True and preflight['metadata_only'] is True
                and preflight['observed'] == {'console_unlocked': True,
                    'own_console_login_complete': True, 'secondary_lock_field_present': False},
                'Explicit unlocked console; optional secondary field absent')
        rows = [json.loads(line) for line in data[prefix + 'runner/eligibility.jsonl'].splitlines()]
        if attempt == '004':
            require(result['passed'] is False and 'TimeoutExpired' in result['error'],
                    'Focus-paused timeout retained')
            final = rows[-1]
            require(final['kind'] == 'snapshot' and final['state']['host_steps'] == 13
                    and final['state']['simulation_suspended'] is True
                    and final['focus_lost_events'] == 1, 'Actual incomplete focus-paused state')
            continue
        require(result['passed'] is True and life['calls'][0]['exit'] == 0, 'Complete native attempt')
        report = read(prefix + 'runner/lose-restart-win.json')
        require(report == read(prefix + 'runner/lose-restart-win.stdout.raw.log'), 'Raw report equality')
        require(report['mode'] == 'native-window' and report['on_screen'] is True
                and report['backend'] == 'Metal' and report['adapter'] == 'Apple M5 Pro'
                and report['host_steps'] == 3056 and report['tick'] == 2156, 'Actual whole native replay')
        require(report['input_recording']['sha256'] == recording['recording_sha256']
                and report['input_recording']['completed'] is True
                and report['input_recording']['physical_devices'] is False
                and report['input_recording']['native_focus_and_surface_required'] is True,
                'Complete recorded input, not physical device validation')
        observed = report['observations']
        require(observed == baseline['observations'] and observed['passed'] is True
                and set(observed['entities']) == set(observation_plan['entities']), 'Exact CPU/window state parity')
        checks = sorted(observation_plan['checks'], key=lambda row: (row['tick'], row['name']))
        require(len(checks) == len(observed['checks']) > 0, 'Complete observation membership')
        for actual, expected in zip(observed['checks'], checks, strict=True):
            require(actual['passed'] is True and all(actual[k] == expected[k]
                    for k in ('name', 'tick', 'expect')), 'Exact expected gameplay check')
        require(report['session'] == {'generation': 2, 'host_tick': 3056,
                'simulation_tick': 2156, 'accepted_frames': 3056 + report['presentation_refreshes']},
                'Restart and final world clock')
        require(report['ui']['viewport'] == [640.0, 480.0] and report['ui']['widgets'] == 7
                and rows[-1]['kind'] == 'final' and rows[-1]['host_steps'] == 3056,
                'Normal-size logical viewport and final native log')
    close = read('root-closeout.json')
    require(close['cua_reset'] is True and all(row['independently_absent'] is True
            for row in close['attempts']), 'Independent shutdown observation')
    review = read('visual-review-corrected.json')
    require(review['images'] == manifest['retained_images'], 'Corrected format-aware image inventory')
    require(review['appearance_complete'] is False and review['phase_gate'] is False
            and 'winning-screen pixels' in review['unverified'], 'Partial appearance scope retained')
    require('Ran 554 tests' in data['python-tools.log'].decode()
            and 'OK (skipped=4)' in data['python-tools.log'].decode(), 'Python suite result')
    require(read('source-review.json')['disposition'] == 'accepted_no_blocking_findings',
            'Independent source review')
    require(manifest['appearance_complete'] is False
            and manifest['performance_or_phase_gate'] is False, 'No broad gate claim')
    selected = (manifest['retained_originals'] if args.originals else [])
    selected += manifest['retained_images'] if args.images else []
    for row in selected:
        content = Path(row['path']).read_bytes()
        require(len(content) == row['bytes'] and sha(content) == row['sha256'], 'Retained original identity')
        if 'mime_type' in row:
            require(image_metadata(content) == (row['mime_type'], row['width'], row['height']),
                    'Actual capture format and dimensions')
    print('PASS two complete native replays, one retained focus timeout; partial Arabic pixels; no phase gate')


if __name__ == '__main__':
    main()
