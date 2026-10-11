import contextlib
import io
import json
from pathlib import Path
import shlex
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import shipping_suite


class ShippingSuiteSelectionTests(unittest.TestCase):
    def test_workflow_installs_the_pinned_guest_target_before_sdk_preparation(self):
        workflow = (shipping_suite.ROOT / '.github/workflows/ci.yml').read_text()
        prerequisite = 'run: rustup target add --toolchain 1.99.0 wasm32-unknown-unknown'
        proof = 'run: python3 tools/shipping_suite.py '
        self.assertEqual(workflow.count(prerequisite), 1)
        self.assertLess(workflow.index(prerequisite), workflow.index(proof))

    def test_flipbook_refuses_without_four_proofs_before_creating_output(self):
        with tempfile.TemporaryDirectory() as storage:
            output = Path(storage) / 'proof'
            with patch.object(sys, 'argv', ['suite', str(output), '--flipbook']), \
                    contextlib.redirect_stderr(io.StringIO()), \
                    self.assertRaises(SystemExit) as raised:
                shipping_suite.main()
            self.assertEqual(raised.exception.code, 2)
            self.assertFalse(output.exists())

    def test_sdk_refuses_without_flipbook_before_creating_output(self):
        for extra in ([], ['--four-proofs']):
            with self.subTest(extra=extra), tempfile.TemporaryDirectory() as storage:
                output = Path(storage) / 'proof'
                with patch.object(sys, 'argv', ['suite', str(output), '--flipbook-sdk', *extra]), \
                        contextlib.redirect_stderr(io.StringIO()), \
                        self.assertRaises(SystemExit) as raised:
                    shipping_suite.main()
                self.assertEqual(raised.exception.code, 2)
                self.assertFalse(output.exists())

    def test_four_proofs_forward_exact_flipbook_selection_before_global_barrier(self):
        for flipbook, sdk, ci in ((False, False, False), (True, False, False),
                                  (True, True, False), (True, True, True)):
            with self.subTest(flipbook=flipbook, sdk=sdk, ci=ci), tempfile.TemporaryDirectory() as storage:
                output = Path(storage) / 'proof'
                events, arguments = [], {}
                session = Mock(path=Path(storage) / 'absent-build')
                session.report.return_value = {}
                session.seal.side_effect = lambda: events.append('sealed')

                def phase(name):
                    def run(args, selected_session):
                        self.assertIs(selected_session, session)
                        arguments[name] = args
                        events.append('prepare:' + name)
                        yield
                        self.assertIn('sealed', events)
                        events.append('execute:' + name)
                    return run

                probes = {name: SimpleNamespace(phases=phase(name)) for name in (
                    'native-scene-audio', 'native-game', 'cooked-script')}
                argv = ['suite', str(output), '--four-proofs'] + (['--flipbook'] if flipbook else []) + (['--flipbook-sdk'] if sdk else [])
                if ci:
                    # Exercise the actual hosted CLI selection through the parser
                    # and phase dispatch, not a second handwritten option list.
                    workflow = (shipping_suite.ROOT / '.github/workflows/ci.yml').read_text()
                    commands = [line.strip()[5:] for line in workflow.splitlines()
                                if line.strip().startswith('run: python3 tools/shipping_suite.py ')]
                    self.assertEqual(len(commands), 1)
                    argv = shlex.split(commands[0])[1:]
                    argv[1] = str(output)
                with patch.object(sys, 'argv', argv), \
                        patch.object(shipping_suite, 'FreshSession', return_value=session), \
                        patch.object(shipping_suite, 'run', side_effect=['/trusted/rustc', 'host: aarch64-apple-darwin']), \
                        patch.object(shipping_suite, 'load_probe', side_effect=probes.__getitem__), \
                        patch.object(shipping_suite.shipping, 'phases', side_effect=phase('runtime')), \
                        contextlib.redirect_stdout(io.StringIO()):
                    shipping_suite.main()
                self.assertEqual(arguments['native-game'].flipbook, flipbook)
                self.assertEqual(arguments['native-game'].flipbook_sdk, sdk)
                self.assertTrue(arguments['native-game'].gpu)
                self.assertTrue(arguments['native-game'].render_input)
                self.assertTrue(arguments['cooked-script'].physics)
                self.assertEqual(events[:5], ['prepare:' + name for name in (
                    'runtime', 'native-scene-audio', 'native-game', 'cooked-script')] + ['sealed'])
                report = json.loads((output / 'session.json').read_text())
                self.assertEqual(report['probes'][2], 'native-game.py --gpu' + (' --flipbook' if flipbook else '')
                                 + (' --flipbook-sdk' if sdk else ''))


if __name__ == '__main__':
    unittest.main()
