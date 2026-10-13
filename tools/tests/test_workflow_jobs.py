import json
import os
import re
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from check_workflow_jobs import validate

JOBS = ['windows-baseline', 'windows-optimized', 'windows-shipping', 'windows-composed']


class WorkflowJobTests(unittest.TestCase):
    def success(self):
        return {name: {'result': 'success', 'outputs': {}} for name in JOBS}

    def test_every_actual_success_passes(self):
        validate(self.success(), JOBS)

    def test_any_non_success_fails_at_each_child(self):
        for name in JOBS:
            for status in ('failure', 'skipped', 'cancelled', 'neutral', 'pending', '', None):
                with self.subTest(job=name, status=status):
                    needs = self.success()
                    needs[name]['result'] = status
                    with self.assertRaisesRegex(ValueError, 'did not succeed'):
                        validate(needs, JOBS)

    def test_missing_extra_or_malformed_results_fail(self):
        variants = [None, [], {}, {'unknown': {'result': 'success'}}]
        for name in JOBS:
            missing = self.success()
            del missing[name]
            variants.append(missing)
            for value in ({}, None, 'success'):
                malformed = self.success()
                malformed[name] = value
                variants.append(malformed)
        extra = self.success()
        extra['unexpected'] = {'result': 'success'}
        variants.append(extra)
        for needs in variants:
            with self.subTest(needs=needs), self.assertRaises(ValueError):
                validate(needs, JOBS)

    def test_empty_or_duplicate_requirements_cannot_pass(self):
        for required in ([], [JOBS[0], JOBS[0]]):
            with self.assertRaises(ValueError):
                validate(self.success(), required)

    def test_real_cli_preserves_failure_exit(self):
        for value, expected in ((json.dumps(self.success()), 0), ('{', 1), ('null', 1)):
            result = subprocess.run([sys.executable, ROOT / 'tools/check_workflow_jobs.py', *JOBS],
                                    env=dict(os.environ, INCANT_WORKFLOW_NEEDS=value),
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, expected, result.stderr)

    def test_workflow_keeps_all_proofs_and_always_success_only_gate(self):
        text = (ROOT / '.github/workflows/development-wasm.yml').read_text()
        gate = text.split('  windows-conformance:\n', 1)[1]
        self.assertIn('name: conformance (windows-latest)', gate)
        self.assertIn('if: ${{ always() }}', gate)
        self.assertIn('needs: [' + ', '.join(JOBS) + ']', gate)
        self.assertIn('INCANT_WORKFLOW_NEEDS: ${{ toJSON(needs) }}', gate)
        self.assertIn('python tools/check_workflow_jobs.py ' + ' '.join(JOBS), gate)
        commands = (
            'shipping-wasm2c.py artifacts/static-wasm-conformance --conformance-only',
            'shipping-wasm2c.py artifacts/static-wasm-optimized --conformance-only --msvc-opt-level 2',
            'shipping-wasm2c.py artifacts/static-wasm-shipping --msvc-opt-level 2',
            'static-game.py artifacts/static-game-shipping',
        )
        for name, command in zip(JOBS, commands):
            job = re.split(r'\n(?=  [a-z][a-z-]*:)', text.split(f'  {name}:\n', 1)[1], maxsplit=1)[0]
            self.assertIn('runs-on: windows-latest', job)
            self.assertIn('timeout-minutes: 90', job)
            self.assertIn(command, job)
            self.assertIn("cache-targets: 'false'", job)
            self.assertIn('if: always()', job)
            self.assertNotIn('continue-on-error:', job)
            self.assertNotIn('\n    if:', job)
        self.assertIn('runner: [macos-latest, ubuntu-latest]', text)


if __name__ == '__main__':
    unittest.main()
