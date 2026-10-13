"""CPU-only g6 guard tests. No renderer or GPU executable can be launched."""
import contextlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
spec = importlib.util.spec_from_file_location('gpu_only_g6',HERE/'gpu_only_g6.py')
gpu = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gpu)


class AdmissionTests(unittest.TestCase):
    def test_real_inputs_and_only_brief_exclusion(self):
        gpu.inputs()
        original = gpu.lease.digest
        # Scheduling copy may change; authority independently binds its new hash.
        with mock.patch.object(gpu.lease,'digest',side_effect=lambda p: 'new brief' if Path(p)==HERE/'brief.md' else original(p)):
            self.assertEqual(gpu.source_inputs()['generation'],6)
        for rel in (gpu.TEST_SOURCE,'crates/incant_render/src/ui/ui.wgsl','Cargo.lock'):
            with self.subTest(path=rel), mock.patch.object(gpu.lease,'digest',side_effect=lambda p: 'changed' if Path(p)==gpu.ROOT/rel else original(p)):
                with self.assertRaises(gpu.lease.Refused):gpu.source_inputs()

    def test_new_build_manifest_and_executable_mutations_refuse(self):
        original = gpu.lease.digest
        for rel in ('frozen-build-g6.json','regression-g6.json','frozen-binaries/generation-6/render-lib'):
            with self.subTest(path=rel), mock.patch.object(gpu.lease,'digest',side_effect=lambda p: 'changed' if Path(p)==gpu.CORRECTION/rel else original(p)):
                with self.assertRaises(gpu.lease.Refused):gpu.inputs()
        with self.assertRaises(gpu.lease.Refused):gpu.source_inputs(new=True)


class GuardTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.cpu=self.root/'0069';self.cpu.mkdir()
        self.correction=self.root/'correction';self.correction.mkdir()
        (self.cpu/'release.json').write_text('{}')
        (self.correction/'cpu-release.json').write_text('{}')
        self.output=self.root/'0069-gpu-003';self.grant=self.root/'authority.json'
        self.prior=self.root/'prior-release.json';self.prior.write_text('{}')
        self.stack=contextlib.ExitStack();self.addCleanup(self.stack.close)
        for module in (gpu.lease,gpu.capture,gpu.regression):self.stack.enter_context(mock.patch.dict(vars(module),{}))
        for key,value in [('CPU',self.cpu),('CORRECTION',self.correction),('EVIDENCE',self.output)]:
            self.stack.enter_context(mock.patch.object(gpu,key,value))

    def grant_body(self):
        value={'packet':gpu.lease.PACKET,'phase':gpu.PHASE,'gpu_only':True,'cpu_only':False,
               'source_commit':gpu.SOURCE,'freeze_sha256':gpu.FREEZE_SHA,'build_sha256':gpu.BUILD_SHA,
               'case_manifest_sha256':gpu.CASES_SHA,'package_freeze_sha256':gpu.old.FREEZE_SHA,
               'cpu_manifest_sha256':gpu.old.MANIFEST_SHA,'wrapper_sha256':gpu.lease.digest(gpu.__file__),
               'original_wrapper_sha256':gpu.OLD_WRAPPER_SHA,'evidence':str(self.output),
               'brief_sha256':gpu.lease.digest(HERE/'brief.md'),'target':str(self.output/'unused-cargo-target'),
               'ceilings':gpu.lease.CEILINGS,'planned_processes':18,
               'cpu_release_sha256':gpu.lease.digest(self.cpu/'release.json'),
               'correction_release_sha256':gpu.lease.digest(self.correction/'cpu-release.json'),
               'holder':'CPU test only; no GPU authority','predecessor_release':{'path':str(self.prior),'sha256':gpu.lease.digest(self.prior)}}
        self.grant.write_text(json.dumps(value));return value

    def test_fresh_authority_binds_both_generations_and_actual_holder(self):
        value=self.grant_body();self.assertEqual(gpu.authority(self.grant,self.output),value)
        for key,changed in [('phase',gpu.old.PHASE),('gpu_only',1),('cpu_only',True),('holder',''),
                            ('source_commit',gpu.old.SOURCE),('freeze_sha256',gpu.old.FREEZE_SHA),
                            ('build_sha256','old build'),('case_manifest_sha256','old manifest'),
                            ('brief_sha256','old instructions'),('package_freeze_sha256','relabelled'),
                            ('planned_processes',19),('wrapper_sha256','other'),('original_wrapper_sha256','other')]:
            with self.subTest(key=key):
                v=self.grant_body();v[key]=changed;self.grant.write_text(json.dumps(v))
                with self.assertRaises(gpu.lease.Refused):gpu.authority(self.grant,self.output)
        self.grant_body()
        with self.assertRaises(gpu.lease.Refused):gpu.authority(self.grant,self.cpu)
        self.assertFalse(self.output.exists())

    def test_seed_keeps_g5_and_g6_records_separate(self):
        def old_seed(evidence,plans):
            evidence.mkdir();(evidence/'freeze').mkdir();(evidence/'regression').mkdir()
            (evidence/'freeze/generation-5.json').write_text('original5')
        for name in ('freeze-g6.json','frozen-build-g6.json','regression-g6.json'):
            (self.correction/name).write_text(name)
        with mock.patch.object(gpu.old,'seed',side_effect=old_seed):gpu.seed(self.output,{})
        self.assertEqual((self.output/'freeze/generation-5.json').read_text(),'original5')
        self.assertEqual((self.output/'freeze/generation-6.json').read_text(),'freeze-g6.json')
        self.assertEqual((self.output/'regression/build-g6.json').read_text(),'frozen-build-g6.json')

    def test_runner_refuses_cpu_and_changed_args_debit_env(self):
        (self.correction/'regression-g6.json').write_text(json.dumps({'cases':[{'id':'one','binary':'bin','test':'one','frames':2}]}))
        launch=mock.Mock(return_value=(0,'out','err'));gpu.lease.run=launch
        gpu.configure(self.output,self.grant,{'runs':{}},{'executables':{'bin':{'path':'/frozen/exe','package':'incant_render'}}})
        argv=['/frozen/exe','--exact','one','--ignored','--test-threads=1','--nocapture']
        options={'label':'regression-g6-one','timeout':120,'cwd':gpu.ROOT/'crates/incant_render',
                 'debit':('regression','regression-g6-one',2),'gpu':True,
                 'env':{'PATH':'/usr/bin:/bin','TMPDIR':str(self.output/'scratch/one'),'HOME':'/fake'}}
        for changed in ({'gpu':False},{'label':'regression-g5-one'},{'timeout':121},
                        {'debit':('regression','regression-g6-one',1)},
                        {'env':{**options['env'],'WGPU_BACKEND':'metal'}},
                        {'env':{**options['env'],'TMPDIR':str(self.output/'scratch/../elsewhere')}}):
            with self.assertRaises(gpu.lease.Refused):gpu.lease.run(argv,**{**options,**changed})
        with self.assertRaises(gpu.lease.Refused):gpu.lease.run(argv+['extra'],**options)
        launch.assert_not_called();gpu.lease.run(argv,**options)
        self.assertNotIn('HOME',launch.call_args.kwargs['env'])

    def test_capture_keeps_g5_but_checks_g6_and_restores_on_exception(self):
        (self.cpu/'freeze').mkdir();(self.cpu/'freeze/generation-5.json').write_text('{"identity":{"tools":{}}}')
        original=mock.Mock(return_value={'generation':6});gpu.lease.freeze=original
        def capture(name,spec):
            self.assertEqual(gpu.lease.freeze()['generation'],5)
            raise KeyboardInterrupt('original')
        gpu.capture.run=capture
        with self.assertRaises(KeyboardInterrupt):gpu.capture_original('one',{})
        original.assert_called_once_with(False);self.assertIs(gpu.lease.freeze,original)
        gpu.lease.freeze=mock.Mock(side_effect=gpu.lease.Refused('source changed'))
        with self.assertRaises(gpu.lease.Refused):gpu.capture_original('one',{})

    def execute_setup(self,error=None):
        for name,value in [('inputs',({'generation':6},{'runs':{'one':{}}},{})),('authority',{}),('seed',None),('configure',None)]:
            self.stack.enter_context(mock.patch.object(gpu,name,return_value=value))
        gpu.lease.LEASE=self.output/'lease.json';gpu.lease.acquire=mock.Mock();gpu.lease.release=mock.Mock()
        gpu.regression.run_all=mock.Mock(return_value=[{'passed':True}],side_effect=error)
        self.stack.enter_context(mock.patch.object(gpu,'capture_original'))
        gpu.lease.frames_state=lambda:{'debited':{'appearance':22,'regression':39},'runs':[0]*18}

    def test_success_and_regression_failure_release(self):
        self.execute_setup();gpu.execute(self.output,self.grant)
        gpu.capture_original.assert_called_once();gpu.lease.release.assert_called_once()
        gpu.capture_original.reset_mock();gpu.lease.release.reset_mock();gpu.regression.run_all.return_value=[{'passed':False}]
        with self.assertRaises(gpu.lease.Refused):gpu.execute(self.output,self.grant)
        gpu.capture_original.assert_not_called();gpu.lease.release.assert_called_once()

    def test_interruption_preserves_original_and_cleanup_failure(self):
        original=KeyboardInterrupt('original');self.execute_setup(original)
        gpu.lease.release.side_effect=RuntimeError('cleanup')
        with self.assertRaises(KeyboardInterrupt) as raised:gpu.execute(self.output,self.grant)
        self.assertIs(raised.exception,original);self.assertIn('authority remains held',original.__notes__[0])
        gpu.capture_original.assert_not_called();gpu.lease.release.assert_called_once()

if __name__=='__main__':unittest.main()
