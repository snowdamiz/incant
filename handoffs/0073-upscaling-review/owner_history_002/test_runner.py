"""CPU-only guards and actual tiny process lifecycle; never invokes renderer."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
P = Path(__file__).resolve().with_name("runner.py")
s = importlib.util.spec_from_file_location("history_runner", P)
r = importlib.util.module_from_spec(s)
s.loader.exec_module(r)


def output(backend="Metal", retained=False):
    if retained:
        return "\n".join(["running 1 test", "test " + r.RETAINED_TEST + " ... RETAINED_OWNER adapter=AdapterInfo { name: \"CPU fixture only\", backend: " + backend + " }", r.RETAINED_MARKER, "ok", "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out"]) + "\n"
    lines = ["running 1 test", "test " + r.TEST + " ... OWNER_HISTORY adapter=AdapterInfo { name: \"CPU fixture only\", backend: " + backend + " }"]
    for i in range(1, 13):
        lines.append(f"OWNER_HISTORY case={i} center=([0.0;4], 1.0, [1,0]) probes=1 baseline={str(i in (7,8)).lower()} current_only={str(i == 3).lower()}")
    lines.extend([r.MARKER, "ok", "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out"])
    return "\n".join(lines) + "\n"


RECORD = dict(exit=0, reaped=True, group_empty_after=True, timed_out=False, interrupted_by=[])


class Controls(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="owner-history-CPU-")
        self.root = Path(self.tmp.name)
        self.e = self.root / "evidence"
        self.e.mkdir()
        self.addCleanup(self.tmp.cleanup)

    def binary(self, name, text=None, body=None):
        p = self.root / name
        if body is None:
            body = ("import sys\nprint(" + repr(text) + ", end='')" if text is not None else
                    "import sys\nprint(" + repr(output(retained=True)) + " if " + repr(r.RETAINED_TEST) + " in sys.argv else " + repr(output()) + ", end='')")
        p.write_text("#!" + sys.executable + "\n" + body + "\n")
        p.chmod(0o555)
        return dict(kind=name, path=str(p), sha256=r.life.sha(p), bytes=p.stat().st_size, mode="0555")

    def plan(self, binaries):
        return dict(schema="incant-owner-history-combined-plan-v2", state="FROZEN_READY_FOR_ROOT_GRANT",
                    producer="a"*40, evidence_dir=str(self.e), budget=r.BUDGET, environment=r.ENV,
                    binaries=binaries, identities={})

    def execute_fixture(self, binaries):
        plan = self.plan(binaries)
        pp = self.root / "plan.json"
        pp.write_text(json.dumps(plan))
        (self.e / "grant.json").write_text('{"CPU_fixture_only":true}')
        with patch.object(r, "E", self.e), patch.object(r, "GLOBAL_LOCK", self.root / "lock"), \
             patch.object(r, "read_plan", return_value=plan), patch.object(r, "grant", return_value=({}, {})):
            return r.execute(pp)

    def test_exact_nocapture_output(self):
        self.assertTrue(r.judge(output(), "", RECORD, r.TEST)[0])

    def test_output_mutations_refused(self):
        cases = [output().replace(r.MARKER, ""), output().replace(r.MARKER, r.MARKER+"\n"+r.MARKER),
                 output("Vulkan"), output().replace("case=7", "case=8"),
                 output().replace("baseline=true", "baseline=false", 1),
                 output().replace("1 passed", "2 passed"), output().replace("case=12", "case=13")]
        for text in cases:
            self.assertFalse(r.judge(text, "", RECORD, r.TEST)[0])
        self.assertFalse(r.judge(output(), "panicked at fixture", RECORD, r.TEST)[0])
        self.assertFalse(r.judge(output(), "", {**RECORD, "exit": 101}, r.TEST)[0])

    def test_no_grant_touches_nothing(self):
        with patch.object(r, "E", self.e):
            with self.assertRaises(r.life.Refused):
                r.execute(self.root / "missing-plan")
        self.assertEqual(list(self.e.iterdir()), [])

    def test_plan_semantic_mutations(self):
        ids={name:dict(path=str(self.root / name),sha256="a"*64) for name in r.ROLES}
        for name,row in ids.items():
            Path(row['path']).write_text(r.TEST+": test\n"+r.RETAINED_TEST+": test\n2 tests, 0 benchmarks\n")
            row['sha256']=r.life.sha(row['path'])
        # CPU fixture supplies admitted hashes only for source identity tests;
        # real run uses hardcoded exact shader/test hashes and binary checks.
        b=[self.binary(x) for x in r.KINDS]
        b[1]['sha256']='b'*64
        data=self.plan(b);data['identities']=ids
        path=self.root/'plan.json'
        def run(data):
            path.write_text(json.dumps(data))
            with patch.object(r,'E',self.e),patch.object(r,'SOURCE_HASHES',{k:ids[k]['sha256'] for k in r.SOURCE_HASHES}),patch.object(r.life,'check_binary'):
                return r.read_plan(path)
        self.assertEqual(run(data)['producer'],'a'*40)
        for mutate in [lambda p:p['budget'].update(queue_submissions=25),
                       lambda p:p['environment'].update(WGPU_BACKEND='vulkan'),
                       lambda p:p.update(state='PREPARATION_ONLY'),
                       lambda p:p['binaries'].reverse(),
                       lambda p:p['binaries'][0].update(mode='0644'),
                       lambda p:p.update(producer='<pending>'),
                       lambda p:p['identities'].pop('cpu_review')]:
            bad=copy.deepcopy(data);mutate(bad)
            with self.assertRaises(r.life.Refused):run(bad)

    def test_grant_mutations(self):
        plan=self.plan([self.binary(x) for x in r.KINDS]);path=self.root/'plan.json';path.write_text(json.dumps(plan))
        prior=self.root/'release.json';prior.write_text(json.dumps(dict(released=True,owned_processes=[])))
        g=dict(schema='incant-owner-history-combined-grant-v2',authority='root',holder='Claude0073 Opus5.5 Max',
               issued_at='2026-10-13',plan_sha256=r.life.sha(path),runner_sha256=r.life.sha(r.HERE),
               lifecycle_sha256=r.LIFECYCLE_SHA,binary_sha256=[x['sha256'] for x in plan['binaries']],
               evidence_dir=str(self.e),predecessor_release_path=str(prior),predecessor_release_sha256=r.life.sha(prior),budget=r.BUDGET)
        def check(g):
            (self.e/'grant.json').write_text(json.dumps(g))
            with patch.object(r,'E',self.e):return r.grant(path,plan)
        self.assertTrue(check(g))
        for field,value in [('authority','claude'),('holder','other'),('runner_sha256','a'*64),
                             ('lifecycle_sha256','b'*64),('plan_sha256','c'*64),('binary_sha256',[]),
                             ('budget',{}),('predecessor_release_sha256','a'*64),('evidence_dir','/tmp/other')]:
            bad=copy.deepcopy(g);bad[field]=value
            with self.assertRaises(r.life.Refused):check(bad)

    def test_four_children_clean_release(self):
        binaries = [self.binary(x) for x in r.KINDS]
        result=self.execute_fixture(binaries)
        self.assertTrue(result['accepted']);self.assertTrue(result['released'])
        self.assertEqual(len(result['children']),4);self.assertEqual(result['owned_processes'],[])
        self.assertEqual(len((self.e/'debits.jsonl').read_text().splitlines()),4)
        debits = [json.loads(x) for x in (self.e/'debits.jsonl').read_text().splitlines()]
        self.assertEqual(sum(x['queue_submissions'] for x in debits), 72)
        self.assertEqual(sum(x['readback_maps'] for x in debits), 48)
        self.assertEqual([x['job'] for x in result['children']], [x[0] for x in r.JOBS])
        for row in result['children']:
            with self.assertRaises(ProcessLookupError):os.kill(row['pid'],0)
        with self.assertRaises(r.life.Refused):self.execute_fixture(binaries)

    def test_failed_first_stops_without_second_debit(self):
        result=self.execute_fixture([self.binary('wgsl',text='early unsupported\n'),self.binary('cooked-metal')])
        self.assertFalse(result['accepted']);self.assertTrue(result['released'])
        self.assertEqual(len(result['children']),1)
        self.assertEqual(len((self.e/'debits.jsonl').read_text().splitlines()),1)

    def test_adapter_difference_fails_second(self):
        result=self.execute_fixture([self.binary('wgsl'),self.binary('cooked-metal',text=output().replace('CPU fixture only','Other CPU fixture'))])
        self.assertFalse(result['accepted']);self.assertTrue(result['released'])
        self.assertTrue(result['children'][1]['adapter_mismatch'])

    def test_timeout_actual_child_cleanup(self):
        with patch.object(r,'TIMEOUT',0.15):
            result=self.execute_fixture([self.binary('wgsl',body='import time; time.sleep(20)'),self.binary('cooked-metal')])
        self.assertTrue(result['released']);self.assertFalse(result['accepted'])
        self.assertTrue(result['children'][0]['timed_out'])

    def test_base_exception_after_launch_records_and_reaps(self):
        class Abort(BaseException):pass
        count=0
        real=r.life.signal.sigpending
        def pending():
            nonlocal count
            count+=1
            if count==2:raise Abort('CPU control')
            return real()
        with patch.object(r.life.signal,'sigpending',pending):
            with self.assertRaises(Abort):
                self.execute_fixture([self.binary('wgsl',body='import time; time.sleep(20)'),self.binary('cooked-metal')])
        release=json.loads((self.e/'release.json').read_text())
        self.assertTrue(release['released']);self.assertIn('Abort',release['exception'])
        self.assertEqual(len(release['children']),1)

    def test_interrupt_actual_child_cleanup(self):
        signal_file=self.root/'sender.py'
        signal_file.write_text('import time,os,signal\ntime.sleep(.3)\nos.kill('+str(os.getpid())+',signal.SIGTERM)\n')
        previous=signal.signal(signal.SIGTERM,r.life.raise_interrupt)
        sender=subprocess.Popen([sys.executable,str(signal_file)])
        try:
            with self.assertRaises(KeyboardInterrupt):
                self.execute_fixture([self.binary('wgsl',body='import time; time.sleep(20)'),self.binary('cooked-metal')])
        finally:
            sender.wait();signal.signal(signal.SIGTERM,previous)
        release=json.loads((self.e/'release.json').read_text())
        self.assertTrue(release['released']);self.assertFalse(release['accepted'])
        self.assertEqual(release['children'][0]['interrupted_by'],['SIGTERM'])


    def test_retained_marker_is_required(self):
        good = output(retained=True)
        self.assertTrue(r.judge(good, "", RECORD, r.RETAINED_TEST)[0])
        for bad in [good.replace(r.RETAINED_MARKER, ""), good.replace("frames=12", "frames=11"),
                    good.replace(r.RETAINED_MARKER, r.RETAINED_MARKER+"\n"+r.RETAINED_MARKER),
                    good.replace("backend: Metal", "backend: Vulkan")]:
            self.assertFalse(r.judge(bad, "", RECORD, r.RETAINED_TEST)[0])

    def test_current_only_case_must_be_three(self):
        self.assertFalse(r.judge(output().replace("current_only=true", "current_only=false"), "", RECORD, r.TEST)[0])
        self.assertFalse(r.judge(output().replace("case=4 center", "case=3 center"), "", RECORD, r.TEST)[0])

    def test_failed_second_prevents_all_retained_children(self):
        result = self.execute_fixture([self.binary('wgsl'), self.binary('cooked-metal', text='unsupported\n')])
        self.assertFalse(result['accepted']); self.assertTrue(result['released'])
        self.assertEqual([x['job'] for x in result['children']], [x[0] for x in r.JOBS[:2]])
        self.assertEqual(len((self.e/'debits.jsonl').read_text().splitlines()), 2)

    def test_failed_third_prevents_fourth(self):
        body = "import sys\nprint('early unsupported' if " + repr(r.RETAINED_TEST) + " in sys.argv else " + repr(output()) + ")"
        result = self.execute_fixture([self.binary('wgsl', body=body), self.binary('cooked-metal')])
        self.assertFalse(result['accepted']); self.assertTrue(result['released'])
        self.assertEqual([x['job'] for x in result['children']], [x[0] for x in r.JOBS[:3]])
        self.assertEqual(len((self.e/'debits.jsonl').read_text().splitlines()), 3)

    def test_retained_adapter_difference_stops_third(self):
        body = "import sys\nprint(" + repr(output(retained=True).replace('CPU fixture only','Other CPU fixture')) + " if " + repr(r.RETAINED_TEST) + " in sys.argv else " + repr(output()) + ")"
        result = self.execute_fixture([self.binary('wgsl', body=body), self.binary('cooked-metal')])
        self.assertFalse(result['accepted']); self.assertTrue(result['released'])
        self.assertEqual(len(result['children']), 3)
        self.assertTrue(result['children'][-1]['adapter_mismatch'])



    def test_live_global_lock_refuses_before_debit(self):
        plan = self.plan([self.binary(x) for x in r.KINDS])
        pp = self.root/'plan.json'; pp.write_text(json.dumps(plan))
        (self.e/'grant.json').write_text('{"CPU_fixture_only":true}')
        lockpath = self.root/'global.lock'
        with open(lockpath, 'a') as held:
            r.fcntl.flock(held, r.fcntl.LOCK_EX | r.fcntl.LOCK_NB)
            with patch.object(r,'E',self.e), patch.object(r,'GLOBAL_LOCK',lockpath), \
                 patch.object(r,'read_plan',return_value=plan), patch.object(r,'grant',return_value=({},{})):
                with self.assertRaises(r.life.Refused): r.execute(pp)
        self.assertFalse((self.e/'debits.jsonl').exists())
        self.assertFalse((self.e/'release.json').exists())

    def test_actual_binary_hash_and_mode_refusals(self):
        row = self.binary('wgsl')
        expected = {k:v for k,v in row.items() if k != 'kind'}
        r.life.check_binary(expected)
        Path(row['path']).chmod(0o644)
        with self.assertRaises(r.life.Refused): r.life.check_binary(expected)
        Path(row['path']).chmod(0o555)
        with self.assertRaises(r.life.Refused): r.life.check_binary({**expected, 'sha256':'a'*64})


if __name__ == '__main__':
    unittest.main(verbosity=2)
