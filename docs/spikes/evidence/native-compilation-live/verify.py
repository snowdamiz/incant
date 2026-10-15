#!/usr/bin/env python3
"""Replay retained data only. Never launch a player, compiler, window or GPU."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

from analyze import analyze, audio_observation, require
from compilation_analysis import analyze_compilation


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--retained', action='store_true', help='Also hash retained binaries/package inputs')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / 'manifest.json').read_text())
    archive = (root / 'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'], 'Archive identity')
    data = json.loads(gzip.decompress(archive))
    require(set(data) == {r['name'] for r in manifest['records']}, 'Exact archive membership')
    for row in manifest['records'] + manifest['analysis_sources']:
        raw = data[row['name']].encode() if row['name'] in data else (root / row['name']).read_bytes()
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Record identity: ' + row['name'])
        if args.originals:
            require(Path(row['original']).read_bytes() == raw, 'Original identity: ' + row['name'])
    read = lambda name: json.loads(data[name])
    build = read('build-final.json')
    require(build['source'] == manifest['source'] and build['source_tree'] == manifest['source_tree']
            and build['player_sha256'] == manifest['player_sha256'], 'Distinct producing source/player')
    require(build['supervisor_exit_code'] == 0 and build['managed_children_returned_zero_after_cleanup'] == 22
            and build['owned_scratch_absent'] is True and build['physics_mode'] == 'fast'
            and build['offline_startup_ticks'] == 1 and build['full_offline_startup_logical_and_pcm_parity'] is True,
            'Shipping build and limited offline startup')
    require(build['codegen']['incant_game_player'] == {
        'codegen-units': '1', 'debuginfo': '2', 'lto': 'fat', 'opt-level': '3',
        'panic': 'abort', 'split-debuginfo': 'packed'}, 'Actual shipping codegen')
    plan = read('live-controller-001/plan.json')
    result = read('window-001/result.json')
    require(result['plan_sha256'] == sha(data['live-controller-001/plan.json'].encode())
            and result['source'] == build['source'] and result['status'] == 'complete'
            and result['managed_returned_after_cleanup'] is True and result['child_exit'] == result['exit_code'] == 0,
            'Completed owned native replay')
    for name, row in result['artifacts'].items():
        raw = data['window-001/' + name].encode()
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Raw native artifact')
    report = read('window-001/stdout.json')
    require(report['backend'] == 'Metal' and report['adapter'] == 'Apple M5 Pro'
            and report['on_screen'] is True and report['host_steps'] == report['tick'] == 2156
            and report['input_recording']['completed'] is True
            and report['input_recording']['sha256'] == plan['recording_sha256'], 'Actual Mac route')
    require(report['observations'] == read('baseline.json')['observations']
            and report['observations']['passed'] is True and len(report['observations']['checks']) == 8
            and len(report['observations']['entities']) == 17, 'Exact full gameplay observations')
    require(audio_observation(report) == result['audio_device'], 'Typed audio observation')
    presentation = read('window-001/presentations.json')
    creation = read('window-001/compilations.json')
    coverage = analyze(presentation)
    construction = analyze_compilation(creation, presentation)
    require(coverage == read('window-001/coverage.json')
            and construction == read('window-001/creation-coverage.json'), 'Replay both raw analyses')
    require(construction['coverage_limited_or_inconclusive'] is False
            and sum(c['attempts'] for c in construction['counts']) == 95
            and sum(c['attempts'] for c in construction['unbound']) == 4
            and len(creation['snapshot']['rows']) == 91
            and all(row['phase'] == 'Catalog' for row in creation['snapshot']['rows'])
            and construction['host_scopes']['scopes'][4] == {'started': 2157, 'committed': 2156, 'abandoned': 1},
            'Catalog calls and zero observed gameplay creation')
    require(coverage['coverage_limited_or_inconclusive'] is True
            and coverage['outcomes'] == {'reported_zero': 3, 'presented': 2154}
            and coverage['coverage']['missing'] == coverage['coverage']['overflow'] == 0,
            'Ambiguous presentation outcomes retained')
    require(all(result[key] is False for key in ('performance_acceptance', 'shader_stall_acceptance', 'display_acceptance')),
            'No gate inferred')
    review = read('independent-review.json')
    require(review['source'] == manifest['source'] and review['result_sha256'] == sha(data['window-001/result.json'].encode())
            and not any(review['claims'].values()), 'Independent review scope/identity')
    tests = read('live-controller-001/data-tests.json')
    require(tests['returncode'] == 0 and tests['tests_passed'] == 23 and tests['native_children'] == 0,
            'Historical pure-data controls')
    if args.retained:
        for row in plan['files']:
            raw = Path(row['path']).read_bytes()
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Retained input: ' + row['path'])
    print(f"PASS: {len(data)} records; complete native replay; 95 creation calls, none in gameplay; "
          "3 zero display timestamps retained. No performance or phase gate.")


if __name__ == '__main__':
    main()
