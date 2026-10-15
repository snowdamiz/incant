"""Coherent data mutations must not turn refusals or fallback into archive hits."""
import copy
import json
import unittest

import verify


class EvidenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = verify.load()

    def changed_record(self, path, mutate):
        data = copy.deepcopy(self.original)
        row = data['records'][path]
        row['text'] = mutate(row['text'])
        raw = row['text'].encode()
        row.update(bytes=len(raw), sha256=verify.digest(raw))
        return data

    def test_retained_packet_verifies_without_native_execution(self):
        self.assertFalse(verify.verify(self.original)['archived_code_executed'])

    def test_performance_claim_is_refused(self):
        data = copy.deepcopy(self.original)
        data['outcome']['performance_measured'] = True
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_hidden_historical_failure_is_refused(self):
        def mutate(text):
            row = json.loads(text)
            row['exit_code'] = 0
            return json.dumps(row)
        data = self.changed_record('evidence/build-001/result.json', mutate)
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_ordinary_creation_cannot_replace_strict_catalog_success(self):
        path = verify.GPU + '/evidence/gpu-ordinary-catalog-reuse-001/stderr.log'
        data = self.changed_record(path, lambda text: text.replace('require_succeeded: 43', 'require_succeeded: 0'))
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_capture_during_cooked_reuse_is_refused(self):
        path = verify.GPU + '/evidence/gpu-cooked-catalog-reuse-001/stderr.log'
        data = self.changed_record(path, lambda text: text.replace('render_added: 0', 'render_added: 41'))
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_missing_archive_acceptance_is_refused(self):
        path = verify.GPU + '/evidence/gpu-ordinary-empty-001/stderr.log'
        data = self.changed_record(path, lambda text: text.replace('require_failed: 2', 'require_failed: 0'))
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_disabled_native_calls_are_refused(self):
        path = verify.GPU + '/evidence/gpu-ordinary-disabled-001/stderr.log'
        data = self.changed_record(path, lambda text: text.replace('render_attached: 0', 'render_attached: 1'))
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_checksum_rejection_cannot_replace_native_parser_rejection(self):
        path = verify.GPU + '/evidence/gpu-ordinary-catalog-invalid-001/stderr.log'
        data = self.changed_record(path, lambda text: text.replace('MTLBinaryArchiveDomain', 'HostChecksum'))
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_cooked_source_identity_cannot_be_substituted(self):
        path = verify.GPU + '/evidence/gpu-cooked-catalog-reuse-001/stderr.log'
        data = self.changed_record(path, lambda text: text.replace(
            '60ec8225b5947537ad61fe1855e933797b5a105a1259e1143744c90c9f13f434',
            'f1e58dc20441680ea4458473ed3b113d2294b6050cb78061426756e7088f6e83'))
        with self.assertRaises(ValueError):
            verify.verify(data)

    def test_changed_hal_features_cannot_reuse_lint(self):
        path = verify.CPU + '/evidence/hal-resolved-closures.json'
        def mutate(text):
            row = json.loads(text)
            graph = row['cooked']['new']
            key = next(k for k in graph if '/third_party/wgpu-hal#' in k)
            graph[key]['features'].append('unqualified-feature')
            return json.dumps(row)
        with self.assertRaises(ValueError):
            verify.verify(self.changed_record(path, mutate))

    def test_gpu_budget_reset_is_refused(self):
        path = verify.GPU + '/continuation.json'
        def mutate(text):
            row = json.loads(text)
            row['gpu_total_seconds'] = 180
            return json.dumps(row)
        with self.assertRaises(ValueError):
            verify.verify(self.changed_record(path, mutate))


if __name__ == '__main__':
    unittest.main()
