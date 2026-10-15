"""Coherent record mutations test evidence checking, not native correctness."""
import copy
import json
import unittest

import verify


class IntegrationEvidenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original, cls.legacy = verify.load()

    def setUp(self):
        self.data = copy.deepcopy(self.original)

    def change(self, path, operation):
        row = self.data['records'][path]
        value = json.loads(row['text']); operation(value)
        raw = (json.dumps(value, indent=2)+'\n').encode()
        row.update(text=raw.decode(), bytes=len(raw), sha256=verify.digest(raw))

    def refused(self):
        with self.assertRaises(ValueError):
            verify.verify(self.data, self.legacy)

    def test_retained_integration_passes(self):
        self.assertEqual(verify.verify(self.data, self.legacy)['CPU_passed'], 14)

    def test_old_GPU_relabel_refused(self):
        self.data['outcome']['original_GPU_source'] = verify.HEAD
        self.refused()

    def test_Windows_execution_claim_refused(self):
        self.data['outcome']['Windows_runtime_executed'] = True
        self.refused()

    def test_rehashed_CPU_cleanup_failure_refused(self):
        self.change('cpu/evidence/cpu-worker-cooked-001/result.json', lambda v: v.update(cleanup_confirmed=False))
        self.refused()

    def test_coherent_Mac_budget_reset_refused(self):
        self.change('cpu/evidence/closeout.json', lambda v: v.update(remaining_build_seconds=1800-v['spent_seconds']['build']))
        self.refused()

    def test_rehashed_compile_success_without_strict_lint_refused(self):
        self.change('windows/run-001/results.json', lambda v: v['results'][1]['argv'].remove('warnings'))
        self.refused()

    def test_coherent_CORE_reset_refused(self):
        self.change('windows/closeout.json', lambda v: v.update(prior_charge_seconds=0, total_charge_seconds=26, remaining_original_sequence_seconds=3574))
        self.refused()

    def test_rehashed_current_native_source_change_refused(self):
        def mutation(v):
            row = next(row for row in v['integrated_files'] if row['path']=='crates/incant_render/src/model_material.wgsl')
            row['sha256'] = '0'*64
            v['changed_paths'].append(row['path'])
        self.change('windows/composition-reuse.json', mutation)
        self.refused()

    def test_coherent_audio_feature_misstatement_refused(self):
        def mutation(v):
            row = next(row for row in v['routes'] if row['stage']=='build-player-cooked-001')
            row['actual_compiler_artifact_features']['incant_audio_runtime']['features'].remove('device')
        self.change('cpu/evidence/actual-feature-closure.json', mutation)
        self.refused()

    def test_rehashed_checkout_restore_failure_refused(self):
        self.change('windows/checkout-restored.json', lambda v: v.update(worktree_clean=False))
        self.refused()


if __name__ == '__main__':
    unittest.main()
