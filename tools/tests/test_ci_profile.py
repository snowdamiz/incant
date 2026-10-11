"""Guard the functional CI graph and its distinct fresh shipping configuration."""
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class FunctionalProfile(unittest.TestCase):
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
