#!/usr/bin/env python3
"""Read-only, bounded analysis of the native presentation host-clock contract.

No engine execution and no arithmetic involving Metal's timestamp epoch. A valid
subset summary is never a performance verdict or proof of complete gameplay.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sys

MAX_ROWS = 36_000
MAX_BYTES = 64 * 1024 * 1024
U64 = (1 << 64) - 1
BOUNDARIES = ['start', 'focus', 'suspend', 'resume', 'resize', 'configure',
              'recreate', 'retry', 'restart', 'role', 'inactive']
FAILURES = ['timeout', 'occluded', 'outdated', 'lost', 'validation',
            'unsupported_size', 'minimized', 'reservation', 'device']
ROLES = ('initial', 'background', 'ui_resize', 'gameplay')
STATUSES = ('open', 'deferred', 'refresh_completed', 'host_committed')
OUTCOMES = {'acquired': 'acquired', 'requested_missing_callback': 'missing',
            'presented': 'presented', 'reported_zero': 'zero', 'invalid': 'invalid',
            'discarded': 'discarded', 'abandoned': 'abandoned'}
CONTRACT = {
    'version': 1, 'unit': 'nanoseconds',
    'clock': 'Rust Instant relative to capture creation',
    'shared_epoch_with_metal': False, 'changes_frame_admission': False,
    'pulse': 'MainEventsCleared entry, before polling and eligibility work',
    'deadline': 'existing fixed-step clock deadline before due calculation; absent for forced resume',
    'began': 'attempt before surface reservation',
    'reserved': 'successful reservation including view creation',
    'completed': 'existing host-commit or refresh/defer point; not GPU completion',
    'finished': 'gameplay after observations and haptics; before restart replacement or event-loop cleanup',
    'missing': 'unreached, invalid or unavailable; never zero-filled',
}
LIMITS = {'rows': MAX_ROWS, 'row_bytes_max': 64, 'attempt_bytes_max': 192,
          'segment_bytes_max': 40, 'segments_per_attempt_max': 2,
          'drain_seconds': 1, 'output_bytes': MAX_BYTES}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def uint(value, name, maximum=U64, nullable=False):
    require((nullable and value is None) or
            (type(value) is int and 0 <= value <= maximum), f'{name}: invalid unsigned integer')


def exact(value, expected, name):
    # JSON booleans must not compare equal to integer versions or bounds.
    require(json.dumps(value, sort_keys=True, allow_nan=False) ==
            json.dumps(expected, sort_keys=True, allow_nan=False), f'{name}: unsupported contract')


def keys(value, names, name):
    require(type(value) is dict and set(value) == set(names.split()), f'{name}: invalid fields')


def array(value, maximum, name):
    require(type(value) is list and len(value) <= maximum, f'{name}: invalid/bounded array')


def duplicate_free(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f'duplicate JSON key: {key[:80]}')
        result[key] = value
    return result


def read_bounded(path, limit):
    with Path(path).open('rb') as stream:
        raw = stream.read(limit + 1)
    require(len(raw) <= limit, 'input exceeds selected byte bound')
    return raw


def read_capture(path, max_input_mib=64):
    require(max_input_mib in (32, 64), 'input bound must be 32 or 64 MiB')
    raw = read_bounded(path, max_input_mib * 1024 * 1024)
    def invalid_constant(value):
        raise ValueError(f'non-JSON numeric constant: {value}')
    value = json.loads(raw, object_pairs_hook=duplicate_free, parse_constant=invalid_constant)
    return value, hashlib.sha256(raw).hexdigest()


def delta(start, end, applicable=True):
    if not applicable:
        return {'state': 'not_applicable', 'ns': None}
    if start is None or end is None:
        return {'state': 'missing', 'ns': None}
    if end < start:
        return {'state': 'invalid', 'ns': None}
    return {'state': 'valid', 'ns': end - start}


def summarize(metrics):
    counts = {state: sum(m['state'] == state for m in metrics)
              for state in ('valid', 'missing', 'invalid', 'not_applicable')}
    values = sorted(m['ns'] for m in metrics if m['state'] == 'valid')
    return {**counts, 'valid_subset_sum_ns': sum(values) if values else None,
            'valid_subset_min_ns': values[0] if values else None,
            'valid_subset_max_ns': values[-1] if values else None,
            'valid_subset_p95_nearest_rank_ns': values[(95 * len(values) + 99) // 100 - 1]
            if values else None}


def validate(data):
    require(type(data) is dict, 'capture must be an object')
    for name, value in {'format': 'incant-native-presentations', 'version': 1,
                        'diagnostic_only': True, 'display_gate': False,
                        'uncapped_gpu_throughput': False, 'final_process_exit_status': None,
                        'host_timing_contract': CONTRACT, 'limits': LIMITS,
                        'boundary_bit_order': BOUNDARIES, 'failure_index_order': FAILURES}.items():
        require(name in data, f'missing {name}')
        exact(data[name], value, name)
    for name in ('requested_host_steps', 'capture_capacity'):
        uint(data[name], name, MAX_ROWS)
        require(data[name] > 0, f'{name}: zero')
    host, rows, coverage = data['host'], data['rows'], data['coverage']
    keys(host, 'attempts segments boundary_counts acquisition_failures omitted_attempts exhausted timing_errors', 'host')
    uint(host['omitted_attempts'], 'omitted_attempts')
    uint(host['timing_errors'], 'timing_errors')
    require(type(host['exhausted']) is bool, 'exhausted: boolean required')
    for name, count in [('boundary_counts', 11), ('acquisition_failures', 9)]:
        array(host[name], count, name)
        require(len(host[name]) == count, f'{name}: length')
        for value in host[name]:
            uint(value, name)
    array(host['attempts'], data['capture_capacity'], 'attempts')
    array(host['segments'], 2 * data['capture_capacity'], 'segments')
    array(rows, data['capture_capacity'], 'rows')
    for i, a in enumerate(host['attempts'], 1):
        keys(a, 'id role session_generation requested_host_step status host_timing', 'attempt')
        for field in ('id', 'session_generation', 'requested_host_step'):
            uint(a[field], field)
        require(a['id'] == i and a['session_generation'] > 0, 'attempt identity/order')
        require(a['role'] in ROLES and a['status'] in STATUSES, 'attempt role/status')
        t = a['host_timing']
        keys(t, 'began_ns reserved_ns completed_ns finished_ns schedule', 'host_timing')
        for name in ('began_ns', 'reserved_ns', 'completed_ns', 'finished_ns'):
            uint(t[name], name, nullable=True)
        s = t['schedule']
        if s is not None:
            keys(s, 'pulse_ns deadline_ns forced_resume due ordinal discarded_ticks', 'schedule')
            for name in ('pulse_ns', 'deadline_ns', 'discarded_ticks'):
                uint(s[name], name, nullable=True)
            for name in ('due', 'ordinal'):
                uint(s[name], name, 255)
            require(type(s['forced_resume']) is bool, 'forced_resume: boolean required')
    for i, s in enumerate(host['segments'], 1):
        keys(s, 'id surface_generation session_generation role boundary_bits', 'segment')
        for name in ('id', 'surface_generation', 'session_generation', 'boundary_bits'):
            uint(s[name], name, 2047 if name == 'boundary_bits' else U64)
        require(s['id'] == i and s['surface_generation'] > 0 and
                s['session_generation'] > 0 and s['role'] in ROLES, 'segment identity')
    counts = dict.fromkeys(OUTCOMES.values(), 0)
    transitions, previous = 0, None
    bindings = {a['id']: [] for a in host['attempts']}
    last_attempt = 0
    for row in rows:
        keys(row, 'surface_generation segment requested_host_step attempt drawable_id outcome presented_seconds invalid_bits', 'row')
        for name in ('surface_generation', 'segment', 'requested_host_step', 'attempt', 'drawable_id'):
            uint(row[name], name)
        uint(row['invalid_bits'], 'invalid_bits', nullable=True)
        seconds = row['presented_seconds']
        require(seconds is None or (type(seconds) in (int, float) and math.isfinite(seconds)
                                     and seconds >= 0), 'presented_seconds: invalid')
        require(row['attempt'] in bindings and 1 <= row['segment'] <= len(host['segments']), 'row identity missing')
        a, s = host['attempts'][row['attempt'] - 1], host['segments'][row['segment'] - 1]
        require(row['requested_host_step'] == a['requested_host_step'] and
                row['surface_generation'] == s['surface_generation'] and
                a['session_generation'] == s['session_generation'] and a['role'] == s['role'], 'row identity mismatch')
        require(row['outcome'] in OUTCOMES, 'unknown callback outcome')
        require((seconds is not None and seconds > 0) if row['outcome'] == 'presented'
                else seconds is None, 'callback outcome/time mismatch')
        require((row['invalid_bits'] is not None) == (row['outcome'] == 'invalid'), 'callback invalid bits mismatch')
        require(row['attempt'] >= last_attempt, 'callback acquisition order')
        last_attempt = row['attempt']
        counts[OUTCOMES[row['outcome']]] += 1
        identity = (row['surface_generation'], row['segment'])
        transitions += identity != previous
        previous = identity
        bindings[a['id']].append(row)
    keys(coverage, 'rows segments acquired missing presented zero invalid discarded abandoned overflow protocol_errors', 'coverage')
    for name, value in coverage.items():
        uint(value, f'coverage.{name}')
    require(coverage['rows'] == len(rows) and coverage['segments'] == transitions and
            all(coverage[k] == v for k, v in counts.items()), 'callback coverage mismatch')
    final = data['finalization']
    require(type(final) is dict, 'finalization object required')
    if final.get('stage') == 'abandoned_before_finalization':
        exact(final, {'stage': 'abandoned_before_finalization', 'drained': False}, 'abandoned finalization')
    else:
        keys(final, 'stage event_loop_code accepted_host_steps session_generation returned_report window_error recording_completed assertions_passed unbind_error drain_elapsed_seconds pending_at_last_drain_scan', 'finalization')
        require(final['stage'] == 'window_finalized', 'unknown finalization')
        require(type(final['event_loop_code']) is int and -(1 << 31) <= final['event_loop_code'] < (1 << 31), 'event_loop_code')
        for name in ('accepted_host_steps', 'session_generation', 'pending_at_last_drain_scan'):
            uint(final[name], name)
        require(type(final['returned_report']) is bool, 'returned_report')
        for name in ('recording_completed', 'assertions_passed'):
            require(final[name] is None or type(final[name]) is bool, name)
        for name in ('window_error', 'unbind_error'):
            require(final[name] is None or (type(final[name]) is str and len(final[name]) <= 4096), name)
        require(type(final['drain_elapsed_seconds']) in (int, float) and
                math.isfinite(final['drain_elapsed_seconds']) and final['drain_elapsed_seconds'] >= 0, 'drain elapsed')
    return bindings


def analyze(data):
    bindings = validate(data)
    host = data['host']
    attempts, batches, by_key = [], [], {}
    for a in host['attempts']:
        t, issues = a['host_timing'], []
        gameplay, committed = a['role'] == 'gameplay', a['status'] == 'host_committed'
        s = t['schedule']
        schedule_valid = False
        if s is not None:
            schedule_valid = (gameplay and 1 <= s['due'] <= 4 and s['ordinal'] < s['due'] and
                              s['pulse_ns'] is not None and s['discarded_ticks'] is not None and
                              s['forced_resume'] == (s['deadline_ns'] is None) and
                              t['began_ns'] is not None and s['pulse_ns'] <= t['began_ns'] and
                              (s['deadline_ns'] is None or s['deadline_ns'] <= s['pulse_ns']))
            if not schedule_valid:
                issues.append('invalid_schedule')
            key = (a['session_generation'], s['pulse_ns'], s['deadline_ns'])
            # Missing pulse has no trustworthy batch identity; do not coalesce it.
            if s['pulse_ns'] is None:
                key += (a['id'],)
            if key not in by_key:
                by_key[key] = len(batches)
                batches.append({'session_generation': a['session_generation'], 'schedule': dict(s),
                                'attempt_ids': [], 'ordinals': [], 'valid': True})
            batch = batches[by_key[key]]
            same = all(s[k] == batch['schedule'][k] for k in s if k != 'ordinal')
            ordered = s['ordinal'] == len(batch['ordinals'])
            consecutive = not batch['attempt_ids'] or batch['attempt_ids'][-1] + 1 == a['id']
            batch['valid'] &= schedule_valid and same and ordered and consecutive
            batch['attempt_ids'].append(a['id'])
            batch['ordinals'].append(s['ordinal'])
        elif gameplay:
            issues.append('missing_schedule')
        stages = [t[k] for k in ('began_ns', 'reserved_ns', 'completed_ns', 'finished_ns')]
        known = [value for value in stages if value is not None]
        if known != sorted(known):
            issues.append('clock_regression')
        if (a['status'] == 'open' and (t['completed_ns'] is not None or t['finished_ns'] is not None)) or \
                (t['finished_ns'] is not None and not (gameplay and committed)):
            issues.append('status_stage_conflict')
        metrics = {
            'pre_attempt': delta(s['pulse_ns'] if s else None, t['began_ns'], gameplay),
            'reserve': delta(t['began_ns'], t['reserved_ns'], gameplay),
            'committed_body': delta(t['reserved_ns'], t['completed_ns'], gameplay and committed),
            'post_commit_haptics': delta(t['completed_ns'], t['finished_ns'], gameplay and committed),
            'finished_total': delta(t['began_ns'], t['finished_ns'], gameplay and committed),
        }
        if issues:
            for metric in metrics.values():
                if metric['state'] == 'valid':
                    metric.update(state='invalid', ns=None)
        attempts.append({'raw': a, 'callback_ownership': bindings[a['id']],
                         'issues': issues, 'metrics': metrics})
    for batch in batches:
        s = batch['schedule']
        batch['complete_ordinals'] = batch['valid'] and batch['ordinals'] == list(range(s['due']))
        batch['wake_lateness'] = delta(s['deadline_ns'], s['pulse_ns'], not s['forced_resume'])
        if not batch['valid']:
            batch['wake_lateness'] = {'state': 'invalid', 'ns': None}
            for attempt_id in batch['attempt_ids']:
                attempt = attempts[attempt_id - 1]
                attempt['issues'].append('invalid_scheduler_batch')
                for metric in attempt['metrics'].values():
                    if metric['state'] == 'valid':
                        metric.update(state='invalid', ns=None)
        batch['discarded_ticks_once'] = s['discarded_ticks'] if batch['valid'] else None
    pairs = []
    for before, after in zip(attempts, attempts[1:]):
        a, b = before['raw'], after['raw']
        owned_a, owned_b = before['callback_ownership'], after['callback_ownership']
        eligible = (a['role'] == b['role'] == 'gameplay' and
                    a['status'] == b['status'] == 'host_committed' and
                    b['id'] == a['id'] + 1 and b['requested_host_step'] == a['requested_host_step'] + 1 and
                    a['session_generation'] == b['session_generation'] and
                    before['metrics']['finished_total']['state'] == after['metrics']['finished_total']['state'] == 'valid' and
                    len(owned_a) == len(owned_b) == 1 and
                    owned_a[0]['segment'] == owned_b[0]['segment'] and
                    owned_a[0]['surface_generation'] == owned_b[0]['surface_generation'])
        interval = delta(a['host_timing']['finished_ns'], b['host_timing']['finished_ns'], eligible)
        pairs.append({'before_attempt': a['id'], 'after_attempt': b['id'],
                      'state': interval['state'], 'ns': interval['ns']})
    reasons = []
    committed = [a for a in attempts if a['raw']['role'] == 'gameplay' and a['raw']['status'] == 'host_committed']
    steps = [a['raw']['requested_host_step'] for a in committed]
    if steps != list(range(1, data['requested_host_steps'] + 1)):
        reasons.append('requested_host_steps_not_fully_committed')
    if any(len(a['callback_ownership']) != 1 for a in committed):
        reasons.append('missing_or_ambiguous_committed_acquisition_binding')
    if any(a['issues'] or any(m['state'] in ('missing', 'invalid') for m in a['metrics'].values())
           or a['raw']['status'] == 'open'
           or (a['raw']['role'] == 'gameplay' and a['raw']['status'] != 'host_committed') for a in attempts):
        reasons.append('partial_or_invalid_attempts')
    if any(not b['complete_ordinals'] for b in batches):
        reasons.append('partial_or_invalid_scheduler_batches')
    if host['exhausted'] or host['omitted_attempts'] or host['timing_errors']:
        reasons.append('host_exhaustion_omissions_or_timing_errors')
    if data['coverage']['overflow'] or data['coverage']['protocol_errors']:
        reasons.append('callback_overflow_or_protocol_errors')
    if any(p['state'] == 'invalid' for p in pairs):
        reasons.append('inter_attempt_clock_regression')
    final = data['finalization']
    if not (final.get('stage') == 'window_finalized' and final['event_loop_code'] == 0 and
            final['returned_report'] and final['window_error'] is None and final['unbind_error'] is None and
            final['accepted_host_steps'] == data['requested_host_steps'] and
            final['recording_completed'] is not False and final['assertions_passed'] is not False):
        reasons.append('incomplete_or_failed_finalization')
    return {
        'format': 'incant-host-stage-analysis', 'version': 1, 'diagnostic_only': True,
        'performance_gate_passed': False, 'display_cadence_measured': False,
        'external_process_and_gameplay_proof_required': True,
        'complete_requested_host_timing': not reasons, 'partial_reasons': reasons,
        'requested_host_steps': data['requested_host_steps'], 'capture_capacity': data['capture_capacity'],
        'host_metadata': {k: v for k, v in host.items() if k != 'attempts'},
        'attempt_status_counts': {status: sum(a['raw']['status'] == status for a in attempts)
                                  for status in STATUSES},
        'schedule_counts': {'present': sum(a['raw']['host_timing']['schedule'] is not None for a in attempts),
                            'missing_gameplay': sum(a['raw']['role'] == 'gameplay' and
                                a['raw']['host_timing']['schedule'] is None for a in attempts),
                            'invalid_batches': sum(not b['valid'] for b in batches)},
        'callback_coverage': data['coverage'], 'finalization': final,
        'attempts': attempts, 'scheduler_batches': batches, 'adjacent_host_finish_intervals': pairs,
        'summary': {**{name: summarize([a['metrics'][name] for a in attempts])
                      for name in ('pre_attempt', 'reserve', 'committed_body', 'post_commit_haptics', 'finished_total')},
                    'batch_wake_lateness': summarize([b['wake_lateness'] for b in batches]),
                    'adjacent_host_finish_intervals': summarize(pairs)},
        'discarded_ticks': {'valid_batches_sum': sum(b['discarded_ticks_once'] for b in batches if b['valid']),
                            'valid_batches': sum(b['valid'] for b in batches),
                            'invalid_batches': sum(not b['valid'] for b in batches)},
        'limitations': [
            'Only one Rust Instant epoch; Metal values are retained as raw ownership evidence only.',
            'Wake lateness is per scheduler batch; one shared deadline is not an ordinal target.',
            'Pre-attempt work includes polling, eligibility, refresh and earlier catch-up work; spans overlap.',
            'Valid-subset quantiles exclude explicitly counted missing/invalid stages; no all-frame gate.',
            'Due-zero/inactive pulses have no attempt; host finish excludes later event-loop cleanup.',
            'Native callback registration has overhead; JSON bound is not an allocation/RSS bound.',
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=Path)
    parser.add_argument('--max-input-mib', type=int, choices=(32, 64), default=64)
    args = parser.parse_args()
    try:
        data, digest = read_capture(args.capture, args.max_input_mib)
        result = analyze(data)
        result['input_sha256'] = digest
        encoded = json.dumps(result, separators=(',', ':'), allow_nan=False).encode()
        require(len(encoded) + 1 <= MAX_BYTES, 'analysis exceeds 64 MiB output bound')
        sys.stdout.buffer.write(encoded + b'\n')
    except (ValueError, KeyError, TypeError, OverflowError, RecursionError, OSError) as error:
        print(json.dumps({'outcome': 'refused', 'error': str(error)[:512]}), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
