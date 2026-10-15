import copy
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import presentation_host_stages as audit


def capture(count=2):
    attempts, rows = [], []
    for i in range(1, count + 1):
        start = 1000 * i
        attempts.append({'id': i, 'role': 'gameplay', 'session_generation': 1,
                         'requested_host_step': i, 'status': 'host_committed',
                         'host_timing': {'began_ns': start + 20, 'reserved_ns': start + 30,
                                         'completed_ns': start + 60, 'finished_ns': start + 70,
                                         'schedule': {'pulse_ns': start, 'deadline_ns': start - 5,
                                                      'forced_resume': False, 'due': 1, 'ordinal': 0,
                                                      'discarded_ticks': 0}}})
        rows.append({'surface_generation': 1, 'segment': 1, 'requested_host_step': i,
                     'attempt': i, 'drawable_id': i, 'outcome': 'presented',
                     'presented_seconds': i * 0.016, 'invalid_bits': None})
    data = {'format': 'incant-native-presentations', 'version': 1, 'diagnostic_only': True,
            'display_gate': False, 'uncapped_gpu_throughput': False,
            'final_process_exit_status': None, 'host_timing_contract': copy.deepcopy(audit.CONTRACT),
            'limits': dict(audit.LIMITS), 'boundary_bit_order': list(audit.BOUNDARIES),
            'failure_index_order': list(audit.FAILURES), 'requested_host_steps': count,
            'capture_capacity': 8,
            'host': {'attempts': attempts, 'segments': [{'id': 1, 'surface_generation': 1,
                     'session_generation': 1, 'role': 'gameplay', 'boundary_bits': 1}],
                     'boundary_counts': [1] + [0] * 10, 'acquisition_failures': [0] * 9,
                     'omitted_attempts': 0, 'exhausted': False, 'timing_errors': 0},
            'rows': rows, 'finalization': {'stage': 'window_finalized', 'event_loop_code': 0,
                     'accepted_host_steps': count, 'session_generation': 1, 'returned_report': True,
                     'window_error': None, 'recording_completed': True, 'assertions_passed': True,
                     'unbind_error': None, 'drain_elapsed_seconds': 0.01, 'pending_at_last_drain_scan': 0}}
    recount(data)
    return data


def recount(data):
    coverage = dict.fromkeys(audit.OUTCOMES.values(), 0)
    transitions, previous = 0, None
    for row in data['rows']:
        coverage[audit.OUTCOMES[row['outcome']]] += 1
        identity = (row['surface_generation'], row['segment'])
        transitions += identity != previous
        previous = identity
    data['coverage'] = {**coverage, 'rows': len(data['rows']), 'segments': transitions,
                        'overflow': 0, 'protocol_errors': 0}


def catchup(data):
    first, second = data['host']['attempts'][:2]
    first['host_timing']['schedule'].update(due=2, discarded_ticks=3)
    second['host_timing']['schedule'] = {**first['host_timing']['schedule'], 'ordinal': 1}


