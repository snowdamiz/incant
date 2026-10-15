"""Synthetic mutations test the verifier, not native rendering or performance."""
import copy
import json
import unittest

import verify


class EvidenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = verify.load()

    def setUp(self):
        self.data = copy.deepcopy(self.original)

    def change(self, path, action):
        row = self.data['records'][path]
        value = json.loads(row['text'])
        action(value)
        raw = (json.dumps(value, indent=2) + '\n').encode()
        row.update(text=raw.decode(), bytes=len(raw), sha256=verify.digest(raw))

    def refused(self):
        with self.assertRaises(ValueError):
            verify.verify(self.data)

    def test_exact_retained_packet_passes(self):
        self.assertEqual(verify.verify(self.data)['cpu_passed'], 28)

    def test_performance_claim_refused(self):
        self.data['outcome']['performance_gate_passed'] = True
        self.refused()

    def test_unobserved_process_claim_refused(self):
        self.data['outcome']['all_nested_pids_individually_observed'] = True
        self.refused()

    def test_headless_cli_execution_claim_refused(self):
        self.data['outcome']['headless_cli_executed'] = True
        self.refused()

    def test_rehashed_cleanup_failure_refused(self):
        self.change(verify.GPU + '/evidence/gpu-cooked-raster-001/result.json',
                    lambda v: v.update(cleanup_confirmed=False))
        self.refused()

    def test_rehashed_worker_main_thread_creation_refused(self):
        def mutate(value):
            value['rows'][-1]['phase'] = 'Gameplay'
        self.change(verify.GPU + '/pixels/ordinary/transition/worker-native-compilation.json', mutate)
        self.refused()

    def test_rehashed_published_without_real_selection_refused(self):
        self.change(verify.GPU + '/pixels/cooked/transition/worker-published.json',
                    lambda v: v['prewarm'].update(real_selections=0))
        self.refused()

    def test_rehashed_source_free_without_fallback_refused(self):
        self.change(verify.GPU + '/pixels/ordinary/player/source-free-player.json',
                    lambda v: v['render_input']['material_prewarm'].update(fallback_selections=0))
        self.refused()

    def test_rehashed_budget_reset_refused(self):
        self.change(verify.GPU + '/evidence/gpu-phase.json',
                    lambda v: v.update(prior_gpu_elapsed_seconds=0, remaining_gpu_seconds=180-v['phase_elapsed_seconds']))
        self.refused()

    def test_rehashed_original_failure_changed_to_success_refused(self):
        first = self.data['failures'][0]
        self.change(first['packet'] + '/evidence/' + first['stage'] + '/result.json',
                    lambda v: v.update(exit_code=0))
        self.refused()

    def test_rehashed_parity_claim_refused(self):
        self.change(verify.GPU + '/evidence/pixel-parity.json',
                    lambda v: v['pairs'][0].update(max_channel_byte_difference=1))
        self.refused()

    def test_reviewed_png_substitution_refused(self):
        self.change(verify.GPU + '/evidence/rendered-review.json',
                    lambda v: v['reviewed_pngs'][0].update(sha256='0' * 64))
        self.refused()


if __name__ == '__main__':
    unittest.main()
