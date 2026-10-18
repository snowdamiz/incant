"""Pure data and feature-policy checks; no Cargo, player or benchmark execution."""
import copy
from pathlib import Path
import unittest

from analyze import analyze
from common import (DIAGNOSTIC, EXTRA, QUERY_FEATURES, QUERIES, TARGET, TICKS,
                    additions, compiler_features, graph_delta, query_authoring_subset, node_version,
                    FEATURES, NAV_FEATURES, BUDGET_FEATURES)
from query_analysis import CAPABILITIES, REGIONS
from test_protocol import report


def query_report():
    data, sources = report()
    d = data['profile']['script_phase_diagnostic']
    d.update(schema=5, query_capabilities=CAPABILITIES, query_regions=REGIONS,
             raw_row_bytes=1536, raw_storage_bound_bytes=4096 * 1536)
    for row in d['rows']:
        row['queries'] = {
            'inclusive_ns': [[10, 2, 3, 1], [12, 2, 4, 3], [0] * 4,
                             [8, 2, 2, 1], [0] * 4, [6, 1, 1, 1]],
            'calls': [[1] * 4, [2] * 4, [0] * 4, [1] * 4, [0] * 4, [1] * 4],
            'failed': [[0] * 4 for _ in range(6)],
            'source_masks': [2, 8, 0, 2, 0, 8], 'recursive_calls': [0] * 6}
    return data, sources


