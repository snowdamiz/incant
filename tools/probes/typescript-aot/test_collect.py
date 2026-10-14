"""Observable fail-closed controls for retained AOT language evidence."""
import copy
import json
import unittest
import collect


class EvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.files = collect.load()

    def changed_result(self, change):
        files = self.files.copy()
        result = json.loads(files['run-002/result.json'])
        change(result)
        files['run-002/result.json'] = json.dumps(result).encode()
        return files

    def test_complete_language_receipt(self):
        collect.validate_results(self.files)

    def test_claimed_adoption_refused(self):
        with self.assertRaises(AssertionError):
            collect.validate_results(self.changed_result(lambda r: r['decision18'].update(adopted=True)))

    def test_missing_case_refused(self):
        with self.assertRaises(AssertionError):
            collect.validate_results(self.changed_result(lambda r: r['cases'].pop()))

    def test_failed_kernel_refused(self):
        with self.assertRaises(AssertionError):
            collect.validate_results(self.changed_result(lambda r: r['cases'][-1].update(assembly_exit=1)))

    def test_false_frontend_claim_refused(self):
        with self.assertRaises(AssertionError):
            collect.validate_results(self.changed_result(lambda r: r['sdk_frontend'].update(passed=False)))

    def test_changed_compiler_log_refused(self):
        files = self.files.copy()
        files['run-002/closure.stderr'] = b''
        with self.assertRaises(AssertionError):
            collect.validate_results(files)

    def test_missing_historical_record_refused(self):
        files = self.files.copy()
        files.pop('run-001/caught-error.stderr')
        with self.assertRaises((AssertionError, KeyError)):
            collect.verify(self.manifest, files)

    def test_changed_payload_refused(self):
        files = self.files.copy()
        files['run-002/bulk-kernel.wasm'] += b'changed'
        with self.assertRaises(AssertionError):
            collect.verify(self.manifest, files)

    def test_changed_source_identity_refused(self):
        manifest = copy.deepcopy(self.manifest)
        manifest['sources']['sdk/ts/src/native.d.ts'] = '0' * 64
        with self.assertRaises(AssertionError):
            collect.verify(manifest, self.files)


if __name__ == '__main__':
    unittest.main()
