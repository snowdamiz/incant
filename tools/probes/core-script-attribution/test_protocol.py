"""Pure-data controls; never start Cargo, a player, or a benchmark."""
import copy
from pathlib import Path
import unittest

from common import DIAGNOSTIC, EXTRA, TARGET, TICKS, compiler_features, graph_delta, source_inventory
from analyze import analyze, REGIONS, NUMERIC


def report():
    rows = []
    for i in range(1, TICKS + 1):
        regions = [100] * len(REGIONS)
        callbacks, calls = [0] * 64, [0] * 64
        callbacks[1], callbacks[3] = 20, 30
        calls[1] = calls[3] = 1
        rows.append({'frame': i, 'ordinal': i, 'valid': True, 'success': True, 'unwound': False,
                     'whole_tick_calls': 1, 'whole_tick_ns': 1000, 'trace_ns': sum(regions),
                     'regions_ns': regions, 'numeric_ns': [10, 10, 20], 'numeric_calls': [1, 1, 1],
                     'source_ns': callbacks, 'source_calls': calls})
    sources = [[1] * 16, [2] * 16]
    totals = {'calls': TICKS, 'wall_ns': TICKS * 1000, 'max_wall_ns': 1000,
              'unwound': 0, 'unavailable_cpu_samples': 0}
    counts = [0] * 10001; counts[1] = TICKS
    data = {'tick': TICKS, 'profile': {'version': 4, 'completed_frames': TICKS, 'failed_frames': 0,
        'dropped_source_samples': 0, 'tracy_process_transport': False,
        'quickjs_heap': {'version': 1, 'mode': 'maintained', 'sampling': 'script_tick_end_per_vm',
            'samples': TICKS, 'detailed_samples': 0, 'invalid_samples': 0,
            'sampled_peak_estimated_used_bytes': None, 'sampled_peak_allocated_bytes': 64,
            'sampled_peak_allocation_count': 4,
            'latest': {'limit_bytes': 32 * 1024**2, 'allocated_bytes': 32, 'allocation_count': 3,
                       'estimated_used_bytes': None}},
        'script_phase_diagnostic': {'schema': 1, 'diagnostic_only': True,
            'eligible_for_performance_gate': False, 'complete': True, 'max_rows': 4096,
            'raw_row_bytes': 832, 'raw_storage_bound_bytes': 4096 * 832, 'attempted': TICKS,
            'open': 0, 'regions': REGIONS, 'numeric_regions': NUMERIC, 'rows': rows},
        'scripts': [{'slot': slot, 'source': ident, 'totals': dict(totals)}
                    for slot, ident in zip((1, 3), sources)],
        'zones': [{'name': 'script.tick', 'totals': totals}],
        'frame_wall': {'version': 1, 'unit': 'ns', 'bucket_width_ns': 10000, 'max_ns': 100000000,
            'attempted_frames': TICKS, 'successful_frames': TICKS, 'failed_frames': 0,
            'invalid_frames': 0, 'active_frames': 0, 'saturated': False,
            'phases': [{'name': 'script.tick', 'counts': counts, 'samples': TICKS,
                        'missing_frames': 0, 'invalid_frames': 0, 'overflow_samples': 0,
                        'p95_lower_ns': 1, 'p95_upper_ns': 10000}]}}}
    return data, sources