class QueryProtocol(unittest.TestCase):
    def test_retained_input_subset_can_omit_only_exact_historical_journal_records(self):
        files = {n: 'content' for n in ['core.json', 'gameplay.ts', 'stress.ts']
                 + [f'art/{i}.bin' for i in range(57)]}
        history = {f'.incant-source-transactions/{i}': 'old' for i in range(13)}
        history['source.journal.jsonl'] = 'old'
        original = dict(files, **history)
        self.assertEqual(query_authoring_subset(files, original), history)
        for change in (lambda f: f.pop('gameplay.ts'), lambda f: f.update({'core.json': 'changed'}),
                       lambda f: (f.pop('art/0.bin'), f.update({'source.journal.jsonl': 'old'})),
                       lambda f: f.update({'new': 'new'})):
            bad = dict(files); change(bad)
            with self.assertRaises(ValueError): query_authoring_subset(bad, original)
        self.assertEqual(node_version(QUERY_FEATURES), 'v22.23.3')
        for mode in (FEATURES, NAV_FEATURES, BUDGET_FEATURES):
            self.assertEqual(node_version(mode), 'v24.21.0')
        with self.assertRaises(ValueError): node_version(QUERY_FEATURES + ['unknown'])

    def test_exact_totals_residuals_ratios_and_same_whole_frame_tail(self):
        data, sources = query_report()
        got = analyze(data, sources, queries=True)
        q = got['queries']; all_frames = q['all_frames']
        self.assertEqual(all_frames['calls'][1], [2 * TICKS] * 4)
        self.assertEqual(all_frames['call_residual_ns'], [4*TICKS, 3*TICKS, 0, 3*TICKS, 0, 3*TICKS])
        self.assertEqual(all_frames['callback_residual_ns'], 14 * TICKS)
        self.assertEqual(all_frames['wall_ns_per_call'][1][0],
                         {'numerator_ns': 12*TICKS, 'denominator_calls': 2*TICKS})
        self.assertIsNone(all_frames['wall_ns_per_call'][2][0])
        self.assertEqual(q['same_frame_tail']['frames'], got['same_frame_tail']['frames'])
        self.assertEqual(q['same_frame_tail']['frames'], list(range(1, 109)))
        self.assertTrue(q['other_queries_present'])
        self.assertFalse(got['eligible_for_performance_gate'])

    def test_missing_extra_and_mislabeled_schema_or_regions_refuse(self):
        original, sources = query_report()
        changes = [lambda d, n=n: d.update(schema=n) for n in (1, 4, 6, 7, 8)]
        changes += [lambda d: d.update(query_capabilities=list(reversed(CAPABILITIES))),
                    lambda d: d.update(query_regions=list(reversed(REGIONS))),
                    lambda d: d.update(navigation_regions=[]),
                    lambda d: d.update(raw_row_bytes=2049, raw_storage_bound_bytes=4096*2049),
                    lambda d: d['rows'][0].pop('queries'),
                    lambda d: d['rows'][0]['queries'].update(argument='never collected')]
        for change in changes:
            data = copy.deepcopy(original); change(data['profile']['script_phase_diagnostic'])
            with self.assertRaises((ValueError, KeyError)): analyze(data, sources, queries=True)
        with self.assertRaises(ValueError): analyze(original, sources)
        with self.assertRaises(ValueError): analyze(original, sources, queries=True, budget=True)
        original['profile']['script_phase_diagnostic']['schema'] = 1
        with self.assertRaises(ValueError): analyze(original, sources)

    def test_malformed_bounds_and_failures_never_become_valid_zero(self):
        original, sources = query_report()
        changes = [lambda q: q.update(calls=[]), lambda q: q.update(inclusive_ns=[[0]*4]*5),
                   lambda q: q['recursive_calls'].__setitem__(0, 1),
                   lambda q: q['failed'][0].__setitem__(0, 1),
                   lambda q: q['calls'][0].__setitem__(2, 0),
                   lambda q: q['inclusive_ns'][2].__setitem__(0, 1)]
        for key, index, bad in [('calls', 0, True), ('calls', 0, -1), ('calls', 0, 2**32),
                                ('inclusive_ns', 1, float('nan')), ('inclusive_ns', 1, float('inf')),
                                ('inclusive_ns', 1, 2**64)]:
            changes.append(lambda q, k=key, i=index, b=bad: q[k][0].__setitem__(i, b))
        for change in changes:
            data = copy.deepcopy(original); change(data['profile']['script_phase_diagnostic']['rows'][0]['queries'])
            with self.assertRaises(ValueError): analyze(data, sources, queries=True)

    def test_nested_stage_call_and_source_overflow_refuse(self):
        original, sources = query_report()
        for change in [lambda q: q['inclusive_ns'][0].__setitem__(1, 9),
                       lambda q: q['inclusive_ns'][0].__setitem__(0, 21),
                       lambda q: q['inclusive_ns'][0].__setitem__(0, 20),  # plus locale8 exceeds source1
                       lambda q: q['source_masks'].__setitem__(0, 1),
                       lambda q: q['source_masks'].__setitem__(0, 0),
                       lambda q: q['source_masks'].__setitem__(2, 2)]:
            data = copy.deepcopy(original); change(data['profile']['script_phase_diagnostic']['rows'][0]['queries'])
            with self.assertRaises(ValueError): analyze(data, sources, queries=True)

    def test_some_zero_query_frames_are_preserved_but_empty_entire_run_refuses(self):
        data, sources = query_report()
        q = data['profile']['script_phase_diagnostic']['rows'][0]['queries']
        for key in ('calls', 'failed', 'inclusive_ns'): q[key] = [[0]*4 for _ in range(6)]
        q['source_masks'] = [0]*6
        result = analyze(data, sources, queries=True)
        self.assertEqual(result['queries']['all_frames']['count'], TICKS)
        for row in data['profile']['script_phase_diagnostic']['rows']: row['queries'] = copy.deepcopy(q)
        with self.assertRaises(ValueError): analyze(data, sources, queries=True)

    def test_normal_reports_and_unknown_feature_combinations_remain_refused(self):
        data, sources = report()
        analyze(data, sources)
        with self.assertRaises(ValueError): analyze(data, sources, queries=True)
        for features in (QUERY_FEATURES + ['diagnostic-navigation'], ['project', QUERIES],
                         QUERY_FEATURES[:-1] + ['unknown']):
            with self.assertRaises(ValueError): additions(features)

    def test_exact_query_graph_receivers_preserve_target_and_host_distinction(self):
        extra = additions(QUERY_FEATURES)
        self.assertEqual(set(extra), EXTRA)
        self.assertEqual(extra['incant_runtime'], {DIAGNOSTIC})
        for name in EXTRA - {'incant_runtime'}: self.assertEqual(extra[name], {DIAGNOSTIC, QUERIES})
        baseline = {name: ['profile'] for name in EXTRA}
        base = '\n'.join(f'{n} v0.0.1|profile' for n in sorted(EXTRA))
        actual = '\n'.join(f'{n} v0.0.1|' + ','.join(sorted({'profile'} | extra[n])) for n in sorted(EXTRA))
        self.assertEqual(set(graph_delta(actual, base, QUERY_FEATURES)), EXTRA)
        root = Path('/private/query-compiler-fixture')
        messages = [{'reason': 'build-finished', 'success': True}]
        for name in EXTRA:
            for kind in (['lib', 'bin'] if name == 'incant_game_player' else ['lib']):
                messages.append({'reason': 'compiler-artifact', 'target': {'name': name, 'kind': [kind]},
                    'filenames': [str(root / TARGET / 'shipping' / (name + '.' + kind))],
                    'features': sorted({'profile'} | extra[name])})
        self.assertEqual(compiler_features(messages, root, baseline, QUERY_FEATURES)['extra_feature'], QUERIES)
        for index in range(1, len(messages)):
            bad = copy.deepcopy(messages)
            bad[index]['features'].append('diagnostic-script-budget')
            with self.assertRaises(ValueError): compiler_features(bad, root, baseline, QUERY_FEATURES)
        for wrong in (actual.replace(QUERIES, 'unknown', 1),
                      actual.replace('incant_runtime v0.0.1|', 'incant_runtime v0.0.1|' + QUERIES + ',')):
            with self.assertRaises(ValueError): graph_delta(wrong, base, QUERY_FEATURES)


if __name__ == '__main__':
    unittest.main()
