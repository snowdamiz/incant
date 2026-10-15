import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native
import native_script_prerequisites as prerequisites


class ScriptPrerequisites(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.engine = self.root / 'engine'
        (self.engine / 'tools').mkdir(parents=True)
        (self.engine / 'tools/build_native_script.mjs').write_text('// trusted driver fixture')
        self.versions = {'typescript': '5.9.3', '@swc/core': '1.16.13', '@jridgewell/sourcemap-codec': '1.6.0'}
        (self.engine / 'package.json').write_text(json.dumps({'devDependencies': self.versions}))
        self.project = {'settings': {}, 'scenes': {'scripted': {'entities': {
            'entity': {'components': {'Script': {'source': 'never-execute.ts'}}}}},
            'plain': {'entities': {}}}}

    def check(self, scene='scripted'):
        prerequisites.check(json.dumps(self.project).encode(), scene, self.engine)

    def install(self):
        for name, version in self.versions.items():
            manifest = self.engine / 'node_modules' / name / 'package.json'
            manifest.parent.mkdir(parents=True)
            manifest.write_text(json.dumps({'version': version}))

    def test_missing_each_compiler_package_is_actionable(self):
        self.install()
        for name in self.versions:
            path = self.engine / 'node_modules' / name / 'package.json'
            saved = path.read_bytes()
            path.unlink()
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'run npm ci.*missing'):
                self.check()
            path.write_bytes(saved)

    def test_wrong_version_refuses_and_matching_versions_pass(self):
        self.install()
        self.check()
        for name in self.versions:
            path = self.engine / 'node_modules' / name / 'package.json'
            saved = path.read_bytes()
            path.write_text(json.dumps({'version': '0.0.0'}))
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'version mismatch'):
                self.check()
            path.write_bytes(saved)

    def test_only_selected_scene_requires_compiler(self):
        self.check('plain')
        self.project['scenes']['plain']['program'] = {'kind': 'wasm', 'module': 'module'}
        self.check('plain')
        with self.assertRaisesRegex(ValueError, 'missing typescript'):
            self.check()

    def test_implicit_single_scene_and_invalid_selection(self):
        with self.assertRaisesRegex(ValueError, 'Choose a scene'):
            self.check(None)
        with self.assertRaisesRegex(ValueError, 'Selected scene'):
            self.check('absent')
        del self.project['scenes']['scripted']
        self.check(None)

    def test_missing_driver_and_malformed_engine_manifest_refuse(self):
        driver = self.engine / 'tools/build_native_script.mjs'
        driver.unlink()
        with self.assertRaisesRegex(ValueError, 'Missing trusted TypeScript compiler driver'):
            self.check()
        driver.write_text('// trusted driver fixture')
        for manifest in ([], {}, {'devDependencies': []}):
            (self.engine / 'package.json').write_text(json.dumps(manifest))
            with self.subTest(manifest=manifest), self.assertRaisesRegex(ValueError, 'Invalid engine compiler'):
                self.check()

    def test_public_build_refuses_before_any_tool_or_cargo_launch(self):
        project = self.root / 'project.json'
        project.write_text(json.dumps(self.project))
        before = sorted(p.relative_to(self.root) for p in self.root.rglob('*'))
        with patch.object(export_native, 'ROOT', self.engine), \
                patch.object(export_native, 'execute', side_effect=AssertionError('tool launched')), \
                patch.object(export_native.native_process, 'run', side_effect=AssertionError('Cargo launched')), \
                self.assertRaisesRegex(ValueError, 'run npm ci'):
            export_native.build(project, self.root / 'staging', self.root / 'storage',
                                'scripted', None, 'headless')
        self.assertEqual(sorted(p.relative_to(self.root) for p in self.root.rglob('*')), before)


if __name__ == '__main__':
    unittest.main()
