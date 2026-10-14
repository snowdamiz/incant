#!/usr/bin/env python3
"""Read-only replay of retained diagnostic data; never launches native code."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def checked(path, row):
    if path.stat().st_size != row['bytes'] or digest(path) != row['sha256']:
        raise ValueError(f'Retained identity changed: {path}')


def observations(value):
    def compact(group):
        return {key: item for key, item in group.items() if key != 'frames'}

    tail = value['tail_rows']
    publication = value['regions'].index('publication')
    largest = sorted(tail, key=lambda r: (-r['regions_ns'][publication], r['frame']))[:6]
    return {
        'format': 'incant-script-attribution-results', 'version': 1,
        'diagnostic_only': True, 'eligible_for_performance_gate': False,
        'regions': value['regions'], 'numeric_regions': value['numeric_regions'],
        'sources': value['sources'],
        'all_frames': compact(value['all_frames']),
        'same_frame_tail': compact(value['same_frame_tail']),
        'tail_tick_modulo_15_counts': dict(sorted(Counter(str(r['frame'] % 15) for r in tail).items())),
        'tail_on_authored_refresh_phases': sum(r['frame'] % 15 in (1, 2, 3) for r in tail),
        'six_largest_tail_publications': [
            {'frame': r['frame'], 'whole_tick_ns': r['whole_tick_ns'],
             'publication_ns': r['regions_ns'][publication],
             'callback_ns': r['source_ns'][:2]} for r in largest],
        'six_publications_total_ns': sum(r['regions_ns'][publication] for r in largest),
        'limits': [
            'Modulo correlation is not a measured findPath duration or exclusive navigation attribution',
            'Nested durations remain inside their major region; no marginal quantile subtraction',
            'All 2156 frames, including startup and six large publications, remain included',
            'One instrumented process supplies observations, not a normal shipping performance verdict'],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external', type=Path, help='Retained evidence directory; defaults to manifest location')
    parser.add_argument('--binaries', action='store_true', help='Also rehash all 45 original build payloads')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[3]
    manifest = json.loads((here / 'manifest.json').read_text())
    external = args.external or Path(manifest['external_root'])
    for name, row in manifest['external_files'].items():
        checked(external / name, row)
    for name, row in manifest['source_files'].items():
        checked(root / name, row)
    for name, row in manifest['local_files'].items():
        checked(here / name, row)
    sys.path.insert(0, str(root / 'tools/probes/core-script-attribution'))
    from analyze import analyze

    raw = json.loads((external / 'private-diagnostic-001/stdout.json').read_text())
    functional = json.loads((external / 'private-functional-001/stdout.json').read_text())
    if {k: v for k, v in raw.items() if k != 'profile'} != functional:
        raise ValueError('Full diagnostic logical result differs from functional control')
    calculated = analyze(raw, manifest['expected_sources'])
    if calculated != json.loads((external / 'private-diagnostic-001/attribution.json').read_text()):
        raise ValueError('Retained full same-frame analysis differs')
    if observations(calculated) != json.loads((here / 'observations.json').read_text()):
        raise ValueError('Compact observations differ')
    machines = []
    for stage in ('functional', 'diagnostic'):
        base = external / f'private-{stage}-001'
        result = json.loads((base / 'result.json').read_text())
        if not (result['exit'] == 0 and result['terminal_reaped'] and result['retries'] == 0
                and result['logical_and_pcm_equal'] and result['status'] == 'complete'):
            raise ValueError('Incomplete child result')
        for name, row in result['artifacts'].items():
            checked(base / name, row)
        if result['artifacts']['output.pcm']['sha256'] != manifest['baseline_pcm_sha256']:
            raise ValueError('PCM differs from fixed historical control')
        machines.extend((result['machine_before'], result['machine_after']))
    if not all(machine == machines[0] for machine in machines):
        raise ValueError('Machine identity differs')
    if digest(external / 'private-functional-001/stdout.json') != manifest['baseline_logical_sha256']:
        raise ValueError('Functional result differs from fixed historical control')
    if args.binaries:
        audit = json.loads((external / 'private-build-002-audit.json').read_text())
        for name, row in audit['files'].items():
            checked(external / 'private-build-002' / name, row)
    print(json.dumps({'verified_external_records': len(manifest['external_files']),
                      'same_frame_rows': 2156, 'tail_rows': 108,
                      'original_build_payloads': 45 if args.binaries else 'not replayed',
                      'eligible_for_performance_gate': False}, sort_keys=True))


if __name__ == '__main__':
    main()
