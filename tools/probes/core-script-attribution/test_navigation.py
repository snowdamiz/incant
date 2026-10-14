"""Data-only diagnostic admission and same-frame feasibility controls."""
import copy
from pathlib import Path
import unittest

from analyze import analyze
from common import (EXTRA, NAV_FEATURES, TARGET, additions, compiler_features,
                    graph_delta)
from navigation_analysis import REGIONS, analyze_navigation
from test_protocol import report


def navigation_report():
    raw, sources = report()
    d = raw['profile']['script_phase_diagnostic']
    d.update(schema=2, navigation_regions=REGIONS)
    for row in d['rows']:
        row['navigation'] = {'ns': [30, 10, 10], 'calls': [2, 1, 1], 'failed': [1, 0, 0]}
    return raw, sources


class NavigationProtocol(unittest.TestCase):
    def test_opt_in_reports_exact_counts_and_retains_same_frame_rows(self):
        raw, sources = navigation_report()
        result = analyze(raw, sources, navigation=True)['navigation']
        self.assertFalse(result['eligible_for_performance_gate'])
        self.assertEqual(result['totals']['calls'], [4312, 2156, 2156])
        self.assertEqual(result['zero_cost_same_frame_p95_ns'], 990)
        self.assertEqual(result['rows'][10], {'frame': 11, 'whole_tick_ns': 1000,
                         'repeated_start_ns': 10, 'zero_cost_repeated_start_ns': 990})

    def test_schema_selection_cannot_silently_omit_navigation(self):
        raw, sources = navigation_report()
        with self.assertRaises(ValueError): analyze(raw, sources)
        raw, sources = report()
        with self.assertRaises(ValueError): analyze(raw, sources, navigation=True)

    def test_missing_invalid_or_uncontained_navigation_is_refused(self):
        original, sources = navigation_report()
        for key, value in [('calls', [1, 2, 0]), ('calls', [1, 1, 64]),
                           ('failed', [3, 0, 0]), ('ns', [30, 20, 20]),
                           ('ns', [51, 10, 10]), ('calls', [0, 0, 0]),
                           ('ns', [True, 0, 0]), ('failed', [0, 0, 65536])]:
            raw = copy.deepcopy(original)
            raw['profile']['script_phase_diagnostic']['rows'][0]['navigation'][key] = value
            with self.assertRaises(ValueError): analyze(raw, sources, navigation=True)

    def test_counterfactual_is_ranked_after_same_frame_subtraction(self):
        d = {'schema': 2, 'navigation_regions': REGIONS, 'rows': []}
        for frame, whole, repeated in [(1, 2_100_000, 300_000),
                                        (2, 2_050_000, 100_000), (3, 1_900_000, 50_000)]:
            d['rows'].append({'frame': frame, 'whole_tick_ns': whole, 'source_ns': [whole],
                             'navigation': {'ns': [repeated+2, 1, repeated],
                                            'calls': [1, 1, 1], 'failed': [0, 0, 0]}})
        got = analyze_navigation(d)
        self.assertEqual(got['zero_cost_same_frame_p95_ns'], 1_950_000)
        self.assertFalse(got['stop_hypothesis'])
        d['rows'][0]['whole_tick_ns'] = 2_400_000
        d['rows'][0]['source_ns'] = [2_400_000]
        self.assertTrue(analyze_navigation(d)['stop_hypothesis'])

    def test_zero_repeated_calls_stop_even_if_instrumented_tick_is_fast(self):
        raw, _ = navigation_report()
        d = raw['profile']['script_phase_diagnostic']
        for row in d['rows']:
            row['navigation'] = {'ns': [20, 10, 0], 'calls': [1, 1, 0], 'failed': [0, 0, 0]}
        self.assertTrue(analyze_navigation(d)['stop_hypothesis'])

    def test_graph_admits_only_exact_five_crate_feature_additions(self):
        extra = additions(NAV_FEATURES)
        baseline = '\n'.join(f'{n} v0.1.0|' for n in sorted(extra))
        actual = '\n'.join(f'{n} v0.1.0|'+','.join(sorted(flags)) for n, flags in sorted(extra.items()))
        self.assertEqual(set(graph_delta(actual, baseline, NAV_FEATURES)), EXTRA | {'incant_nav'})
        for wrong in (actual.replace('incant_nav v0.1.0|diagnostic-navigation', 'incant_nav v0.1.0|'),
                      actual+'\nextra v1.0|', actual.replace('diagnostic-navigation', 'default', 1)):
            with self.assertRaises(ValueError): graph_delta(wrong, baseline, NAV_FEATURES)
        with self.assertRaises(ValueError): graph_delta(actual, baseline)
        with self.assertRaises(ValueError): additions(NAV_FEATURES+['native-checkpoints'])

    def test_actual_target_compiler_must_include_navigation_recipient(self):
        root = Path('/private/example-target')
        extra = additions(NAV_FEATURES)
        baseline = {name: [] for name in extra}
        messages = [{'reason': 'build-finished', 'success': True}]
        for name, flags in extra.items():
            for kind in (['lib', 'bin'] if name == 'incant_game_player' else ['lib']):
                messages.append({'reason': 'compiler-artifact', 'target': {'name': name, 'kind': [kind]},
                                 'filenames': [str(root/TARGET/'shipping'/f'{name}.{kind}')],
                                 'features': sorted(flags)})
        got = compiler_features(messages, root, baseline, NAV_FEATURES)
        self.assertEqual(got['features']['incant_nav'], ['diagnostic-navigation'])
        without_nav = [m for m in messages if m.get('target', {}).get('name') != 'incant_nav']
        with self.assertRaises(ValueError): compiler_features(without_nav, root, baseline, NAV_FEATURES)


if __name__ == '__main__': unittest.main()
