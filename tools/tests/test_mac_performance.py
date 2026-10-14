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


def measurement():
    return {'format': 'incant-mac-cpu-measurement', 'version': 1,
            'match': {'machine': {'model': 'fixture'}, 'ticks': 100, 'script_source_sha256': 'fixture'},
            'metrics': subject.metrics(report(), 100)}


class MacPerformance(unittest.TestCase):
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
