"""Saved-data refusal controls only; no compiler/player is executed."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('core_pgo_portable', HERE / 'verify.py')
v = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(v)


class Replay(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.records = v.load(HERE)

    def changed(self, name, change):
        records = dict(self.records)
        data = v.decode(records[name])
        change(data)
        records[name] = json.dumps(data).encode()
        return records

    def refuse(self, name, change, message):
        with self.assertRaisesRegex(ValueError, message):
            v.validate(self.manifest, self.changed(name, change))

    def test_positive_raw_replay(self):
        result = v.validate(self.manifest, self.records)
        self.assertEqual(result['timed_frames'], 12224)
        self.assertEqual(result['timed_PCM_bytes'], 78233600)
        self.assertFalse(result['repeatable_win_improvement'])
        self.assertFalse(result['adopted'])

    def test_preserved_producer_failure(self):
        self.refuse('core-shipping-006/metadata/result.json', lambda x: x.update(status='complete', exit=0), 'Historical producer failure')

    def test_real_final_owner_source(self):
        self.refuse('core-cpu-012/closeout.json', lambda x: x.update(source=v.PRODUCER), 'Actual final owner')

    def test_no_generation_shipping_claim(self):
        self.refuse('core-shipping-007/packages/generate/build-receipt.json', lambda x: x.update(source_free_claim=True), 'Generation is nonshipping')

    def test_real_counters_required(self):
        def change(x):
            for field in ('symbol_artifact', 'packaged_artifact'):
                x['instrumentation'][field]['sections']['__llvm_prf_cnts']['bytes'] = 0
        self.refuse('core-shipping-007/packages/generate/build-receipt.json', change, 'Real profile payload')

    def test_no_cooker_profile_application(self):
        self.refuse('core-shipping-007/packages/generate/rquickjs-sys-cooker.json', lambda x: x.update(c_unit_profiled=True), 'Only runtime C unit profiled')

    def test_cooker_optimization_not_forced(self):
        self.refuse('core-shipping-007/packages/use/descriptor.json', lambda x: x['recipes']['cooker'].update(optimization=3), 'Actual role profile scope')

    def test_full_gameplay_parity(self):
        self.refuse('core-timing-002/cases/U0/stdout.json', lambda x: x.update(tick=2155), 'Timed full gameplay')

    def test_whole_pcm_identity(self):
        manifest = copy.deepcopy(self.manifest)
        row = next(r for r in manifest['external_artifacts'] if '/cases/U0/audio.pcm' in r['path'])
        row['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'Whole PCM identity'):
            v.validate(manifest, self.records)

    def test_ordinary_feature_graph_remains_ordinary(self):
        def change(x):
            x['pgo_feature_graph'][0]['features'].append('diagnostic-quickjs-use')
        self.refuse('core-shipping-007/packages/ordinary/build-receipt.json', change, 'Actual PGO feature graph')

    def test_ratio_recomputed_from_raw_histograms(self):
        def change(x):
            x['paired_p95_ratios']['0']['script.tick.p95_wall_ns']['lower_fraction'] = [1, 2]
        self.refuse('core-timing-002/metadata/result.json', change, 'Actual raw paired ratio')

    def test_histogram_quantile_cannot_be_replaced(self):
        def change(x):
            row = next(r for r in x['profile']['frame_wall']['phases'] if r['name'] == 'script.tick')
            row.update(p95_lower_ns=1, p95_upper_ns=10000)
        self.refuse('core-timing-002/cases/U0/stdout.json', change, 'Raw quantile mismatch')

    def test_histogram_overflow_cannot_be_omitted(self):
        def change(x):
            x['profile']['frame_wall']['phases'][0]['overflow_samples'] = 1
        self.refuse('core-timing-002/cases/U0/stdout.json', change, 'Histogram coverage')

    def test_cleanup_unconfirmed_refuses(self):
        self.refuse('core-timing-002/metadata/managed-calls.json', lambda x: x[0].update(cleanup_confirmed=False), 'Actual call cleanup')

    def test_original_wrapper_identity_not_invented(self):
        self.refuse('core-timing-001/prelaunch-closeout.json', lambda x: x.update(wrapper_PID_observed=True, actual_wrapper_PID=9287), 'First wrapper failure preserved')

    def test_no_adoption_overclaim(self):
        manifest = dict(self.manifest, adopted=True)
        with self.assertRaisesRegex(ValueError, 'Producer/default/adoption scope'):
            v.validate(manifest, self.records)

    def test_bounded_manifest_and_compressed_read(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'manifest.json').write_bytes(b'x' * (v.MANIFEST_CAP + 1))
            with self.assertRaisesRegex(ValueError, 'Input exceeds bound'):
                v.load(root)
            (root / 'manifest.json').write_bytes((HERE / 'manifest.json').read_bytes())
            (root / 'records.json.gz').write_bytes(b'x' * (v.PACKED_CAP + 1))
            with self.assertRaisesRegex(ValueError, 'Input exceeds bound'):
                v.load(root)

    def test_duplicate_and_nonfinite_json_refuse(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(ValueError):
                v.decode(raw)


if __name__ == '__main__':
    unittest.main()
