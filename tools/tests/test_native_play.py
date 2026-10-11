import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('native_play', ROOT / 'tools/native_play.py')
play = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(play)


class CoordinatorTests(unittest.TestCase):
    def test_bounded_utf8_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'source'
            path.write_bytes(b'abc')
            self.assertEqual(play.bounded(path, 3), b'abc')
            with self.assertRaises(ValueError):
                play.bounded(path, 2)
            path.write_bytes(b'\xff')
            with self.assertRaises(UnicodeError):
                play.bounded(path, 3)

    def test_concurrent_build_copies_are_serialized_and_exception_releases_lock(self):
        script = '''import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from native_play import build_lock
print('waiting', flush=True)
with build_lock(Path(sys.argv[2])):
    print('acquired', flush=True)
'''
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'build.lock'
            child = None
            try:
                with self.assertRaisesRegex(RuntimeError, 'cancelled build'):
                    with play.build_lock(path):
                        child = subprocess.Popen(
                            [sys.executable, '-c', script, str(ROOT / 'tools'), str(path)],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                        )
                        self.assertEqual(child.stdout.readline().strip(), 'waiting')
                        with self.assertRaises(subprocess.TimeoutExpired):
                            child.wait(timeout=0.25)
                        raise RuntimeError('cancelled build')
                stdout, stderr = child.communicate(timeout=5)
                self.assertEqual(child.returncode, 0, stderr)
                self.assertEqual(stdout.strip(), 'acquired')
            finally:
                if child is not None and child.poll() is None:
                    child.kill()
                    child.communicate()


if __name__ == '__main__':
    unittest.main()