class HostStages(unittest.TestCase):
    def test_stage_arithmetic_and_complete_host_coverage_do_not_claim_gate(self):
        result = audit.analyze(capture())
        self.assertTrue(result['complete_requested_host_timing'])
        self.assertFalse(result['performance_gate_passed'])
        self.assertFalse(result['display_cadence_measured'])
        self.assertTrue(result['external_process_and_gameplay_proof_required'])
        expected = {'pre_attempt': 20, 'reserve': 10, 'committed_body': 30,
                    'post_commit_haptics': 10, 'finished_total': 50}
        for metric, value in expected.items():
            self.assertEqual(result['attempts'][0]['metrics'][metric], {'state': 'valid', 'ns': value})
            self.assertEqual(result['summary'][metric]['valid'], 2)
        self.assertEqual(result['adjacent_host_finish_intervals'][0]['ns'], 1000)

    def test_catchup_uses_one_deadline_and_discard_count_with_raw_ordinals(self):
        data = capture()
        catchup(data)
        result = audit.analyze(data)
        self.assertEqual(len(result['scheduler_batches']), 1)
        batch = result['scheduler_batches'][0]
        self.assertEqual(batch['ordinals'], [0, 1])
        self.assertEqual(batch['attempt_ids'], [1, 2])
        self.assertEqual(batch['wake_lateness'], {'state': 'valid', 'ns': 5})
        self.assertEqual(result['discarded_ticks']['valid_batches_sum'], 3)
        self.assertEqual(result['summary']['batch_wake_lateness']['valid'], 1)
        self.assertEqual(result['attempts'][1]['metrics']['pre_attempt']['ns'], 1020)
        self.assertEqual(result['attempts'][1]['raw'], data['host']['attempts'][1])

    def test_inconsistent_or_repeated_ordinals_invalidate_whole_batch(self):
        for name, value in [('ordinal', 0), ('discarded_ticks', 9), ('due', 3)]:
            data = capture()
            catchup(data)
            data['host']['attempts'][1]['host_timing']['schedule'][name] = value
            with self.subTest(name=name):
                result = audit.analyze(data)
                self.assertFalse(result['scheduler_batches'][0]['valid'])
                self.assertEqual(result['discarded_ticks']['valid_batches_sum'], 0)
                self.assertEqual(result['discarded_ticks']['invalid_batches'], 1)
                self.assertEqual(result['summary']['finished_total']['invalid'], 2)
                self.assertEqual(result['adjacent_host_finish_intervals'][0]['state'], 'not_applicable')
                self.assertFalse(result['complete_requested_host_timing'])

    def test_truncated_catchup_is_partial_but_known_discarded_count_once(self):
        data = capture(1)
        data['host']['attempts'][0]['host_timing']['schedule'].update(due=4, discarded_ticks=7)
        result = audit.analyze(data)
        self.assertTrue(result['scheduler_batches'][0]['valid'])
        self.assertFalse(result['scheduler_batches'][0]['complete_ordinals'])
        self.assertEqual(result['discarded_ticks']['valid_batches_sum'], 7)
        self.assertFalse(result['complete_requested_host_timing'])

    def test_batch_identity_includes_session_and_missing_pulse_is_not_coalesced(self):
        data = capture()
        catchup(data)
        second = data['host']['attempts'][1]
        second['session_generation'] = 2
        second['host_timing']['schedule']['ordinal'] = 0
        data['host']['segments'].append({**data['host']['segments'][0], 'id': 2,
                                         'session_generation': 2, 'boundary_bits': 1 << 8})
        data['rows'][1]['segment'] = 2
        recount(data)
        self.assertEqual(len(audit.analyze(data)['scheduler_batches']), 2)
        for a in data['host']['attempts']:
            a['host_timing']['schedule']['pulse_ns'] = None
        self.assertEqual(len(audit.analyze(data)['scheduler_batches']), 2)

    def test_partial_attempts_keep_unreached_stages_and_break_cadence(self):
        for status in ('open', 'deferred', 'host_committed'):
            data = capture(3)
            middle = data['host']['attempts'][1]
            middle['status'] = status
            middle['host_timing']['finished_ns'] = None
            if status != 'host_committed':
                middle['host_timing']['reserved_ns'] = None
            if status == 'open':
                middle['host_timing']['completed_ns'] = None
            with self.subTest(status=status):
                result = audit.analyze(data)
                self.assertFalse(result['complete_requested_host_timing'])
                self.assertEqual(result['summary']['adjacent_host_finish_intervals']['valid'], 0)
                self.assertIsNone(result['attempts'][1]['raw']['host_timing']['finished_ns'])
                if status == 'host_committed':
                    self.assertEqual(result['summary']['finished_total']['missing'], 1)

    def test_segment_surface_and_host_step_gaps_are_not_joined(self):
        for change in ('segment', 'surface', 'step', 'missing_row', 'two_rows'):
            data = capture(3)
            if change in ('segment', 'surface'):
                data['host']['segments'].append({**data['host']['segments'][0], 'id': 2,
                    'surface_generation': 2 if change == 'surface' else 1, 'boundary_bits': 1 << 1})
                for row in data['rows'][1:]:
                    row['segment'] = 2
                    row['surface_generation'] = data['host']['segments'][1]['surface_generation']
            elif change == 'step':
                data['host']['attempts'][1]['requested_host_step'] = 7
                data['rows'][1]['requested_host_step'] = 7
            elif change == 'missing_row':
                data['rows'].pop(1)
            else:
                data['rows'].insert(2, {**data['rows'][1], 'drawable_id': 99})
            recount(data)
            result = audit.analyze(data)
            with self.subTest(change=change):
                self.assertEqual(result['adjacent_host_finish_intervals'][0]['state'], 'not_applicable')
                if change not in ('segment', 'surface'):
                    self.assertEqual(result['adjacent_host_finish_intervals'][1]['state'], 'not_applicable')

    def test_forced_resume_and_exact_zero_clock_values(self):
        data = capture(1)
        t = data['host']['attempts'][0]['host_timing']
        t.update(began_ns=0, reserved_ns=0, completed_ns=0, finished_ns=0)
        t['schedule'].update(pulse_ns=0, deadline_ns=None, forced_resume=True)
        result = audit.analyze(data)
        self.assertEqual(result['summary']['batch_wake_lateness']['not_applicable'], 1)
        self.assertEqual(result['summary']['finished_total']['valid_subset_sum_ns'], 0)
        self.assertTrue(result['complete_requested_host_timing'])

    def test_clock_regression_and_invalid_schedule_are_explicit(self):
        for field, value in [('reserved_ns', 1019), ('finished_ns', 0)]:
            data = capture(1)
            data['host']['attempts'][0]['host_timing'][field] = value
            result = audit.analyze(data)
            self.assertIn('clock_regression', result['attempts'][0]['issues'])
            self.assertEqual(result['summary']['finished_total']['invalid'], 1)
            self.assertFalse(result['complete_requested_host_timing'])
        data = capture(1)
        data['host']['attempts'][0]['host_timing']['schedule']['deadline_ns'] = 9999
        result = audit.analyze(data)
        self.assertIn('invalid_schedule', result['attempts'][0]['issues'])
        self.assertEqual(result['summary']['batch_wake_lateness']['invalid'], 1)

    def test_missing_schedule_and_status_conflicts_are_partial(self):
        data = capture(1)
        data['host']['attempts'][0]['host_timing']['schedule'] = None
        result = audit.analyze(data)
        self.assertEqual(result['attempts'][0]['issues'], ['missing_schedule'])
        self.assertFalse(result['complete_requested_host_timing'])
        data = capture(1)
        data['host']['attempts'][0]['status'] = 'open'
        self.assertIn('status_stage_conflict', audit.analyze(data)['attempts'][0]['issues'])

    def test_limits_errors_and_final_failure_never_become_complete(self):
        for section, field, value in [('host', 'exhausted', True), ('host', 'omitted_attempts', 1),
                                     ('host', 'timing_errors', 1), ('coverage', 'overflow', 1),
                                     ('coverage', 'protocol_errors', 1), ('finalization', 'window_error', 'failure'),
                                     ('finalization', 'returned_report', False), ('finalization', 'event_loop_code', 1)]:
            data = capture()
            data[section][field] = value
            with self.subTest(field=field):
                result = audit.analyze(data)
                self.assertFalse(result['complete_requested_host_timing'])
                self.assertTrue(result['partial_reasons'])
        data['finalization'] = {'stage': 'abandoned_before_finalization', 'drained': False}
        self.assertFalse(audit.analyze(data)['complete_requested_host_timing'])

    def test_unknown_contract_and_bad_numbers_refuse(self):
        for section, field, value in [(None, 'version', True), (None, 'version', 2),
                ('host_timing_contract', 'version', 2), ('host_timing_contract', 'unit', 'seconds'),
                ('host_timing_contract', 'shared_epoch_with_metal', True),
                (None, 'capture_capacity', 0), (None, 'capture_capacity', 36_001),
                ('coverage', 'rows', 99), ('host', 'timing_errors', -1)]:
            data = capture()
            (data if section is None else data[section])[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                audit.analyze(data)
        for value in (True, -1, 1.5, audit.U64 + 1, float('nan'), float('inf')):
            data = capture()
            data['host']['attempts'][0]['host_timing']['began_ns'] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                audit.analyze(data)

    def test_malformed_identity_arrays_and_callbacks_refuse(self):
        for mutation in ('attempt_id', 'row_id', 'row_step', 'capacity', 'unknown_outcome', 'segment_bits'):
            data = capture()
            if mutation == 'attempt_id': data['host']['attempts'][0]['id'] = 2
            if mutation == 'row_id': data['rows'][0]['attempt'] = 8
            if mutation == 'row_step': data['rows'][0]['requested_host_step'] = 8
            if mutation == 'capacity': data['capture_capacity'] = 1
            if mutation == 'unknown_outcome': data['rows'][0]['outcome'] = 'okay'
            if mutation == 'segment_bits': data['host']['segments'][0]['boundary_bits'] = 2048
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                audit.analyze(data)

    def test_metal_epoch_changes_do_not_change_host_metrics(self):
        data = capture()
        before = audit.analyze(data)
        data['rows'][0]['presented_seconds'] = 9e100
        data['rows'][1]['presented_seconds'] = None
        data['rows'][1]['outcome'] = 'reported_zero'
        recount(data)
        after = audit.analyze(data)
        self.assertEqual(before['summary'], after['summary'])
        self.assertEqual(after['callback_coverage']['zero'], 1)
        self.assertFalse(after['display_cadence_measured'])

    def test_reader_bounds_duplicate_keys_and_nonfinite_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'capture.json'
            for text in ('{"version":1,"version":1}', '{"x":NaN}', '{"x":Infinity}'):
                path.write_text(text)
                with self.assertRaises(ValueError):
                    audit.read_capture(path)
            path.write_text(json.dumps(capture()))
            data, digest = audit.read_capture(path, 32)
            self.assertEqual(data, capture())
            self.assertEqual(len(digest), 64)
            with self.assertRaises(ValueError):
                audit.read_capture(path, 16)
        # The public reader only admits32/64MiB; exercise its exact primitive
        # with a small ceiling, including a prefix that would otherwise parse.
        stream = io.BytesIO(b'{}' + b' ' * 15)
        with patch.object(Path, 'open', return_value=stream), self.assertRaisesRegex(ValueError, 'byte bound'):
            audit.read_bounded('/unused', 16)
        with patch.object(audit, 'read_bounded', return_value=b'{}') as read:
            self.assertEqual(audit.read_capture('/unused', 64)[0], {})
            read.assert_called_once_with('/unused', 64 * 1024 * 1024)

    def test_inter_attempt_clock_regression_invalidates_cadence_not_stage_duration(self):
        data = capture()
        t = data['host']['attempts'][1]['host_timing']
        for key in ('began_ns', 'reserved_ns', 'completed_ns', 'finished_ns'):
            t[key] -= 1500
        t['schedule']['pulse_ns'] -= 1500
        t['schedule']['deadline_ns'] -= 1500
        result = audit.analyze(data)
        self.assertEqual(result['summary']['finished_total']['valid'], 2)
        self.assertEqual(result['summary']['adjacent_host_finish_intervals']['invalid'], 1)
        self.assertIn('inter_attempt_clock_regression', result['partial_reasons'])

    def test_non_gameplay_refresh_breaks_intervals_and_is_not_a_committed_tick(self):
        data = capture(3)
        a = data['host']['attempts'][1]
        a.update(role='ui_resize', status='refresh_completed')
        a['host_timing'].update(finished_ns=None, schedule=None)
        data['host']['segments'].extend([
            {**data['host']['segments'][0], 'id': 2, 'role': 'ui_resize', 'boundary_bits': 1 << 9},
            {**data['host']['segments'][0], 'id': 3, 'boundary_bits': 1 << 9}])
        data['rows'][1]['segment'] = 2
        data['rows'][2]['segment'] = 3
        recount(data)
        result = audit.analyze(data)
        self.assertEqual(result['summary']['finished_total']['not_applicable'], 1)
        self.assertEqual(result['summary']['adjacent_host_finish_intervals']['valid'], 0)
        self.assertEqual(result['attempt_status_counts']['refresh_completed'], 1)

    def test_callback_outcome_payload_contradictions_refuse(self):
        for outcome, seconds, bits in [('reported_zero', 0, None), ('presented', None, None),
                                       ('presented', 0, None), ('invalid', None, None),
                                       ('discarded', None, 123)]:
            data = capture()
            data['rows'][0].update(outcome=outcome, presented_seconds=seconds, invalid_bits=bits)
            recount(data)
            with self.subTest(outcome=outcome, seconds=seconds, bits=bits), self.assertRaises(ValueError):
                audit.analyze(data)


if __name__ == '__main__':
    unittest.main()
