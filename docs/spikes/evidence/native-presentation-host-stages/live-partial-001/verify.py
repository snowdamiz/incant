#!/usr/bin/env python3
"""Verify retained partial data; never execute archived code or native artifacts."""
import argparse
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
BASE = '/Users/andreyyurlov/.codex/incant-evidence/native-host-stages-player-001/'
MAX_COMPRESSED = 2 * 1024**2
MAX_EXPANDED = 8 * 1024**2


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        out = {}
        for k, v in items:
            require(k not in out, 'Duplicate JSON key')
            out[k] = v
        return out
    def invalid(_):
        raise ValueError('Nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def load():
    manifest = decode((HERE / 'manifest.json').read_bytes())
    with (HERE / 'records.json.gz').open('rb') as stream:
        raw = stream.read(MAX_COMPRESSED + 1)
    require(len(raw) <= MAX_COMPRESSED and len(raw) == manifest['archive_bytes']
            and digest(raw) == manifest['archive_sha256'], 'Compressed archive identity')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        expanded = stream.read(MAX_EXPANDED + 1)
    require(len(expanded) <= MAX_EXPANDED and len(expanded) == manifest['expanded_bytes'],
            'Expanded archive bound')
    packet = decode(expanded)
    require(packet['format'] == 'incant-host-stage-partial-records-v1', 'Record format')
    records = {}
    for row in packet['records']:
        require(row['path'] not in records, 'Duplicate record')
        text = row['text'].encode()
        require(len(text) == row['bytes'] and digest(text) == row['sha256'], 'Record byte identity')
        records[row['path']] = row['text']
    require(len(records) == manifest['record_count'] == 22, 'Record inventory')
    return manifest, records


def validate(manifest, records):
    def text(name):
        return records[BASE + name]
    def get(name):
        return decode(text(name))
    result, child, closeout = [get('window-001/' + n) for n in
                               ('result.json', 'child-result.json', 'root-closeout.json')]
    plan = get('live-controller-002/plan.json')
    grant = get('live-controller-002/root-grant.json')
    require(manifest['source'] == result['source'] == plan['source'] == closeout['source']
            == 'e93ac30a9cce9deadd4013ac5fa7abbc3e60a2ac', 'Producer source')
    require(plan['player_sha256'] == manifest['player_sha256']
            == '45c5374583b7543af7e4b5d77a6776d79f7a5eeb3c945c9b640adf5b3a970ca6', 'Player identity')
    require(result['plan_sha256'] == grant['plan_sha256']
            == digest(text('live-controller-002/plan.json').encode()), 'Plan binding')
    require(result['runner_sha256'] == grant['runner_sha256']
            == digest(text('live-controller-002/run.py').encode()), 'Runner binding')
    require(closeout['result_sha256'] == digest(text('window-001/result.json').encode()), 'Closeout binding')
    require(result['status'] == 'failed' and result['exit_code'] == child['exit_code']
            == closeout['actual_wrapper_exit'] == 1 and result['retries'] == 0, 'Failed run must remain failed')
    require(child['cleanup_confirmed'] and not child['cleanup_errors']
            and child['error'] is None and closeout['wrapper_terminal_reaped'], 'Cleanup/terminal receipt')
    require(set(closeout['owned_PID_and_group_absence']) == {62934, 62944}
            and child['supervisor_pid'] == 62934
            and child['known_identities'][0][0] == 62944, 'Owned process identities')
    require(child['elapsed_seconds'] <= plan['child_seconds'] == 120
            and result['elapsed_seconds'] <= plan['stage_seconds'] == 150, 'Actual time charges')
    require(not result['performance_acceptance'] and not result['display_acceptance']
            and not result['live_device_pcm_captured'] and not manifest['full_gameplay_parity']
            and not manifest['performance_gate_passed'], 'Unsupported acceptance claim')
    require(text('window-001/stdout.json') == ''
            and text('window-001/stderr.log') == 'Error: Invalid("native observation finished before all checks")\n',
            'Original terminal diagnostics')
    for name, row in result['artifacts'].items():
        raw = text('window-001/' + name).encode()
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Original output binding')

    # Import only the live repository's reviewed analyzer, never an archived script.
    source = REPO / 'tools/presentation_host_stages.py'
    require(digest(source.read_bytes()) == manifest['host_analyzer_sha256'], 'Trusted analyzer identity changed')
    spec = importlib.util.spec_from_file_location('host_stage_analysis', source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    capture = get('window-001/presentations.json')
    actual = module.analyze(capture)
    actual['input_sha256'] = digest(text('window-001/presentations.json').encode())
    require(actual == get('window-001/host-stages.json'), 'Recomputed host analysis differs')
    require(not actual['complete_requested_host_timing'] and actual['partial_reasons'] == [
        'requested_host_steps_not_fully_committed', 'incomplete_or_failed_finalization'], 'Partial route classification')
    require(capture['finalization']['accepted_host_steps'] == 263
            and capture['requested_host_steps'] == 2156
            and not capture['finalization']['returned_report'], 'Incomplete replay counts')
    attempts = {a['raw']['id']: a for a in actual['attempts']}
    pairs = [p for p in actual['adjacent_host_finish_intervals'] if p['state'] == 'valid']
    largest = []
    for pair in sorted(pairs, key=lambda p: p['ns'], reverse=True)[:5]:
        before, after = attempts[pair['before_attempt']], attempts[pair['after_attempt']]
        t = after['raw']['host_timing']
        wait = t['schedule']['pulse_ns'] - before['raw']['host_timing']['finished_ns']
        metrics = after['metrics']
        require(wait + metrics['pre_attempt']['ns'] + metrics['finished_total']['ns'] == pair['ns'],
                'Individual interval decomposition')
        largest.append({'before_step': before['raw']['requested_host_step'],
                        'after_step': after['raw']['requested_host_step'], 'interval_ns': pair['ns'],
                        'between_previous_finish_and_next_pulse_ns': wait,
                        'wake_lateness_ns': t['schedule']['pulse_ns'] - t['schedule']['deadline_ns'],
                        'next_metrics': metrics})
    states = [decode(line) for line in text('window-001/window-state.jsonl').splitlines()]
    snapshots = [s for s in states if s['kind'] == 'snapshot']
    loss = next(s for s in snapshots if s['reason'] == 'focus_lost')
    tail = [s for s in snapshots if s['elapsed_us'] >= loss['elapsed_us']]
    require(all(s['state']['host_steps'] == s['state']['world_tick'] == 263
                and not s['state']['cached_focused'] and not s['state']['native_focused']
                and s['state']['simulation_suspended'] for s in tail), 'Inactive tail changed')
    require(states[-1]['kind'] == 'final' and states[-1]['host_steps'] == 263
            and not states[-1]['returned_report'], 'Window finalization')
    callbacks = get('window-001/callback-coverage.json')
    positive, equal_or_reversed = [], []
    raw_rows = capture['rows']
    for index, (before, after) in enumerate(zip(raw_rows, raw_rows[1:])):
        if (before['segment'], before['surface_generation']) != (after['segment'], after['surface_generation']):
            continue
        if before['outcome'] == after['outcome'] == 'presented':
            delta = after['presented_seconds'] - before['presented_seconds']
            if delta <= 0:
                equal_or_reversed.append({'before': index, 'after': index + 1, 'seconds': delta})
            else:
                positive.append({'before': index, 'after': index + 1, 'seconds': delta})
    require(equal_or_reversed == callbacks['timestamp_order_issues']
            and positive == [{k: r[k] for k in ('before', 'after', 'seconds')}
                             for r in callbacks['observed_positive_intervals']], 'Raw callback adjacency differs')
    largest_callback = max(positive, key=lambda r: r['seconds'])
    before, after = [raw_rows[largest_callback[k]] for k in ('before', 'after')]
    host_pair_ns = (attempts[after['attempt']]['raw']['host_timing']['finished_ns']
                    - attempts[before['attempt']]['raw']['host_timing']['finished_ns'])
    worst_wake = max((b for b in actual['scheduler_batches'] if b['wake_lateness']['state'] == 'valid'),
                     key=lambda b: b['wake_lateness']['ns'])
    after_wake = attempts[worst_wake['attempt_ids'][0]]
    prior_wake = attempts[after_wake['raw']['id'] - 1]
    prior_finish = prior_wake['raw']['host_timing']['finished_ns']
    native = get('window-001/compilations.json')
    creation = get('window-001/creation-coverage.json')
    require(native['finalization']['accepted_host_steps'] == 263
            and not native['finalization']['returned_report'], 'Creation route is partial')
    require(len(native['snapshot']['rows']) == 91
            and all(r['phase'] == 'Catalog' and r['outcome'] == 'Success'
                    for r in native['snapshot']['rows']), 'Observed native call scope')
    creation_rows = native['snapshot']['rows']
    for row in creation['phase_kind_observations']:
        selected = [r for r in creation_rows if (r['phase'], r['kind']) == (row['phase'], row['kind'])]
        require(len(selected) == row['calls'] == row['valid_duration_calls']
                and row['outcomes'] == {'Success': len(selected)}
                and row['sum_call_ns'] == sum(r['elapsed_ns'] for r in selected)
                and row['max_call_ns'] == max(r['elapsed_ns'] for r in selected), 'Raw creation durations differ')
    require(sum(r['calls'] for r in creation['phase_kind_observations']) == len(creation_rows),
            'Creation summary omits rows')
    return {'status': 'failed_partial_263_of_2156', 'host_summary': actual['summary'],
            'largest_adjacent_host_intervals': largest,
            'first_recorded_focus_loss_us': loss['elapsed_us'],
            'last_inactive_snapshot_us': tail[-1]['elapsed_us'],
            'inactive_snapshot_count': len(tail), 'final_window_state_us': states[-1]['elapsed_us'],
            'physical_extents': [list(p) for p in sorted({tuple(s['state']['physical_size']) for s in snapshots})],
            'callback_outcomes': callbacks['outcomes'],
            'positive_adjacent_callback_intervals': len(callbacks['observed_positive_intervals']),
            'callback_timestamp_order_issues': callbacks['timestamp_order_issues'],
            'largest_callback_interval': {'before_step': before['requested_host_step'],
                'after_step': after['requested_host_step'], 'callback_seconds': largest_callback['seconds'],
                'corresponding_host_finish_interval_ns': host_pair_ns,
                'cross_epoch_subtraction_performed': False},
            'maximum_wake_lateness': {'step': after_wake['raw']['requested_host_step'],
                'lateness_ns': worst_wake['wake_lateness']['ns'],
                'previous_finished_attempt_ns': prior_wake['metrics']['finished_total']['ns'],
                'previous_finish_to_next_pulse_ns': worst_wake['schedule']['pulse_ns'] - prior_finish},
            'creation_observations': creation['phase_kind_observations'],
            'creation_local_coverage_flag_is_not_route_completion': True,
            'live_device_audio_diagnostics_available': False,
            'full_gameplay_parity': False, 'performance_gate_passed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    args = parser.parse_args()
    manifest, records = load()
    result = validate(manifest, records)
    if args.originals:
        for path, text in records.items():
            require(Path(path).read_bytes() == text.encode(), 'Original differs: ' + path)
    require(result == decode((HERE / 'summary.json').read_bytes()), 'Derived summary differs')
    print('PASS: 22 exact records; partial263/2156, failed exit and focus tail retained; no native execution or gate claim')


if __name__ == '__main__':
    main()
