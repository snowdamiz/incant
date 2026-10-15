"""Guard the functional CI graph and its distinct fresh shipping configuration."""
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class FunctionalProfile(unittest.TestCase):
    def test_renderer_jobs_select_the_catalog_each_harness_requires(self):
        features = tomllib.loads((ROOT / 'crates/incant_render/Cargo.toml').read_text())['features']

        def selected(command):
            args = shlex.split(command.split('run:', 1)[1])
            pending = ['default'] + [value.removeprefix('incant_render/')
                                    for value in args[args.index('--features') + 1].split(',')]
            resolved = set()
            while pending:
                name = pending.pop()
                if name not in resolved:
                    resolved.add(name)
                    pending.extend(features.get(name, []))
            return resolved

        for name in ('ci.yml', 'desktop.yml'):
            workflow = (ROOT / '.github/workflows' / name).read_text()
            cooked = [line for line in workflow.splitlines()
                      if 'run: cargo test -p incant_render -p incant_render_gpu_host --all-targets' in line]
            self.assertEqual(len(cooked), 1)
            with self.subTest(workflow=name):
                self.assertTrue({'cooked-output', 'cooked-geometry', 'cooked-materials',
                                 'cooked-sprites', 'cooked-ui', 'cooked-text', 'cooked-clusters',
                                 'cooked-temporal', 'cooked-bloom', 'persistent-pipeline-cache'}
                                <= selected(cooked[0]))
                self.assertIn('-- --ignored --skip vulkan_catalog_cache_survives_fresh_processes_and_corruption', cooked[0])
        cache = [line for line in (ROOT / '.github/workflows/desktop.yml').read_text().splitlines()
                 if 'run: cargo test -p incant_render --test persistent_cache' in line]
        self.assertEqual(len(cache), 1)
        modes = selected(cache[0])
        self.assertIn('persistent-pipeline-cache', modes)
        self.assertNotIn('cooked-output', modes)
        self.assertNotIn('cooked-materials', modes)  # SSAO intentionally refuses this path.
        self.assertIn('-- --ignored --exact vulkan_catalog_cache_survives_fresh_processes_and_corruption', cache[0])

    def test_profile_does_not_weaken_shipping_or_default_release(self):
        profiles = tomllib.loads((ROOT / 'Cargo.toml').read_text())['profile']
        self.assertEqual(profiles['ci'], {'inherits': 'release', 'opt-level': 1})
        self.assertEqual(profiles['release'], {'strip': False})
        self.assertEqual(profiles['shipping'], {
            'inherits': 'release', 'opt-level': 3, 'lto': 'fat',
            'codegen-units': 1, 'panic': 'abort', 'debug': 2,
            'debug-assertions': False, 'incremental': False,
            'strip': 'none', 'split-debuginfo': 'off',
        })

    def test_native_jobs_use_one_test_tool_profile_and_explicit_probe_inputs(self):
        for name in ('ci.yml', 'desktop.yml'):
            workflow = (ROOT / '.github/workflows' / name).read_text()
            with self.subTest(workflow=name):
                self.assertNotIn('target/release/', workflow)
                self.assertNotIn('--release', workflow)
                self.assertEqual(workflow.count('cargo test --workspace --profile ci'), 2)
                self.assertIn('-- --ignored --test-threads=1 --nocapture', workflow)
                self.assertIn('incant_editor/custom-protocol,incant_audio_runtime/scene', workflow)
                self.assertIn('cargo clean --profile release', workflow)
                self.assertIn("save-if: ${{ github.event_name == 'push' }}", workflow)
                for line in workflow.splitlines():
                    if 'run:' in line and ('tools/probes/' in line or 'tools/shipping.py' in line or 'tools/platforms/render-check.py' in line):
                        self.assertIn('target/ci/incant_headless', line)
                    if 'run:' in line and 'tools/probes/native-game.py' in line:
                        self.assertIn('--audio-binary target/ci/examples/native_scene_audio', line)
                        self.assertIn('--fixture-binary target/ci/examples/no_renderer_fixtures', line)

    def test_missing_explicit_renderer_binary_does_not_fall_back_or_create_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / 'output'
            result = subprocess.run([sys.executable, str(ROOT / 'tools/platforms/render-check.py'),
                '--binary', str(root / 'missing'), '--output', str(output)], text=True, capture_output=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn('headless binary is not a file', result.stderr)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
