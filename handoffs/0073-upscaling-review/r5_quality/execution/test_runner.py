"""CPU-only refusal/artifact/lifecycle controls. Never launches a renderer."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zlib

sys.dont_write_bytecode = True
P = Path(__file__).resolve().with_name('runner.py')
s = importlib.util.spec_from_file_location('quality_runner', P)
r = importlib.util.module_from_spec(s)
s.loader.exec_module(r)
GOOD = 'running 1 test\ntest CPU_only ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out\n'


class Controls(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='owner-quality-CPU-')
        self.root = Path(self.tmp.name)
        self.e = self.root / 'evidence'
        self.e.mkdir()
        self.addCleanup(self.tmp.cleanup)

    def binary(self, body=None):
        p = self.root / 'CPU-process'
        ordinary = 'running 1 test\nupscaling adapter: AdapterInfo { name: "CPU fixture", backend: Metal }\n'
        ordinary += ''.join(f'upscaling mode{mode} attachment{a}: [0.0]\n' for mode in range(19) for a in range(2))
        ordinary += GOOD.split('test CPU_only ... ok\n')[1]
        owner = 'running 1 test\nOWNER_HISTORY adapter=AdapterInfo { name: "CPU fixture", backend: Metal }\n'
        owner += ''.join(f'OWNER_HISTORY case={case} center=[] probes=1 baseline={str(case in [7, 8]).lower()} current_only={str(case == 3).lower()}\n' for case in range(1, 13))
        owner += r.MARKER + '\n' + GOOD.split('test CPU_only ... ok\n')[1]
        script = ('import sys\narg = " ".join(sys.argv)\nprint(' + repr(ordinary) + ' if ' + repr(r.ORDINARY) + ' in arg else ' + repr(owner) + ' if ' + repr(r.OWNER) + ' in arg else ' + repr(GOOD) + ', end="")')
        p.write_text('#!' + sys.executable + '\n' + (body or script) + '\n')
        p.chmod(0o555)
        return dict(path=str(p), sha256=r.life.sha(p), bytes=p.stat().st_size, mode='0555')

    def setup_plan(self, row):
        q = r.quality()
        plan = dict(state='FROZEN_READY_FOR_ROOT_GRANT', binary=row, numeric_binaries=[row,row], groups=r.ORDER,
                    budget=r.budget(q, r.ORDER))
        path = self.root / 'plan.json'
        path.write_text(json.dumps(plan))
        (self.e / 'grant.json').write_text('{"CPU_fixture_only":true}')
        return path, plan, q

    def execute_fixture(self, row, adapter=None):
        path, plan, q = self.setup_plan(row)
        def observed(directory, group, spec):
            # Lifecycle control only. Artifact acceptance is tested separately below.
            return {'adapter': {'backend': 'Metal', 'name': (adapter or {}).get(group, 'CPU fixture')}, 'pngs': []}
        with patch.object(r, 'E', self.e), patch.object(r, 'GLOBAL_LOCK', self.root / 'lock'), \
             patch.object(r, 'read_plan', return_value=(plan, q)), patch.object(r, 'grant', return_value=({}, {})), \
             patch.object(r, 'artifacts', side_effect=observed):
            return r.execute(path)

    def test_real_frozen_plan_readonly(self):
        plan, _ = r.read_plan(P.with_name('plan-ready.json'))
        self.assertEqual(plan['budget']['queue_submissions'], 840)
        self.assertEqual(plan['budget']['readback_maps'], 442)
        self.assertEqual(plan['budget']['pngs'], 15)

    def test_plan_mutations_refused(self):
        original = json.loads(P.with_name('plan-ready.json').read_text())
        path = self.root / 'mutated.json'
        for mutate in [lambda p: p['groups'].reverse(), lambda p: p['groups'].pop(),
                       lambda p: p['budget'].update(queue_submissions=779),
                       lambda p: p['environment'].update(WGPU_BACKEND='vulkan'),
                       lambda p: p.update(producer='a'*40), lambda p: p.update(state='EXECUTE'),
                       lambda p: p['fixture'].update(sha256='a'*64),
                       lambda p: p['binary'].update(mode='0644'), lambda p: p['numeric_order'].reverse(),
                       lambda p: p['numeric_order'][0].__setitem__(3,20), lambda p: p['numeric_binaries'][0].update(sha256='a'*64),
                       lambda p: p['numeric_evidence']['freeze'].update(sha256='a'*64)]:
            bad = copy.deepcopy(original); mutate(bad); path.write_text(json.dumps(bad))
            with self.assertRaises(r.life.Refused): r.read_plan(path)

    def test_grant_mutations_refused(self):
        path, plan, _ = self.setup_plan(self.binary())
        prior = self.root / 'release.json'
        prior.write_text(json.dumps(dict(released=True, owned_processes=[])))
        good = dict(schema='incant-r5-numeric-quality-grant-v1', authority='root', holder='Claude0073 Opus5.5 Max',
                    issued_at='2026-10-13', plan_sha256=r.life.sha(path), runner_sha256=r.life.sha(r.HERE),
                    lifecycle_sha256=r.LIFECYCLE_SHA, binary_sha256=plan['binary']['sha256'],
                    numeric_binary_sha256=[row['sha256'] for row in plan['numeric_binaries']], evidence_dir=str(self.e),
                    groups=plan['groups'], budget=plan['budget'], predecessor_release_path=str(prior),
                    predecessor_release_sha256=r.life.sha(prior))
        def check(value):
            (self.e / 'grant.json').write_text(json.dumps(value))
            with patch.object(r, 'E', self.e): return r.grant(path, plan)
        check(good)
        for key, value in [('authority','claude'), ('holder','other'), ('runner_sha256','a'*64),
                           ('lifecycle_sha256','a'*64), ('plan_sha256','a'*64), ('binary_sha256','a'*64),
                           ('numeric_binary_sha256',[]), ('budget',{}), ('groups',r.ORDER[::-1]), ('predecessor_release_sha256','a'*64)]:
            bad = copy.deepcopy(good); bad[key] = value
            with self.assertRaises(r.life.Refused): check(bad)

    def test_no_grant_no_side_effect(self):
        with patch.object(r, 'E', self.e):
            with self.assertRaises(r.life.Refused): r.execute(self.root / 'absent')
        self.assertEqual(list(self.e.iterdir()), [])

    def test_nine_actual_cpu_children_release_and_no_retry(self):
        row = self.binary(); result = self.execute_fixture(row)
        self.assertTrue(result['accepted']); self.assertTrue(result['released'])
        self.assertEqual([c['group'] for c in result['children']], [j[0] for j in r.NUMERIC_JOBS]+r.ORDER)
        self.assertEqual(result['owned_processes'], [])
        debits = [json.loads(x) for x in (self.e / 'debits.jsonl').read_text().splitlines()]
        self.assertEqual(sum(x['queue_submissions'] for x in debits), 840)
        self.assertEqual(sum(x['readback_maps'] for x in debits), 442)
        for child in result['children']:
            with self.assertRaises(ProcessLookupError): os.kill(child['pid'], 0)
        with self.assertRaises(r.life.Refused): self.execute_fixture(row)

    def test_first_failure_stops_before_second_debit(self):
        result = self.execute_fixture(self.binary("print('early unsupported')"))
        self.assertTrue(result['released']); self.assertFalse(result['accepted'])
        self.assertEqual(len(result['children']), 1)
        self.assertEqual(len((self.e / 'debits.jsonl').read_text().splitlines()), 1)

    def test_adapter_mismatch_stops_at_quality_motion(self):
        result = self.execute_fixture(self.binary(), {'motion': 'Other CPU fixture'})
        self.assertFalse(result['accepted']); self.assertTrue(result['released'])
        self.assertEqual(len(result['children']), 6)
        self.assertTrue(result['children'][5]['adapter_mismatch'])

    def test_timeout_reaps_actual_child(self):
        with patch.object(r, 'TIMEOUT', .15):
            result = self.execute_fixture(self.binary('import time; time.sleep(20)'))
        self.assertTrue(result['released']); self.assertFalse(result['accepted'])
        self.assertTrue(result['children'][0]['timed_out'])

    def test_base_exception_reaps_and_retains(self):
        class Abort(BaseException): pass
        count = 0
        real = r.life.signal.sigpending
        def pending():
            nonlocal count
            count += 1
            if count == 2: raise Abort('CPU control')
            return real()
        with patch.object(r.life.signal, 'sigpending', pending):
            with self.assertRaises(Abort): self.execute_fixture(self.binary('import time; time.sleep(20)'))
        result = json.loads((self.e / 'release.json').read_text())
        self.assertTrue(result['released']); self.assertIn('Abort', result['exception'])

    def test_signal_reaps_actual_child(self):
        sender = self.root / 'sender.py'
        sender.write_text('import time,os,signal\ntime.sleep(.3)\nos.kill(' + str(os.getpid()) + ',signal.SIGTERM)\n')
        previous = signal.signal(signal.SIGTERM, r.life.raise_interrupt)
        process = subprocess.Popen([sys.executable, str(sender)])
        try:
            with self.assertRaises(KeyboardInterrupt): self.execute_fixture(self.binary('import time; time.sleep(20)'))
        finally:
            process.wait(); signal.signal(signal.SIGTERM, previous)
        result = json.loads((self.e / 'release.json').read_text())
        self.assertTrue(result['released']); self.assertEqual(result['children'][0]['interrupted_by'], ['SIGTERM'])

    def test_busy_global_lock_prevents_debit(self):
        path, plan, q = self.setup_plan(self.binary())
        lockpath = self.root / 'global.lock'
        with open(lockpath, 'a') as held:
            r.fcntl.flock(held, r.fcntl.LOCK_EX | r.fcntl.LOCK_NB)
            with patch.object(r,'E',self.e), patch.object(r,'GLOBAL_LOCK',lockpath), \
                 patch.object(r,'read_plan',return_value=(plan,q)), patch.object(r,'grant',return_value=({},{})):
                with self.assertRaises(r.life.Refused): r.execute(path)
        self.assertFalse((self.e / 'debits.jsonl').exists())

    def test_binary_mode_and_hash_refusals(self):
        row = self.binary(); r.life.check_binary(row)
        Path(row['path']).chmod(0o644)
        with self.assertRaises(r.life.Refused): r.life.check_binary(row)
        Path(row['path']).chmod(0o555)
        with self.assertRaises(r.life.Refused): r.life.check_binary({**row, 'sha256':'a'*64})

    def artifact_fixture(self):
        group = 'motion'; spec = r.quality()['groups'][group]
        directory = self.root / 'artifact-control'; root = directory / 'artifacts'; root.mkdir(parents=True)
        def chunk(kind, payload):
            return struct.pack('>I',len(payload)) + kind + payload + struct.pack('>I',zlib.crc32(kind+payload)&0xffffffff)
        # Structural PNG metadata only; this is not an executed renderer or pixel proof.
        raw = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR',struct.pack('>IIBBBBB',960,540,8,6,0,0,0)) + chunk(b'IDAT',b'') + chunk(b'IEND',b'')
        pngs = []
        for name in spec['pngs']:
            (root / name).write_bytes(raw)
            pngs.append(dict(name=name, sha256=r.life.sha(root / name), rgba_sha256='a'*64))
        report = dict(case=group,outcome='pass',checks={key:True for key in r.expected_checks(group)},
                      budget_limit=spec['budget'],budget_used=spec['budget'],adapter=dict(backend='Metal',name='CPU metadata'),pngs=pngs)
        events = [dict(event='open',case=group,budget=spec['budget'])]
        for i in range(1,spec['budget']['scene_submissions']+1):
            name='shot'+str(i)
            events += [dict(event='charge',kind=kind,index=i,name=name) for kind in ['Scene','Readback']]
            events.append(dict(event='shot',name=name,ui=False,rgba_sha256='a'*64))
        events += [dict(event='png',png=p) for p in pngs] + [dict(event='finish',**{'pass':True})]
        (root/'report.json').write_text(json.dumps(report))
        (root/'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in events))
        return directory,group,spec,report,events

    def test_exact_artifact_metadata_and_mutations(self):
        directory,group,spec,report,events = self.artifact_fixture(); root=directory/'artifacts'
        result = r.artifacts(directory,group,spec)
        self.assertEqual(result['shots'],44); self.assertEqual(len(result['pngs']),4)
        for mutate in [lambda p:p.update(outcome='fail'), lambda p:p['checks'].pop(next(iter(p['checks']))),
                       lambda p:p['checks'].update(extra=True), lambda p:p.update(budget_used={}),
                       lambda p:p['adapter'].update(backend='Vulkan'), lambda p:p['pngs'][0].update(sha256='b'*64)]:
            bad=copy.deepcopy(report);mutate(bad);(root/'report.json').write_text(json.dumps(bad))
            with self.assertRaises(r.life.Refused):r.artifacts(directory,group,spec)
        (root/'report.json').write_text(json.dumps(report))
        for change in [lambda es:es.pop(1), lambda es:es[1].update(index=2), lambda es:es[-1].update(**{'pass':False})]:
            bad=copy.deepcopy(events);change(bad);(root/'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in bad))
            with self.assertRaises(r.life.Refused):r.artifacts(directory,group,spec)

    def test_png_bounds_crc_and_extra_files(self):
        directory,group,spec,_,_ = self.artifact_fixture(); root=directory/'artifacts'
        path=root/spec['pngs'][0]; original=path.read_bytes()
        path.write_bytes(original[:-1])
        with self.assertRaises(r.life.Refused):r.png_identity(path)
        path.write_bytes(original[:-1]+bytes([original[-1]^1]))
        with self.assertRaises(r.life.Refused):r.png_identity(path)
        path.write_bytes(original);(root/'extra.png').write_bytes(original)
        with self.assertRaises(r.life.Refused):r.artifacts(directory,group,spec)


    def test_fourth_numeric_failure_prevents_every_quality_child(self):
        row = self.binary()
        path = Path(row['path']); path.chmod(0o755)
        script = path.read_text()
        path.write_text(script.replace('import sys\n', 'import sys,os\nif os.getcwd().endswith("owner-cooked"): print("unsupported"); sys.exit(0)\n'))
        path.chmod(0o555); row['sha256']=r.life.sha(path);row['bytes']=path.stat().st_size
        result=self.execute_fixture(row)
        self.assertTrue(result['released']);self.assertFalse(result['accepted'])
        self.assertEqual([x['group'] for x in result['children']],[j[0] for j in r.NUMERIC_JOBS])
        self.assertFalse((self.e/'stationary').exists())

    def test_numeric_marker_and_exact_sequence_refusals(self):
        record=dict(exit=0,reaped=True,group_empty_after=True,timed_out=False,interrupted_by=[])
        stdout='running 1 test\nOWNER_HISTORY adapter=AdapterInfo { name: "CPU fixture", backend: Metal }\n'
        stdout+=''.join(f'OWNER_HISTORY case={c} center=[] probes=1 baseline={str(c in [7,8]).lower()} current_only={str(c==3).lower()}\n' for c in range(1,13))
        stdout+=r.MARKER+'\n'+GOOD.split('test CPU_only ... ok\n')[1]
        self.assertTrue(all(r.judge_numeric(stdout,'',record,r.OWNER)[0].values()))
        for bad in [stdout.replace(r.MARKER,''),stdout.replace('case=12','case=11'),stdout.replace('current_only=true','current_only=false'),stdout.replace('backend: Metal','backend: Vulkan')]:
            self.assertFalse(all(r.judge_numeric(bad,'',record,r.OWNER)[0].values()))

    def test_adapter_key_cross_kind(self):
        self.assertEqual(r.adapter_key('AdapterInfo { name: "Test Metal", backend: Metal }'),r.adapter_key(dict(name='Test Metal',backend='Metal')))
        with self.assertRaises(r.life.Refused):r.adapter_key('AdapterInfo { name: "Test Metal", backend: Vulkan }')


if __name__ == '__main__':
    unittest.main(verbosity=2)
