"""Acceptance/refusal controls for measurement ingestion; no performance claims."""
import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import mac_performance as subject


def row(name, index=100):
    counts = [0] * 10001
    counts[index] = 100
    return {'name': name, 'samples': 100, 'missing_frames': 0, 'invalid_frames': 0,
            'overflow_samples': 0, 'counts': counts,
            'p95_lower_ns': (index - 1) * 10000 + 1, 'p95_upper_ns': index * 10000}


def report():
    return {'tick': 100, 'profile': {'version': 2, 'completed_frames': 100, 'failed_frames': 0,
            'dropped_source_samples': 0, 'tracy_process_transport': False,
            'rust_allocations': {'installed': True, 'saturated': False, 'process_peak_requested_bytes': 1000},
            'frame_wall': {'version': 1, 'unit': 'ns', 'bucket_width_ns': 10000, 'max_ns': 100000000,
                'attempted_frames': 100, 'successful_frames': 100, 'failed_frames': 0, 'invalid_frames': 0,
                'active_frames': 0, 'saturated': False, 'phases': [row(name) for name in subject.PHASES]}}}


def current_report(mode='maintained'):
    result = report()
    detail = mode == 'detailed'
    result['profile']['version'] = 3
    result['profile']['quickjs_heap'] = {
        'version': 1, 'mode': mode, 'sampling': 'script_tick_end_per_vm',
        'samples': 100, 'invalid_samples': 0, 'detailed_samples': 100 if detail else 0,
        'latest': {'allocated_bytes': 32, 'allocation_count': 3, 'limit_bytes': 64,
                   'estimated_used_bytes': 17 if detail else None},
        'sampled_peak_allocated_bytes': 40, 'sampled_peak_allocation_count': 4,
        'sampled_peak_estimated_used_bytes': 20 if detail else None}
    return result


def measurement():
    return {'format': 'incant-mac-cpu-measurement', 'version': 1,
            'match': {'machine': {'model': 'fixture'}, 'ticks': 100, 'script_source_sha256': 'fixture'},
            'metrics': subject.metrics(report(), 100)}


