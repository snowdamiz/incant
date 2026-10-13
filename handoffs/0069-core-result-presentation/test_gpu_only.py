"""CPU-only admission/cleanup controls. Never calls a renderer or GPU executable."""
import contextlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('gpu_only', HERE / 'gpu_only.py')
gpu = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gpu)


class GuardTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.cpu = self.root / '0069'
        self.cpu.mkdir()
        (self.cpu / 'release.json').write_text('{}\n')
        self.output = self.root / '0069-gpu-test'
        self.grant = self.root / 'root-authority.json'
        self.release = self.root / 'prior-gpu-release.json'
        self.release.write_text('{"released": true}\n')
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(mock.patch.object(gpu, 'CPU', self.cpu))
        for module in (gpu.lease, gpu.capture, gpu.regression):
            self.stack.enter_context(mock.patch.dict(vars(module), {}))

    def authority(self):
        value = {'packet': gpu.lease.PACKET, 'phase': gpu.PHASE, 'gpu_only': True, 'cpu_only': False,
                 'source_commit': gpu.SOURCE, 'freeze_sha256': gpu.FREEZE_SHA,
                 'cpu_manifest_sha256': gpu.MANIFEST_SHA, 'wrapper_sha256': gpu.lease.digest(gpu.__file__),
                 'brief_sha256': gpu.lease.digest(gpu.HERE / 'brief.md'),
                 'evidence': str(self.output), 'target': str(self.output / 'unused-cargo-target'),
                 'ceilings': gpu.lease.CEILINGS, 'planned_processes': 18,
                 'cpu_release_sha256': gpu.lease.digest(self.cpu / 'release.json'),
                 'holder': 'actual test holder (CPU only; no GPU execution)',
                 'predecessor_release': {'path': str(self.release), 'sha256': gpu.lease.digest(self.release)}}
        self.grant.write_text(json.dumps(value))
        return value

    def test_authority_is_explicit_and_cannot_reopen_cpu_namespace(self):
        expected = self.authority()
        self.assertEqual(gpu.authority(self.grant, self.output), expected)
        self.assertFalse(self.output.exists())
        with self.assertRaises(gpu.lease.Refused):
            gpu.authority(self.grant, self.cpu)
        self.grant.unlink()
        with self.assertRaises(gpu.lease.Refused):
            gpu.authority(self.grant, self.output)

    def test_stale_cpu_only_wrong_source_and_bool_alias_authorities_refuse(self):
        for key, value in [('gpu_only', 1), ('cpu_only', True), ('source_commit', 'other'),
                           ('planned_processes', 19), ('wrapper_sha256', 'other'),
                           ('brief_sha256', 'stale instructions'), ('holder', '')]:
            with self.subTest(key=key):
                body = self.authority()
                body[key] = value
                self.grant.write_text(json.dumps(body))
                with self.assertRaises(gpu.lease.Refused):
                    gpu.authority(self.grant, self.output)
                self.assertFalse(self.output.exists())

    def test_seed_selects_copied_binary_record_and_preserves_cpu_ledger(self):
        for d in ('freeze', 'regression', 'shipping-002', 'packages'):
            (self.cpu / d).mkdir()
        (self.cpu / 'freeze/generation-5.json').write_text('{"generation":5}')
        copied = b'{"executables":{"test":{"path":"/retained/copy"}}}'
        (self.cpu / 'regression/frozen-build-g5.json').write_bytes(copied)
        (self.cpu / 'regression/build-g5.json').write_text('DO NOT USE LIVE TARGET')
        (self.cpu / 'ledger.jsonl').write_text('sealed\n')
        route = self.cpu / 'shipping-002/example-input.json'
        route.write_text('{"ticks":1}')
        with mock.patch.object(gpu, 'ROUTES', {'example': gpu.lease.digest(route)}):
            gpu.seed(self.output, {'runs': {'case': {'route': 'example'}}})
        self.assertEqual((self.output / 'regression/build-g5.json').read_bytes(), copied)
        self.assertEqual((self.cpu / 'ledger.jsonl').read_text(), 'sealed\n')
        self.assertEqual((self.output / 'routes/example.json').read_bytes(), route.read_bytes())
        with self.assertRaises(FileExistsError):
            gpu.seed(self.output, {'runs': {}})

    def test_configured_runner_rejects_cpu_extra_argv_wrong_debit_and_strips_fake_home(self):
        original = mock.Mock(return_value=(0, 'stdout', 'stderr'))
        gpu.lease.run = original
        case = {'id': 'one', 'binary': 'bin', 'test': 'exact::one', 'frames': 2}
        gpu.regression.manifest = lambda: {'cases': [case]}
        gpu.configure(self.output, self.grant, {'runs': {}},
                      {'executables': {'bin': {'path': '/copied/exe', 'package': 'incant_render'}}})
        argv = ['/copied/exe', '--exact', 'exact::one', '--ignored', '--test-threads=1', '--nocapture']
        options = {'label': 'regression-g5-one', 'timeout': 120,
                   'cwd': gpu.ROOT / 'crates/incant_render', 'debit': ('regression', 'regression-g5-one', 2),
                   'gpu': True, 'env': {'PATH': '/usr/bin:/bin', 'HOME': '/fake',
                                       'TMPDIR': str(self.output / 'scratch/run')}}
        for changed in ({'gpu': False}, {'debit': ('regression', 'regression-g5-one', 1)},
                        {'timeout': 121}, {'label': 'unplanned'}):
            with self.assertRaises(gpu.lease.Refused):
                gpu.lease.run(argv, **{**options, **changed})
        with self.assertRaises(gpu.lease.Refused):
            gpu.lease.run(argv + ['extra'], **options)
        original.assert_not_called()
        gpu.lease.run(argv, **options)
        self.assertNotIn('HOME', original.call_args.kwargs['env'])

    def execution(self, error=None, cleanup_error=None):
        frozen = {'generation': 5}
        plans = {'runs': {'one': {'frame': 1}}}
        self.stack.enter_context(mock.patch.object(gpu, 'inputs', return_value=(frozen, plans, {})))
        self.stack.enter_context(mock.patch.object(gpu, 'authority', return_value={}))
        self.stack.enter_context(mock.patch.object(gpu, 'seed'))
        self.stack.enter_context(mock.patch.object(gpu, 'configure'))
        gpu.lease.LEASE = self.output / 'lease.json'
        gpu.lease.acquire = mock.Mock()
        gpu.lease.release = mock.Mock(side_effect=cleanup_error)
        gpu.regression.run_all = mock.Mock(return_value=[{'passed': True}], side_effect=error)
        gpu.capture.run = mock.Mock()
        gpu.lease.frames_state = lambda: {'debited': {'appearance': 22, 'regression': 39}, 'runs': [0] * 18}
        return gpu.lease.release

    def test_success_releases_after_captures(self):
        release = self.execution()
        gpu.execute(self.output, self.grant)
        release.assert_called_once()
        gpu.capture.run.assert_called_once()

    def test_interrupt_and_release_failure_preserve_original_exception(self):
        original = KeyboardInterrupt('original')
        release = self.execution(original, RuntimeError('cleanup'))
        with self.assertRaises(KeyboardInterrupt) as raised:
            gpu.execute(self.output, self.grant)
        self.assertIs(raised.exception, original)
        self.assertIn('authority remains held', original.__notes__[0])
        release.assert_called_once()
        gpu.capture.run.assert_not_called()

    def test_regression_failure_prevents_all_appearance_and_releases(self):
        release = self.execution()
        gpu.regression.run_all.return_value = [{'passed': False}]
        with self.assertRaises(gpu.lease.Refused):
            gpu.execute(self.output, self.grant)
        release.assert_called_once()
        gpu.capture.run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
