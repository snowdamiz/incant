"""Exercise the gate CLI against isolated evidence, never the real approval ledger."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class PhaseZeroGate(unittest.TestCase):
    def run_gate(self, ledger, strict=True):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'tools').mkdir()
            (root / 'docs/gates').mkdir(parents=True)
            shutil.copyfile(Path(__file__).parents[1] / 'check_gate.py', root / 'tools/check_gate.py')
            (root / 'docs/gates/phase0.json').write_text(json.dumps(ledger))
            result = subprocess.run(
                [sys.executable, str(root / 'tools/check_gate.py')] + (['--strict'] if strict else []),
                capture_output=True, text=True, check=False,
            )
            return result.returncode, json.loads(result.stdout)

    def ready(self):
        return {
            'spikes': {'auth': {'passed': True}},
            'nightly_runnable_targets': {'ios': True},
            'year_one_staffing_confirmed': True,
            'director_approval': True,
        }

    def test_signing_is_no_longer_a_phase_zero_requirement(self):
        ledger = self.ready()
        for legacy_field in (False, True):
            if legacy_field:
                ledger['signing_certificates_obtained'] = False
            code, report = self.run_gate(ledger)
            self.assertEqual(code, 0)
            self.assertTrue(report['passed'])
            self.assertEqual(report['remaining'], [])

    def test_every_remaining_category_still_blocks_strict_gate(self):
        cases = [('spike:auth', 'spike'), ('nightly:ios', 'nightly'),
                 ('year_one_staffing_confirmed', 'field'), ('director_approval', 'field')]
        for expected, category in cases:
            with self.subTest(expected=expected):
                ledger = self.ready()
                if category == 'spike':
                    ledger['spikes']['auth']['passed'] = False
                elif category == 'nightly':
                    ledger['nightly_runnable_targets']['ios'] = False
                else:
                    ledger[expected] = False
                code, report = self.run_gate(ledger)
                self.assertEqual(code, 1)
                self.assertFalse(report['passed'])
                self.assertEqual(report['remaining'], [expected])

    def test_non_strict_reports_missing_approval_without_failing_the_command(self):
        ledger = self.ready()
        ledger['director_approval'] = False
        code, report = self.run_gate(ledger, strict=False)
        self.assertEqual(code, 0)
        self.assertFalse(report['passed'])
        self.assertEqual(report['remaining'], ['director_approval'])
