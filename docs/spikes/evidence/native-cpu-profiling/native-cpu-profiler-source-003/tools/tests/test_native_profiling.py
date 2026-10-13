import copy
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native
import native_profiling as audit


def graph(mode):
    return '\n'.join(f'{name} v{audit.VERSIONS.get(name, "0.0.1")}|{",".join(sorted(flags))}'
                     for name, flags in audit.expected(mode).items())


def messages(root, mode):
    rows = [{'reason': 'build-finished', 'success': True}]
    for name, flags in audit.expected(mode).items():
        rows.append({'reason': 'compiler-artifact', 'target': {'name': name.replace('-', '_'), 'kind': ['lib']},
                     'package_id': f'registry+https://example.invalid/#{name}@{audit.VERSIONS.get(name, "0.0.1")}',
                     'features': sorted(flags), 'filenames': [str(root / 'target-triple/shipping/deps' / f'lib{name}.rlib')]})
    return rows


class Profiling(unittest.TestCase):
    def test_only_explicit_headless_option_admits_features_before_io(self):
        self.assertEqual(audit.features(None, 'window'), [])
        self.assertEqual(audit.features('summary', 'headless'), ['profile'])
        self.assertEqual(audit.features('tracy', 'headless'), ['profile-tracy'])
        for mode, route in [('unknown', 'headless'), ('summary', 'window'), ('tracy', 'window')]:
            with self.subTest(mode=mode, route=route), self.assertRaises(ValueError):
                export_native.export(Path('/never-read'), Path('/never-created'), mode=route, profiling=mode)

    def test_cli_preserves_explicit_option_without_project_mutation(self):
        with patch.object(sys, 'argv', ['export_native.py', 'source', 'output', '--profiling', 'tracy']), \
                patch.object(sys, 'stdout', io.StringIO()), \
                patch.object(export_native, 'export', return_value={'target': 'cpu', 'player_sha256': 'hash'}) as call:
            export_native.main()
        self.assertEqual(call.call_args.kwargs['profiling'], 'tracy')

    def test_environment_override_refuses_before_project_io_but_defaults_unchanged(self):
        for key in ('TRACY_CLIENT_LIB', 'TRACY_CLIENT_LIB_PATH', 'TRACY_CLIENT_STATIC', 'TRACY_CLIENT_SYS_CXXFLAGS',
                    'CXXFLAGS', 'CXXFLAGS_aarch64_apple_darwin', 'TARGET_CXXFLAGS', 'CPPFLAGS'):
            with self.subTest(key=key):
                audit.environment(None, {key: 'override'})
                audit.environment('tracy', {key: ''})
                with patch.dict('os.environ', {key: 'override'}), self.assertRaisesRegex(ValueError, 'Unset ' + key):
                    export_native.export(Path('/never-read'), Path('/never-created'), profiling='tracy')

    def test_selected_graph_refuses_transport_and_feature_unification_leaks(self):
        for mode in audit.MODES:
            self.assertEqual(audit.graph(graph(mode), mode),
                             {name: sorted(flags) for name, flags in sorted(audit.expected(mode).items())})
        cases = [(graph('summary'), None), (graph('tracy'), 'summary'), (graph(None), 'summary'),
                 (graph('tracy').replace('only-localhost', 'broadcast'), 'tracy'),
                 (graph('tracy').replace('0.29.0', '0.30.0'), 'tracy'),
                 (graph('tracy') + '\ntracy-client v0.18.5|default', 'tracy'),
                 (graph('tracy').replace('ondemand,', ''), 'tracy')]
        for text, mode in cases:
            with self.subTest(text=text, mode=mode), self.assertRaises(ValueError):
                audit.graph(text, mode)

    def test_compiler_target_units_are_required_and_host_cannot_stand_in(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            rows = messages(root, 'tracy')
            host = copy.deepcopy(rows[-1])
            host['features'] = ['default']
            host['filenames'] = [str(root / 'shipping/deps/host.rlib')]
            result = audit.artifacts(rows + [host], 'tracy', target_dir=root, target='target-triple', profile='shipping')
            self.assertEqual(result['ignored_build_host'], ['tracy-client-sys'])
            for change in ('host-only', 'wrong-target', 'features', 'conflict', 'failure', 'version', 'missing'):
                bad = copy.deepcopy(rows)
                if change == 'host-only': bad[-1] = host
                if change == 'wrong-target': bad[-1]['filenames'] = [str(root / 'other/shipping/wrong.rlib')]
                if change == 'features': bad[-1]['features'].append('code-transfer')
                if change == 'conflict': bad.append({**bad[-1], 'features': ['default']})
                if change == 'failure': bad[0]['success'] = False
                if change == 'version': bad[-1]['package_id'] = bad[-1]['package_id'].replace('0.29.0', '0.30.0')
                if change == 'missing': bad.pop()
                with self.subTest(change=change), self.assertRaises(ValueError):
                    audit.artifacts(bad, 'tracy', target_dir=root, target='target-triple', profile='shipping')


if __name__ == '__main__':
    unittest.main()