class MacPerformance(unittest.TestCase):
    def test_version_discriminators_reject_boolean_float_text_and_unknown_values(self):
        for value in (True, False, 1.0, 2.0, 3.0, '3', None, 0, 4):
            for path in [('version',), ('quickjs_heap', 'version'), ('frame_wall', 'version')]:
                data = current_report(); node = data['profile']
                for key in path[:-1]: node = node[key]
                node[path[-1]] = value
                with self.subTest(value=value, path=path), self.assertRaises(ValueError):
                    subject.metrics(data, 100)
            current = measurement(); current['version'] = value
            with self.subTest(value=value, route='comparison'), self.assertRaises(ValueError):
                subject.comparison(current, measurement())
        # Genuine supported integer versions remain admitted on both protocols.
        subject.metrics(report(), 100)
        subject.metrics(current_report(), 100)
        self.assertTrue(subject.comparison(measurement(), measurement())['passed'])

    def test_raw_baseline_loader_rejects_coercible_measurement_versions(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            raw = json.dumps(report()).encode()
            (directory / 'stdout.json').write_bytes(raw)
            measured = measurement(); measured['profile_report_sha256'] = subject.digest(raw)
            path = directory / 'measurement.json'
            for value in (True, 1.0, '1', None, 0, 3):
                measured['version'] = value; path.write_text(json.dumps(measured))
                with self.subTest(value=value), self.assertRaises(ValueError): subject.load_baseline(path)

    def test_maintained_and_detailed_instrumentation_are_explicit(self):
        for mode in ('maintained', 'detailed'):
            data = current_report(mode)
            self.assertEqual(subject.metrics(data, 100), subject.metrics(report(), 100))
            self.assertEqual(subject.profile_instrumentation(data)['quickjs_heap_mode'], mode)
        data = current_report()
        for key, value in [('invalid_samples', 1), ('detailed_samples', 1),
                           ('sampled_peak_estimated_used_bytes', 0), ('mode', 'unknown')]:
            bad = copy.deepcopy(data); bad['profile']['quickjs_heap'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): subject.metrics(bad, 100)
        data['profile']['quickjs_heap']['latest']['estimated_used_bytes'] = 0
        with self.assertRaises(ValueError): subject.metrics(data, 100)

    def test_empty_static_vm_observations_are_null_not_zero(self):
        data = current_report(); heap = data['profile']['quickjs_heap']
        heap.update(samples=0, latest=None, sampled_peak_allocated_bytes=None,
                    sampled_peak_allocation_count=None, sampled_peak_estimated_used_bytes=None)
        subject.profile_instrumentation(data)
        heap['sampled_peak_allocated_bytes'] = 0
        with self.assertRaises(ValueError): subject.profile_instrumentation(data)

    def test_cross_instrumentation_and_missing_current_identity_refuse(self):
        legacy = measurement(); current = copy.deepcopy(legacy)
        current['version'] = 2
        current['match']['profile_instrumentation'] = subject.profile_instrumentation(current_report())
        with self.assertRaisesRegex(ValueError, 'instrumentation differs'):
            subject.comparison(current, legacy)
        legacy_explicit = copy.deepcopy(current)
        legacy_explicit['match']['profile_instrumentation'] = subject.profile_instrumentation(report())
        self.assertTrue(subject.comparison(legacy_explicit, legacy)['passed'])
        detailed = copy.deepcopy(current)
        detailed['match']['profile_instrumentation'] = subject.profile_instrumentation(current_report('detailed'))
        with self.assertRaisesRegex(ValueError, 'differs'): subject.comparison(current, detailed)
        self.assertTrue(subject.comparison(current, current)['passed'])
        current['match'].pop('profile_instrumentation')
        with self.assertRaisesRegex(ValueError, 'Missing instrumentation'): subject.comparison(current, current)

    def test_current_baseline_binds_raw_instrumentation_and_refuses_relabeling(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            raw = json.dumps(current_report()).encode()
            (directory / 'stdout.json').write_bytes(raw)
            measured = measurement(); measured['version'] = 2
            measured['profile_report_sha256'] = subject.digest(raw)
            measured['match']['profile_instrumentation'] = subject.profile_instrumentation(current_report())
            path = directory / 'measurement.json'; path.write_text(json.dumps(measured))
            self.assertEqual(subject.load_baseline(path), measured)
            measured['match']['profile_instrumentation']['quickjs_heap_mode'] = 'detailed'
            path.write_text(json.dumps(measured))
            with self.assertRaisesRegex(ValueError, 'instrumentation identity'): subject.load_baseline(path)
            measured['version'] = 1
            path.write_text(json.dumps(measured))
            with self.assertRaisesRegex(ValueError, 'Legacy measurement'): subject.load_baseline(path)

    def test_static_phase_requires_wasm_and_distinct_complete_histogram(self):
        subject.phase_selection('script.tick', None)
        subject.phase_selection('script.static_module', {'kind': 'wasm'})
        for phase, program in [('script.tick', {'kind': 'wasm'}), ('script.static_module', None),
                               ('script.unknown', None), ('script.tick', {'kind': 'unknown'})]:
            with self.subTest(phase=phase, program=program), self.assertRaises(ValueError):
                subject.phase_selection(phase, program)
        data = report()
        with self.assertRaisesRegex(ValueError, 'Missing'):
            subject.metrics(data, 100, script_phase='script.static_module')
        data['profile']['frame_wall']['phases'][1]['name'] = 'script.static_module'
        self.assertIn('script.static_module.p95_wall_ns', subject.metrics(data, 100, script_phase='script.static_module'))
        data['profile']['frame_wall']['phases'][1]['missing_frames'] = 1
        with self.assertRaisesRegex(ValueError, 'Incomplete'):
            subject.metrics(data, 100, script_phase='script.static_module')

    def test_cross_phase_comparison_never_becomes_ts_regression_acceptance(self):
        baseline = measurement(); current = copy.deepcopy(baseline)
        current['match']['script_phase'] = 'script.static_module'
        current['script_phase'] = 'script.static_module'
        with self.assertRaisesRegex(ValueError, 'differs'): subject.comparison(current, baseline)
        with self.assertRaisesRegex(ValueError, 'TypeScript regression'):
            subject.comparison(current, current)

    def test_report_phase_cannot_disagree_with_match_identity(self):
        current = measurement(); current['script_phase'] = 'script.static_module'
        with self.assertRaisesRegex(ValueError, 'phase disagrees'):
            subject.comparison(current, current)

    def test_actual_nearest_rank_and_conservative_interval(self):
        data = row('script.tick')
        data['counts'][100] = 94
        data['counts'][200] = 6
        data.update(p95_lower_ns=1990001, p95_upper_ns=2000000)
        self.assertEqual(subject.interval(data, 100), {'lower': 1990001, 'upper': 2000000})
        data['p95_upper_ns'] -= 1
        with self.assertRaisesRegex(ValueError, 'disagrees'):
            subject.interval(data, 100)

    def test_incomplete_failed_overflow_and_missing_phase_never_pass(self):
        for field in ('failed_frames', 'invalid_frames', 'active_frames'):
            data = report(); data['profile']['frame_wall'][field] = 1
            with self.subTest(field=field), self.assertRaises(ValueError): subject.metrics(data, 100)
        for field in ('missing_frames', 'invalid_frames', 'overflow_samples'):
            data = report(); data['profile']['frame_wall']['phases'][1][field] = 1
            with self.subTest(field=field), self.assertRaises(ValueError): subject.metrics(data, 100)
        data = report(); data['profile']['frame_wall']['phases'].pop()
        with self.assertRaisesRegex(ValueError, 'Missing'): subject.metrics(data, 100)
        data = report(); data['profile']['version'] = 1
        with self.assertRaisesRegex(ValueError, 'unsupported'): subject.metrics(data, 100)

    def test_malformed_and_duplicate_histograms_refuse(self):
        for count in (-1, True, 1.0, 2**64):
            data = report(); data['profile']['frame_wall']['phases'][0]['counts'][0] = count
            with self.subTest(count=count), self.assertRaises(ValueError): subject.metrics(data, 100)
        data = report(); data['profile']['frame_wall']['phases'].append(row('script.tick'))
        with self.assertRaisesRegex(ValueError, 'Duplicate'): subject.metrics(data, 100)
        data = report(); data['profile']['frame_wall']['phases'][0]['counts'][100] -= 1
        with self.assertRaisesRegex(ValueError, 'coverage'): subject.metrics(data, 100)

    def test_baseline_mismatch_in_machine_or_script_refuses(self):
        current = measurement()
        for key in ('machine', 'script_source_sha256', 'ticks'):
            prior = copy.deepcopy(current); prior['match'][key] = 'different'
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'differs'):
                subject.comparison(current, prior)

    def test_regression_limit_handles_uncertainty_without_false_pass(self):
        current = measurement(); prior = copy.deepcopy(current)
        self.assertTrue(subject.comparison(current, prior)['passed'])
        current['metrics']['player.frame.p95_wall_ns'] = {'lower': 1040001, 'upper': 1050000}
        self.assertEqual(subject.comparison(current, prior)['metrics']['player.frame.p95_wall_ns'], 'inconclusive')
        current['metrics']['player.frame.p95_wall_ns'] = {'lower': 1060001, 'upper': 1070000}
        self.assertEqual(subject.comparison(current, prior)['metrics']['player.frame.p95_wall_ns'], 'regression')
        current['metrics']['player.frame.p95_wall_ns'] = {'lower': 900001, 'upper': 910000}
        self.assertEqual(subject.comparison(current, prior)['metrics']['player.frame.p95_wall_ns'], 'pass')

    def test_script_budget_and_scope_are_distinct_from_phase_gate(self):
        current = measurement(); prior = copy.deepcopy(current)
        current['metrics']['script.tick.p95_wall_ns'] = {'lower': 1990001, 'upper': 2000000}
        result = subject.comparison(current, prior)
        self.assertEqual(result['script_2ms_p95'], 'pass')
        self.assertFalse(result['passed']) # A 2x regression is still rejected.
        self.assertFalse(result['phase_gate_passed'])
        current['metrics']['script.tick.p95_wall_ns'] = {'lower': 2000001, 'upper': 2010000}
        self.assertEqual(subject.comparison(current, prior)['script_2ms_p95'], 'fail')

    def test_non_mac_refuses_before_any_host_command(self):
        with patch.object(subject.platform, 'system', return_value='Linux'), \
                patch.object(subject, 'command_text') as command, self.assertRaises(ValueError):
            subject.mac_identity()
        command.assert_not_called()

    def test_baseline_requires_unchanged_raw_report_and_derived_metrics(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).resolve()
            raw = json.dumps(report()).encode()
            (directory / 'stdout.json').write_bytes(raw)
            measured = measurement(); measured['profile_report_sha256'] = subject.digest(raw)
            path = directory / 'measurement.json'
            path.write_text(json.dumps(measured))
            self.assertEqual(subject.load_baseline(path), measured)
            measured['metrics']['script.tick.p95_wall_ns']['upper'] -= 1
            path.write_text(json.dumps(measured))
            with self.assertRaisesRegex(ValueError, 'Baseline metrics'): subject.load_baseline(path)
            (directory / 'stdout.json').write_bytes(raw + b' ')
            with self.assertRaisesRegex(ValueError, 'identity'): subject.load_baseline(path)


if __name__ == '__main__':
    unittest.main()
