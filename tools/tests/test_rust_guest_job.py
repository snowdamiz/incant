"""Pure fixed host-mailbox and snapshot controls. No OS child or sandbox execution."""
import json
from pathlib import Path
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import rust_guest_job as job
import rust_guest_build as service
from rust_guest_sandbox import Refused

class Jobs(unittest.TestCase):
    def test_diagnostics_cap_is_bytes_and_preserves_valid_utf8(self):
        for text in ('x', 'x'*65536, 'x'*65535+'€', '😀'*65536):
            result=job.bounded_diagnostics(text)
            self.assertLessEqual(len(result.encode()),65536)
            self.assertTrue(text.startswith(result))
    def test_bounded_existing_package_is_read_only_and_has_no_tool_authority(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve();package=root/'logic';package.mkdir()
            (package/'module.rs').write_text('mod helper;')
            (package/'helper.rs').write_text('pub const X:u32=3;')
            before={str(p):p.read_bytes() for p in package.iterdir()}
            view=job.package_view(root,'logic','module.rs')
            self.assertEqual({f['path'] for f in view['files']},{'module.rs','helper.rs'})
            self.assertEqual(before,{str(p):p.read_bytes() for p in package.iterdir()})
            for name in ('../logic','/logic','.incant','logic/..'):
                with self.subTest(name=name),self.assertRaises(Refused):job.package_view(root,name,'module.rs')
            (package/'Cargo.toml').write_text('forbidden')
            with self.assertRaises(Refused):job.package_view(root,'logic','module.rs')

    def proxy(self,root):
        (root/'calls').mkdir();work=root/'work';work.mkdir()
        return job.ProxyOwner(work,service.Limits(seconds=5,child_seconds=3),lambda:False,root)

    def reply(self,root,row,output=b'ok'):
        def respond():
            request=root/'calls/000/request.json';deadline=time.monotonic()+2
            while not request.exists():
                if time.monotonic()>deadline:return
                time.sleep(.001)
            self.observed=json.loads(request.read_text())
            (request.parent/'stdout').write_bytes(output);(request.parent/'stderr').write_bytes(b'')
            job.private_write(request.parent/'response.json',row)
        thread=threading.Thread(target=respond);thread.start();return thread

    def test_proxy_cannot_spawn_and_requires_rust_terminal_group_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve();owner=self.proxy(root)
            thread=self.reply(root,{'pid':123,'returncode':0,'group_absent':True})
            with mock.patch('subprocess.Popen',side_effect=AssertionError('must delegate')):
                result=owner.run(['/trusted/rustc','-vV'],cwd=root,env={'LANG':'C'},label='compiler')
            thread.join();self.assertEqual(result.stdout,b'ok')
            self.assertEqual(self.observed['argv'],['/trusted/rustc','-vV'])
            self.assertEqual(owner.calls[0]['returncode'],0)
            self.assertTrue(owner.calls[0]['group_absent'])

    def test_unknown_or_unclosed_response_is_not_success(self):
        for row in ({'pid':1,'returncode':0,'group_absent':False},{'pid':1,'returncode':0,'group_absent':True,'ignored':True}):
            with self.subTest(row=row),tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary).resolve();owner=self.proxy(root);thread=self.reply(root,row)
                with self.assertRaises(Refused):owner.run(['/trusted/rustc','-vV'],cwd=root,env={},label='compiler')
                thread.join()

    def test_cancel_before_launch_and_stream_cap(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve();owner=self.proxy(root);owner.cancel=lambda:True
            with self.assertRaisesRegex(Refused,'cancelled'):owner.run(['/trusted/rustc'],cwd=root,env={},label='compiler')
            self.assertFalse((root/'calls/000').exists())
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve();owner=self.proxy(root);owner.limits=service.Limits(seconds=5,stream_bytes=3)
            thread=self.reply(root,{'pid':123,'returncode':0,'group_absent':True},b'too large')
            with self.assertRaises(Refused):owner.run(['/trusted/rustc'],cwd=root,env={},label='compiler')
            thread.join()

    def test_atomic_reply_and_create_only_input_names(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);path=root/'reply.json';job.private_write(path,{'status':'complete'})
            self.assertEqual(json.loads(path.read_text()),{'status':'complete'})
            self.assertFalse(path.with_name('reply.json.tmp').exists())
            self.assertFalse(list(root.glob('*.wasm')))

if __name__=='__main__':unittest.main()
