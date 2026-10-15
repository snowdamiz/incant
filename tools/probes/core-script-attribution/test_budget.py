"""Pure-data private-builder/analysis controls; no Cargo, guest or performance run."""
import copy
from pathlib import Path
import unittest

from analyze import analyze
from budget_analysis import REGIONS
from common import (BUDGET, BUDGET_FEATURES, DIAGNOSTIC, EXTRA, TARGET, TICKS,
                    additions, compiler_features, graph_delta)
from test_protocol import report


def budget_report():
    data, sources = report()
    d = data['profile']['script_phase_diagnostic']
    d.update(schema=3, budget_check_regions=REGIONS, raw_row_bytes=912, raw_storage_bound_bytes=4096 * 912)
    for row in d['rows']:
        row['budget_checks'] = {'ns': [10, 5, 0, 10], 'calls': [1, 2, 0, 3],
                                'clock_reads': [1, 2, 0, 3], 'refused': [0] * 4}
    return data, sources


class BudgetProtocol(unittest.TestCase):
    def test_budget_schema3_totals_and_same_frame_selection_remain_observations(self):
        data, sources = budget_report()
        result = analyze(data, sources, budget=True)
        counts = result['budget_checks']
        self.assertEqual(counts['regions'], REGIONS)
        self.assertEqual(counts['all_frames']['calls'], [TICKS, 2 * TICKS, 0, 3 * TICKS])
        self.assertEqual(counts['all_frames']['ns'], [10 * TICKS, 5 * TICKS, 0, 10 * TICKS])
        self.assertEqual(counts['same_frame_tail']['frames'], list(range(1, 109)))
        self.assertEqual(counts['same_frame_tail']['wall_ns_per_check'][1],
                         {'numerator_ns': 108 * 5, 'denominator_checks': 108 * 2})
        self.assertIsNone(counts['all_frames']['wall_ns_per_check'][2])
        self.assertFalse(result['eligible_for_performance_gate'])
        with self.assertRaises(ValueError): analyze(data, sources)  # no schema bridge
        with self.assertRaises(ValueError): analyze(data, sources, navigation=True, budget=True)
        data['profile']['script_phase_diagnostic']['schema'] = 1
        with self.assertRaises(ValueError): analyze(data, sources)  # cannot relabel budget fields

    def test_budget_refuses_wrong_schema_regions_counts_clock_coverage_or_parent(self):
        original, sources = budget_report()
        for mutate in (
            lambda d: d.update(schema=1), lambda d: d.update(schema=4),
            lambda d: d.update(budget_check_regions=list(reversed(REGIONS))),
            lambda d: d.update(navigation_regions=[]),
            lambda d: d['rows'][0].update(navigation={}),
            lambda d: d['rows'][0]['budget_checks'].update(extra=0),
            lambda d: d['rows'][0]['budget_checks'].update(ns=[0]),
            lambda d: d['rows'][0]['budget_checks']['calls'].__setitem__(0, True),
            lambda d: d['rows'][0]['budget_checks']['calls'].__setitem__(0, -1),
            lambda d: d['rows'][0]['budget_checks']['calls'].__setitem__(0, 2**32),
            lambda d: d['rows'][0]['budget_checks']['clock_reads'].__setitem__(1, 1),
            lambda d: d['rows'][0]['budget_checks']['refused'].__setitem__(0, 1),
            lambda d: d['rows'][0]['budget_checks']['ns'].__setitem__(0, 2**64),
            lambda d: d['rows'][0]['budget_checks']['ns'].__setitem__(2, 1),
            lambda d: d['rows'][0]['budget_checks']['ns'].__setitem__(0, 51),
            lambda d: d['rows'][0]['budget_checks']['ns'].__setitem__(3, 1400),
        ):
            data = copy.deepcopy(original); mutate(data['profile']['script_phase_diagnostic'])
            with self.assertRaises(ValueError): analyze(data, sources, budget=True)
        for index in (0, 1):
            data, sources = budget_report()
            for row in data['profile']['script_phase_diagnostic']['rows']:
                for field in ('ns', 'calls', 'clock_reads'):
                    row['budget_checks'][field][index] = 0
            with self.assertRaises(ValueError): analyze(data, sources, budget=True)

    def test_exact_budget_receivers_reject_missing_extra_and_runtime_budget_flags(self):
        extra = additions(BUDGET_FEATURES)
        self.assertEqual(set(extra), EXTRA)
        self.assertEqual(extra['incant_runtime'], {DIAGNOSTIC})
        for name in EXTRA - {'incant_runtime'}: self.assertEqual(extra[name], {DIAGNOSTIC, BUDGET})
        base = '\n'.join(f'{name} v0.0.1|profile' for name in sorted(EXTRA))
        actual = '\n'.join(f'{name} v0.0.1|' + ','.join(sorted({'profile'} | extra[name])) for name in sorted(EXTRA))
        self.assertEqual(set(graph_delta(actual, base, BUDGET_FEATURES)), EXTRA)
        for wrong in (actual.replace(BUDGET, 'unknown', 1),
                      actual.replace('incant_runtime v0.0.1|', 'incant_runtime v0.0.1|' + BUDGET + ',')):
            with self.assertRaises(ValueError): graph_delta(wrong, base, BUDGET_FEATURES)
        with self.assertRaises(ValueError): additions(BUDGET_FEATURES + ['diagnostic-navigation'])

        root = Path('/private/budget-compiler-fixture')
        baseline = {name: ['profile'] for name in EXTRA}
        messages = [{'reason': 'build-finished', 'success': True}]
        for name in EXTRA:
            for kind in (['lib', 'bin'] if name == 'incant_game_player' else ['lib']):
                messages.append({'reason': 'compiler-artifact', 'target': {'name': name, 'kind': [kind]},
                    'filenames': [str(root / TARGET / 'shipping' / (name + '.' + kind))],
                    'features': sorted({'profile'} | extra[name])})
        actual = compiler_features(messages, root, baseline, BUDGET_FEATURES)
        self.assertEqual(actual['extra_feature'], BUDGET)
        for i, message in enumerate(messages[1:], 1):
            if message['target']['name'] == 'incant_runtime':
                bad = copy.deepcopy(messages); bad[i]['features'].append(BUDGET)
                with self.assertRaises(ValueError): compiler_features(bad, root, baseline, BUDGET_FEATURES)


if __name__ == '__main__':
    unittest.main()
