import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import physics_features as physics


class PhysicsFeaturesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.target = 'aarch64-apple-darwin'
        self.output = self.root / 'target'

    def package(self, name, body):
        directory = self.root / 'crates' / name
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / 'Cargo.toml'
        path.write_text(f'[package]\nname = "{name}"\nversion = "0.0.1"\n' + body)
        return path

    def fixture(self):
        (self.root / 'Cargo.toml').write_text('''[workspace]
members = ["crates/*"]
resolver = "3"
[workspace.dependencies]
incant_physics_runtime = { path = "crates/incant_physics_runtime", default-features = false }
incant_runtime = { path = "crates/incant_runtime", default-features = false }
''')
        self.package('incant_physics_runtime', '''
[features]
default = ["deterministic"]
deterministic = ["rapier3d/enhanced-determinism"]
fast-simd8 = ["rapier3d/simd8"]
[dependencies]
rapier3d = "=0.36.0"
[target.'cfg(not(target_family = "wasm"))'.dependencies]
rapier3d = { version = "=0.36.0", features = ["parallel"] }
''')
        self.package('incant_runtime', '''
[features]
default = ["physics-deterministic"]
physics-deterministic = ["incant_physics_runtime/deterministic"]
physics-fast-simd8 = ["incant_physics_runtime/fast-simd8"]
[dependencies]
incant_physics_runtime.workspace = true
''')
        self.package('player', '''
[features]
default = ["physics-deterministic"]
physics-deterministic = ["incant_runtime/physics-deterministic", "optional_scene?/physics-deterministic"]
physics-fast-simd8 = ["incant_runtime/physics-fast-simd8", "optional_scene?/physics-fast-simd8"]
[dependencies]
incant_runtime.workspace = true
optional_scene = { package = "incant_runtime", path = "../incant_runtime", optional = true, default-features = false }
''')

    def mutate(self, name, before, after):
        path = self.root / 'crates' / name / 'Cargo.toml'
        text = path.read_text()
        self.assertIn(before, text)
        path.write_text(text.replace(before, after))

    def test_valid_default_optional_target_and_workspace_forwarding(self):
        self.fixture()
        self.assertEqual(physics.manifest_errors(self.root), [])
        self.assertEqual(physics.manifest_errors(self.root, ['player']), [])
        self.assertEqual(physics.manifest_errors(self.root, ['absent']), ['unknown selected package: absent'])

    def test_unconditional_legacy_rapier_flag_is_rejected(self):
        self.fixture()
        self.package('incant_physics', '''
[features]
default = ["physics-deterministic"]
physics-deterministic = ["incant_physics_runtime/deterministic", "rapier3d/enhanced-determinism"]
physics-fast-simd8 = ["incant_physics_runtime/fast-simd8", "rapier3d/simd8"]
[dependencies]
incant_physics_runtime.workspace = true
rapier3d = { version = "=0.36.0", features = ["enhanced-determinism"] }
''')
        self.assertTrue(any('unconditional Rapier' in error for error in physics.manifest_errors(self.root)))

    def test_member_false_cannot_cancel_workspace_defaults(self):
        self.fixture()
        path = self.root / 'Cargo.toml'
        path.write_text(path.read_text().replace(', default-features = false', ''))
        self.mutate('incant_runtime', 'incant_physics_runtime.workspace = true',
                    'incant_physics_runtime = { workspace = true, default-features = false }')
        errors = physics.manifest_errors(self.root)
        self.assertTrue(any('default-enabled physics edge dependencies incant_physics_runtime' in e for e in errors))

    def test_direct_default_and_optional_activation_are_rejected(self):
        self.fixture()
        self.mutate('player', 'optional = true, default-features = false', 'optional = true')
        self.mutate('player', 'optional_scene?/physics-fast-simd8', 'optional_scene/physics-fast-simd8')
        errors = physics.manifest_errors(self.root)
        self.assertTrue(any('default-enabled' in e for e in errors))
        self.assertTrue(any('missing forwarding optional_scene?' in e for e in errors))
        self.assertTrue(any('unexpected activation/forwarding optional_scene/' in e for e in errors))

    def test_build_and_dev_edges_are_audited_even_for_test_only_ancestor(self):
        self.fixture()
        self.package('tests_only', '''
[features]
default = ["physics-deterministic"]
physics-deterministic = []
physics-fast-simd8 = []
[build-dependencies]
producer = { package = "incant_runtime", path = "../incant_runtime" }
[dev-dependencies]
incant_runtime.workspace = true
''')
        errors = physics.manifest_errors(self.root)
        self.assertTrue(any('default-enabled physics edge build-dependencies producer' in e for e in errors))
        self.assertTrue(any('missing forwarding incant_runtime/physics-deterministic' in e for e in errors))

    def test_direct_and_inherited_dependency_features_cannot_force_a_mode(self):
        for flag in ('physics-deterministic', 'physics-fast-simd8', 'default'):
            with self.subTest(flag=flag):
                self.fixture()
                self.mutate('player', 'incant_runtime.workspace = true',
                            f'incant_runtime = {{ workspace = true, features = ["{flag}"] }}')
                self.assertTrue(any('unconditional physics feature on dependencies incant_runtime' in e
                                    for e in physics.manifest_errors(self.root, ['player'])))
        self.fixture()
        path = self.root / 'Cargo.toml'
        before = 'incant_runtime = { path = "crates/incant_runtime", default-features = false }'
        self.assertIn(before, path.read_text())
        path.write_text(path.read_text().replace(before, before[:-2] + ', features = ["physics-deterministic"] }'))
        self.assertTrue(any('unconditional physics feature on dependencies incant_runtime' in e
                            for e in physics.manifest_errors(self.root, ['player'])))

    def test_renamed_optional_build_dev_and_target_features_cannot_force_a_mode(self):
        self.fixture()
        self.mutate('player', 'optional = true, default-features = false',
                    'optional = true, default-features = false, features = ["physics-deterministic"]')
        self.assertTrue(any('unconditional physics feature on dependencies optional_scene' in e
                            for e in physics.manifest_errors(self.root, ['player'])))
        for kind in ('build-dependencies', 'dev-dependencies',
                     "target.'cfg(target_os = \"macos\")'.build-dependencies"):
            with self.subTest(kind=kind):
                self.fixture()
                for mode in ('physics-deterministic', 'physics-fast-simd8'):
                    self.mutate('player', f'"incant_runtime/{mode}"',
                                f'"incant_runtime/{mode}", "producer/{mode}"')
                path = self.root / 'crates/player/Cargo.toml'
                path.write_text(path.read_text() + f'\n[{kind}]\n'
                                'producer = { package = "incant_runtime", path = "../incant_runtime", default-features = false }\n')
                self.assertEqual(physics.manifest_errors(self.root, ['player']), [])
                self.mutate('player', 'path = "../incant_runtime", default-features = false }',
                            'path = "../incant_runtime", default-features = false, features = ["physics-fast-simd8"] }')
                self.assertTrue(any('unconditional physics feature' in e and 'producer' in e
                                    for e in physics.manifest_errors(self.root, ['player'])))

    def test_non_mode_aliases_cannot_select_local_dependency_or_weak_modes(self):
        for entry in ('physics-deterministic', 'incant_runtime/physics-fast-simd8',
                      'optional_scene?/physics-deterministic'):
            with self.subTest(entry=entry):
                self.fixture()
                self.mutate('player', '[features]', '[features]\n'
                            f'alias = ["cycle"]\ncycle = ["alias", "{entry}"]')
                errors = physics.manifest_errors(self.root, ['player'])
                for alias in ('alias', 'cycle'):
                    self.assertIn(f'player: unrelated feature {alias} activates physics mode', errors)

    def test_benign_activation_aliases_and_cycles_preserve_valid_forwarding(self):
        self.fixture()
        self.mutate('incant_runtime', '[features]', '[features]\nutility = []')
        self.mutate('player', '[features]', '[features]\n'
                    'ui = ["dep:optional_scene", "incant_runtime/utility"]\n'
                    'alias = ["ui"]\ncycle_a = ["cycle_b"]\ncycle_b = ["cycle_a"]')
        self.mutate('player', 'incant_runtime.workspace = true',
                    'incant_runtime = { workspace = true, features = ["utility"] }')
        self.assertEqual(physics.manifest_errors(self.root, ['player']), [])
        self.mutate('incant_runtime', 'utility = []', 'utility = ["physics-deterministic"]')
        errors = physics.manifest_errors(self.root, ['player'])
        self.assertIn('player: unrelated feature alias activates physics mode', errors)
        self.assertIn('player: unconditional physics feature on dependencies incant_runtime', errors)

    def test_compiler_and_cook_defaults_are_preserved_without_activation(self):
        self.fixture()
        self.package('incant_script', '''
[features]
default = ["compiler", "physics-deterministic"]
compiler = []
physics-deterministic = ["incant_runtime/physics-deterministic"]
physics-fast-simd8 = ["incant_runtime/physics-fast-simd8"]
[dependencies]
incant_runtime.workspace = true
''')
        self.assertEqual(physics.manifest_errors(self.root), [])
        self.mutate('incant_script', '"compiler", "physics-deterministic"', '"physics-deterministic"')
        self.assertTrue(any('defaults must preserve' in e for e in physics.manifest_errors(self.root)))
        self.mutate('incant_script', 'physics-fast-simd8 = [', 'physics-fast-simd8 = ["compiler", ')
        self.assertTrue(any('unexpected activation/forwarding compiler' in e for e in physics.manifest_errors(self.root)))

    @staticmethod
    def features(mode, target='aarch64-apple-darwin'):
        result = {}
        for name in physics.SOLVERS:
            flags = []
            if name == 'simba':
                flags += ['std', 'wide', 'libm']
                if mode == 'deterministic':
                    flags += ['libm_force']
            else:
                if mode == 'deterministic':
                    flags += ['enhanced-determinism']
                if mode == 'fast-simd8':
                    flags += ['simd8']
                if not target.startswith('wasm'):
                    flags += ['parallel']
                    if name.startswith('parry'):
                        flags += ['rayon']
            result[name] = sorted(flags)
        return result

    def records(self, mode, target=None, host=False):
        target = target or self.target
        directory = self.output / 'ci' if host else self.output / target / 'ci'
        return [{'reason': 'compiler-artifact', 'target': {'name': name, 'kind': ['lib']},
                 'package_id': f'registry+https://github.com/rust-lang/crates.io-index#{name}@{physics.VERSIONS[name]}',
                 'features': flags, 'filenames': [str(directory / 'deps' / f'lib{name}-123.rlib')]}
                for name, flags in self.features(mode, target).items()]

    def validate(self, records, mode, target=None):
        return physics.validate_artifacts([*records, {'reason': 'build-finished', 'success': True}], mode, target_dir=self.output,
                                          target=target or self.target, profile='ci')

    def vendored_records(self, mode, target=None, host=False):
        rows = self.records(mode, target, host)
        for row in rows:
            name = row['target']['name']
            if name.startswith('parry'):
                package = Path(physics.__file__).resolve().parents[1] / 'third_party' / name
                row['package_id'] = 'path+' + package.as_uri() + '#' + physics.VERSIONS[name]
                row['manifest_path'] = str(package / 'Cargo.toml')
        return rows

    def test_vendored_parry_name_omitted_ids_preserve_exact_target_features(self):
        for mode in physics.MODES:
            for target in (self.target, 'wasm32-unknown-unknown'):
                with self.subTest(mode=mode, target=target):
                    rows = self.vendored_records(mode, target)
                    result = self.validate(rows, mode, target)
                    self.assertEqual(result['features'], self.features(mode, target))
                    for row in rows:
                        name = row['target']['name']
                        if name.startswith('parry'):
                            row['package_id'] = row['package_id'].replace('#', '#' + name + '@')
                    self.assertEqual(self.validate(rows, mode, target)['features'], result['features'])

    def test_vendored_identity_requires_exact_package_path_version_manifest_and_library(self):
        rows = self.vendored_records('fast')
        for name in ('parry2d', 'parry3d'):
            row = next(row for row in rows if row['target']['name'] == name)
            package = Path(row['manifest_path']).parent
            cases = [('package_id', row['package_id'].replace('#0.31.1', '#0.31.2')),
                     ('package_id', 'path+' + (self.root / name).as_uri() + '#0.31.1'),
                     ('package_id', 'path+' + package.as_uri() + '#foreign@0.31.1'),
                     ('manifest_path', str(self.root / name / 'Cargo.toml')),
                     ('manifest_path', None),
                     ('target', {'name': name, 'kind': ['bin']})]
            for key, value in cases:
                changed = copy.deepcopy(rows)
                next(r for r in changed if r['target']['name'] == name)[key] = value
                with self.subTest(name=name, key=key, value=value), self.assertRaisesRegex(ValueError, 'identity'):
                    self.validate(changed, 'fast')

    def test_vendored_host_units_cannot_satisfy_target_and_wrong_math_still_refuses(self):
        host = self.vendored_records('deterministic', host=True)
        with self.assertRaisesRegex(ValueError, 'missing target physics dependency'):
            self.validate(host, 'deterministic')
        with self.assertRaisesRegex(ValueError, 'enhanced-determinism must be enabled'):
            self.validate(host + self.vendored_records('fast'), 'deterministic')
        result = self.validate(host + self.vendored_records('fast'), 'fast')
        self.assertEqual(result['ignored_build_host'], sorted(physics.SOLVERS))

    def test_native_modes_and_wasm_serial_target_features(self):
        for mode in physics.MODES:
            for target in (self.target, 'wasm32-unknown-unknown'):
                with self.subTest(mode=mode, target=target):
                    result = self.validate(self.records(mode, target), mode, target)
                    self.assertEqual(result['features'], self.features(mode, target))

    def test_correct_host_cannot_mask_wrong_target_or_missing_target(self):
        host = self.records('deterministic', host=True)
        with self.assertRaisesRegex(ValueError, 'missing target physics dependency'):
            self.validate(host, 'deterministic')
        with self.assertRaisesRegex(ValueError, 'enhanced-determinism must be enabled'):
            self.validate(host + self.records('fast'), 'deterministic')
        result = self.validate(host + self.records('fast'), 'fast')
        self.assertEqual(result['ignored_build_host'], sorted(physics.SOLVERS))

    def test_each_accidentally_unified_math_flag_refuses(self):
        for name in physics.SOLVERS:
            for mode in physics.MODES:
                records = self.records(mode)
                flag = 'libm_force' if name == 'simba' else 'enhanced-determinism'
                row = next(row for row in records if row['target']['name'] == name)
                if flag in row['features']:
                    row['features'].remove(flag)
                else:
                    row['features'].append(flag)
                with self.subTest(name=name, mode=mode), self.assertRaisesRegex(ValueError, name + ':'):
                    self.validate(records, mode)

    def test_missing_conflicting_mixed_relative_and_other_target_artifacts_refuse(self):
        records = self.records('fast')
        mutations = []
        mutations.append(records[:-1])
        conflicting = copy.deepcopy(records)
        other = copy.deepcopy(records[0]);other['features'].append('simd8')
        mutations.append(conflicting + [other])
        for filenames in ([], ['relative.rlib'],
                          [str(self.output / 'ci/librapier2d.rlib'), records[0]['filenames'][0]],
                          [str(self.output / 'x86_64-unknown-linux-gnu/ci/librapier2d.rlib')]):
            changed = copy.deepcopy(records);changed[0]['filenames'] = filenames;mutations.append(changed)
        for changed in mutations:
            with self.subTest(changed=changed[0]), self.assertRaises(ValueError):
                self.validate(changed, 'fast')
        with self.assertRaisesRegex(ValueError, 'explicit target'):
            self.validate(records, 'fast', '..')

    def test_symlink_into_host_cannot_supply_target_evidence(self):
        host = self.output / 'ci'
        host.mkdir(parents=True)
        parent = self.output / self.target
        parent.mkdir()
        (parent / 'ci').symlink_to(host, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'missing target physics dependency'):
            self.validate(self.records('fast'), 'fast')

    def test_target_parallel_and_simd_flags_are_enforced(self):
        for target, name, flag in [('wasm32-unknown-unknown', 'parry3d', 'rayon'),
                                   (self.target, 'rapier3d', 'parallel'),
                                   (self.target, 'rapier2d', 'simd8')]:
            rows = self.records('fast', target)
            row = next(row for row in rows if row['target']['name'] == name)
            row['features'] = list(set(row['features']) ^ {flag})
            with self.assertRaisesRegex(ValueError, flag):
                self.validate(rows, 'fast', target)

    def test_scoped_graph_rejects_flattened_conflicting_units(self):
        flags = self.features('fast')
        text = '\n'.join(f'{name} v0.1.0|{",".join(values)}' for name, values in flags.items())
        self.assertEqual(physics.selected_graph_errors(text, 'fast', self.target), [])
        self.assertTrue(any('conflicting' in e for e in physics.selected_graph_errors(
            text + '\nrapier3d v0.1.0|parallel,enhanced-determinism', 'fast', self.target)))
        self.assertTrue(any('invalid selected' in e for e in physics.selected_graph_errors(
            'workspace metadata is not a selected graph', 'fast', self.target)))
        self.assertTrue(any('missing target' in e for e in physics.selected_graph_errors('', 'fast', self.target)))

    def test_json_log_accepts_test_text_but_refuses_broken_objects(self):
        path = self.root / 'cargo.log'
        rows = self.records('fast')
        path.write_text('running tests\n' + '\n'.join(json.dumps(row) for row in rows))
        self.assertEqual(physics.compiler_messages(path), rows)
        path.write_text('{broken Cargo JSON\n')
        with self.assertRaises(ValueError):
            physics.compiler_messages(path)

    def test_failed_truncated_or_unrelated_library_records_cannot_certify_build(self):
        rows = self.records('fast')
        for completion in ([], [{'reason': 'build-finished', 'success': False}]):
            with self.assertRaisesRegex(ValueError, 'successful Cargo build-finished'):
                physics.validate_artifacts(rows + completion, 'fast', target_dir=self.output,
                                           target=self.target, profile='ci')
        for key, value in [('package_id', 'path+file:///fake#rapier2d@0.1.0'),
                           ('target', {'name': 'rapier2d', 'kind': ['bin']})]:
            changed = copy.deepcopy(rows)
            changed[0][key] = value
            with self.assertRaisesRegex(ValueError, 'identity'):
                self.validate(changed, 'fast')


if __name__ == '__main__':
    unittest.main()