class Protocol(unittest.TestCase):
    def test_inventory_refuses_missing_transitive_sources_or_extra_rows(self):
        tracked = ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', '.cargo/config.toml',
                   'crates/example/build.rs', 'sdk/compiler.js', 'third_party/engine/source.c',
                   'tools/package.py', 'docs/scope.md']
        files = {name: 'digest' for name in tracked if not name.startswith('docs/')}
        source_inventory(files, tracked)
        for name in files:
            with self.assertRaises(ValueError):
                source_inventory({k: v for k, v in files.items() if k != name}, tracked)
        with self.assertRaises(ValueError):
            source_inventory(dict(files, arbitrary='digest'), tracked)

    def test_same_frame_tail_and_nested_residuals(self):
        raw, sources = report()
        got = analyze(raw, sources)
        self.assertEqual(got['same_frame_tail']['frames'], list(range(1, 109)))
        self.assertEqual(got['all_frames']['residual_totals_ns'], {
            'callback_region': TICKS * 50, 'validation': TICKS * 90,
            'scope_cleanup': TICKS * 70, 'trace_minus_whole': TICKS * 400})
        self.assertFalse(got['eligible_for_performance_gate'])

    def test_refuses_missing_identity_coverage_nested_overflow_and_histogram_drift(self):
        original, sources = report()
        for mutate in (
            lambda p: p.update(version=3),
            lambda p: p['script_phase_diagnostic'].update(complete=False),
            lambda p: p['script_phase_diagnostic']['rows'][0].update(frame=True),
            lambda p: p['script_phase_diagnostic']['rows'][1].update(frame=1),
            lambda p: p['script_phase_diagnostic']['rows'][0].update(whole_tick_ns=1001),
            lambda p: p['script_phase_diagnostic']['rows'][0]['source_calls'].__setitem__(2, 1),
            lambda p: p['script_phase_diagnostic']['rows'][0]['numeric_ns'].__setitem__(0, 101),
            lambda p: p['script_phase_diagnostic']['rows'][0]['source_ns'].__setitem__(1, 101),
            lambda p: p['frame_wall']['phases'][0]['counts'].__setitem__(1, TICKS - 1),
        ):
            bad = copy.deepcopy(original); mutate(bad['profile'])
            with self.assertRaises(ValueError):
                analyze(bad, sources)

    def test_graph_allows_exact_four_additions_only(self):
        base = '\n'.join(f'{n} v0.0.1|profile' for n in sorted(EXTRA)) + '\nother v1|a\nother v2|b'
        actual = '\n'.join(f'{n} v0.0.1|profile,{DIAGNOSTIC}' for n in sorted(EXTRA)) + '\nother v1|a\nother v2|b'
        self.assertEqual(set(graph_delta(actual, base)), EXTRA)
        for wrong in (actual.replace('other v2|b', 'other v2|b,new'),
                      actual.replace('other v2', 'other v3'), actual.replace(DIAGNOSTIC, 'other-feature', 1)):
            with self.assertRaises(ValueError):
                graph_delta(wrong, base)

    def test_graph_preserves_duplicate_package_version_feature_rows(self):
        receivers = [f'{name} v0.0.1|{flags}' for name in sorted(EXTRA)
                     for flags in ('profile', 'profile,retained')]
        units = ['indexmap v2.14.2|default,std', 'indexmap v2.14.2|std']
        baseline = '\n'.join(receivers + units)
        actual = '\n'.join([row + ',' + DIAGNOSTIC for row in receivers] + units)
        change = graph_delta(actual, baseline)
        self.assertEqual(set(change), EXTRA)
        for name in EXTRA:
            self.assertEqual(change[name]['before'], [['profile'], ['profile', 'retained']])
            self.assertEqual(change[name]['after'],
                             [[DIAGNOSTIC, 'profile'], [DIAGNOSTIC, 'profile', 'retained']])
        # Repeated edges to the same exact unit do not add a distinct unit.
        self.assertEqual(graph_delta(actual + '\n' + units[0] + ' (*)', baseline), change)

    def test_graph_refuses_merged_missing_or_extra_nonreceiver_units(self):
        receivers = [f'{name} v0.0.1|profile' for name in sorted(EXTRA)]
        units = ['indexmap v2.14.2|default,std', 'indexmap v2.14.2|std']
        baseline = '\n'.join(receivers + units)
        changed = [row + ',' + DIAGNOSTIC for row in receivers]
        for wrong in (
            [units[0]],  # A union would conceal the lost std-only unit.
            [units[1]],
            units + ['indexmap v2.14.2|default,std,new'],
            [units[0] + ',' + DIAGNOSTIC, units[1]],
        ):
            with self.assertRaises(ValueError):
                graph_delta('\n'.join(changed + wrong), baseline)

    def test_graph_requires_diagnostic_on_every_row_of_exact_four_receivers(self):
        units = ['indexmap v2.14.2|default,std', 'indexmap v2.14.2|std']
        rows = [f'{name} v0.0.1|{flags}' for name in sorted(EXTRA)
                for flags in ('profile', 'profile,retained')]
        baseline = '\n'.join(rows + units)
        changed = [row + ',' + DIAGNOSTIC for row in rows]
        for index in range(len(rows)):
            for mode in ('missing_row', 'missing_flag', 'extra_row'):
                wrong = list(changed)
                if mode == 'missing_row':
                    del wrong[index]
                elif mode == 'missing_flag':
                    wrong[index] = rows[index]
                else:
                    wrong.append(changed[index] + ',new')
                with self.subTest(index=index, mode=mode), self.assertRaises(ValueError):
                    graph_delta('\n'.join(wrong + units), baseline)

    def test_compiler_requires_both_target_units_and_no_extra_runtime_features(self):
        root = Path('/private/diagnostic-fixture')
        baseline = {name: ['profile'] for name in EXTRA}
        messages = [{'reason': 'build-finished', 'success': True}]
        for name in EXTRA:
            for kind in (['lib', 'bin'] if name == 'incant_game_player' else ['lib']):
                messages.append({'reason': 'compiler-artifact', 'target': {'name': name, 'kind': [kind]},
                                 'filenames': [str(root / TARGET / 'shipping' / (name + '.' + kind))],
                                 'features': ['profile', DIAGNOSTIC]})
        compiler_features(messages, root, baseline)
        for mode in ('host', 'extra', 'missing_bin'):
            bad = copy.deepcopy(messages)
            if mode == 'host':
                for m in bad[1:]: m['filenames'] = [f.replace(TARGET + '/', '') for f in m['filenames']]
            elif mode == 'extra': bad[1]['features'].append('native-checkpoints')
            else: bad = [m for m in bad if m.get('target', {}).get('kind') != ['bin']]
            with self.assertRaises(ValueError):
                compiler_features(bad, root, baseline)


if __name__ == '__main__':
    unittest.main()
