#!/usr/bin/env python3
"""Replay immutable texture-refresh evidence; never build or execute native code."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='rehash external originals')
    parser.add_argument('--binaries', action='store_true', help='rehash retained GPU executable, never run it')
    parser.add_argument('--source-root', type=Path, help='check all23 curated source files')
    parser.add_argument('--source-ref', help='with source-root, read source through git show instead of live files')
    args = parser.parse_args()
    require(not args.source_ref or args.source_root is not None, 'source-ref requires source-root')
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / 'manifest.json').read_text())
    archive = (root / 'artifacts.json.gz').read_bytes()
    require(digest(archive) == manifest['archive_sha256'], 'archive identity')
    payloads = {h: base64.b64decode(data, validate=True)
                for h, data in json.loads(gzip.decompress(archive)).items()}
    for h, data in payloads.items():
        require(digest(data) == h, 'payload identity ' + h)
    records = {}
    for row in manifest['records']:
        require(row['id'] not in records, 'duplicate record ' + row['id'])
        data = payloads[row['sha256']]
        require(len(data) == row['bytes'], 'record length ' + row['id'])
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'original ' + row['original'])
        records[row['id']] = row

    def raw(ident):
        return payloads[records[ident]['sha256']]

    def document(ident):
        return json.loads(raw(ident))

    freezes = {}
    for attempt in manifest['attempts']:
        key = attempt['source']
        freeze = document(key + '/freeze.json')
        require(digest(raw(key + '/freeze.json')) == attempt['freeze_sha256'], key + ' freeze')
        require(freeze['source_sha'] == attempt['source_sha'], key + ' source')
        freezes[key] = freeze
        for row in freeze['records']:
            data = raw(key + '/inputs/' + row['path'])
            require(digest(data) == row['sha256'] and len(data) == row['bytes'], key + ' input ' + row['path'])
        for command in attempt['commands']:
            receipt = document(command['receipt'])
            require(receipt['owned_groups_reaped'] is True, command['receipt'] + ' ownership')
            matching = [r for r in receipt['results'] if Path(r['log']).name == command['log']]
            require(len(matching) == 1, command['log'] + ' result binding')
            result = matching[0]
            require(result['source_sha'] == attempt['source_sha'] == command['source_sha'], 'executed source')
            require(result['argv'] == command['argv'] and result['exit_code'] == command['exit_code'], 'executed command')
            require(digest(raw(command['log'])) == command['raw_sha256'] == result['log_sha256'], 'raw result identity')
            counts = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;', raw(command['log']).decode())
            parsed = [{'state':m[0], 'passed':int(m[1]), 'failed':int(m[2])} for m in counts]
            require(parsed == command['test_results'], 'raw test counts')

    require(len(freezes) == 10, 'ten source revisions')
    require([a['source_sha'] for a in manifest['attempts']] == manifest['ordered_source_commits'], 'source order')
    final = freezes['source-010']
    require(final['source_sha'] == manifest['source'] and len(final['records']) == 23, 'final source')
    require(digest(raw('source-010/freeze.json')) == manifest['source010_freeze_sha256'], 'final freeze')
    require(len(manifest['production_paths']) == 17 and len(manifest['test_paths']) == 6, 'source classes')
    require(set(manifest['production_paths'] + manifest['test_paths']) == {r['path'] for r in final['records']}, 'source coverage')
    for path in manifest['production_paths']:
        require(raw('source-008/inputs/' + path) == raw('source-010/inputs/' + path), 'production successor ' + path)
    for row in final['records']:
        if args.source_root is not None:
            if args.source_ref:
                data = subprocess.check_output(['git', 'show', args.source_ref + ':' + row['path']], cwd=args.source_root)
            else:
                data = (args.source_root / row['path']).read_bytes()
            require(digest(data) == row['sha256'], 'requested source ' + row['path'])

    # Preserve refusals and failed attempts; a successful later source never relabels them.
    require(document('focused-source001-results.json')['results'][0]['exit_code'] == -15, 'cancelled001')
    require('not a test result' in document('focused-source001-cancelled.json')['reason'], 'cancel scope')
    require(document('focused-source002-results.json')['results'][0]['exit_code'] == 101, 'compile002')
    require(document('focused-source003-results.json')['results'][0]['exit_code'] == 101, 'compile003')
    require(document('broader-source004-results.json')['results'][-1]['exit_code'] == 101, 'editor compile004')
    for key in ('005', '006'):
        require(document('broader-source' + key + '-results.json')['results'][-1]['exit_code'] == 101, 'lint' + key)
    require('14 passed; 1 failed' in raw('broader-source007-2.raw.log').decode(), 'first-lock failure007')
    require(document('gpu-compile-source009-results.json')['results'][-1]['exit_code'] == 101, 'lint009')
    require('5 passed; 0 failed' in raw('broader-source007-0.raw.log').decode(), 'Play5')
    require('6 passed; 0 failed' in raw('broader-source007-0.raw.log').decode(), 'native6')
    require('1 passed; 0 failed' in raw('broader-source007-1.raw.log').decode(), 'IK1')
    editor = raw('broader-source008-0.raw.log').decode()
    require('15 passed; 0 failed' in editor and 'first=1 second=0; empty catch=0' in editor, 'editor15 and mutex observation')
    require(all(r['exit_code'] == 0 for r in document('broader-source008-results.json')['results']), 'editor and scoped lint008')
    require(all(r['exit_code'] == 0 for r in document('gpu-compile-source010-results.json')['results']), 'compile/lint010')

    launch = document('gpu-launch-010.json')
    gpu = document('gpu-source010-result.json')
    require(gpu['source_sha'] == launch['source_sha'] == manifest['source'], 'actual GPU source')
    require(gpu['argv'] == launch['argv'] and gpu['timeout_seconds'] == 120, 'actual GPU argv/bound')
    require(gpu['exit_code'] == 0 and gpu['owned_groups_reaped'] is True, 'actual GPU result')
    require(digest(raw('gpu-source010.raw.log')) == gpu['log_sha256'], 'actual GPU raw identity')
    require(gpu['executable_sha256'] == launch['executable_sha256'] == manifest['native_binary']['sha256'], 'actual GPU artifact')
    compile_log = raw('gpu-compile-source010-0.raw.log').decode()
    artifacts = [json.loads(line) for line in compile_log.splitlines() if line.startswith('{')]
    matches = [r for r in artifacts if r.get('reason') == 'compiler-artifact' and r.get('target', {}).get('name') == 'game' and r.get('executable')]
    require(len(matches) == 1 and matches[0] == launch['compiler_artifact'], 'unique producing compiler artifact')
    observed = raw('gpu-source010.raw.log').decode()
    require('actual adapter:' in observed and 'backend: Metal' in observed, 'actual Metal')
    require('old_red=2304 new_blue=2304; final_tick=2 frames=6 presents=6' in observed, 'numerical outcome')
    require('1 passed; 0 failed' in observed, 'actual GPU one pass')
    closure = document(manifest['source_closure'])
    require(closure['producer_source'] == manifest['source'] and closure['curated_paths'] == final['records'], 'source closure')
    require(len(closure['base_delta']) == 15, 'diagnostic base delta')
    require(not set(r['path'] for r in closure['base_delta']).intersection(manifest['production_paths'] + manifest['test_paths']), 'no texture base overlap')
    require(manifest['performance_or_phase_gate'] is False and manifest['native_binary']['shipping_or_window'] is False, 'scope')
    if args.binaries:
        binary = manifest['native_binary']
        data = Path(binary.get('retained_copy', binary['path'])).read_bytes()
        require(len(data) == binary['bytes'] and digest(data) == binary['sha256'], 'retained executable')
    print(json.dumps({'verified':True, 'records':len(records), 'source':manifest['source'],
                      'current_production_texture_and_ik_passes':12, 'editor_play_passes':15,
                      'actual_mac_gpu_passes':1, 'failed_attempts_preserved':True,
                      'main283_composition_tests':'not executed in producer packet', 'performance_or_phase_gate':False}))


if __name__ == '__main__':
    main()
