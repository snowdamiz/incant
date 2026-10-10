"""Credential-free tests for the ACP transport and worktree boundary."""
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('handoff', ROOT / 'tools/handoff/main.py')
handoff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(handoff)

class HandoffTests(unittest.TestCase):
    def test_packet_ids_reject_traversal(self):
        for value in ('../escape', '/root', 'a/b', 'a;b', '', 'a' * 65):
            with self.assertRaises(handoff.HandoffError):
                handoff.packet_id(value)
        self.assertEqual(handoff.packet_id('0001-hierarchy'), '0001-hierarchy')

    def test_paths_reject_symlink_escape(self):
        with tempfile.TemporaryDirectory() as root, tempfile.TemporaryDirectory() as outside:
            (Path(root) / 'escape').symlink_to(outside, target_is_directory=True)
            for path in ('relative', Path(root) / '../other', Path(root) / 'escape/secret'):
                with self.assertRaises(handoff.HandoffError):
                    handoff.within(root, path)
            self.assertEqual(handoff.within(root, Path(root) / 'safe.txt'), (Path(root) / 'safe.txt').resolve())

    def test_client_services_scoped_read_during_request(self):
        adapter = '''import json,sys
request=json.loads(input())
print(json.dumps({'jsonrpc':'2.0','id':99,'method':'fs/read_text_file','params':{'path':sys.argv[1]}}),flush=True)
reply=json.loads(input())
print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':reply['result']}),flush=True)
input()
'''
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'sample.txt'
            path.write_text('worktree data\n')
            client = handoff.AcpClient([sys.executable, '-u', '-c', adapter, str(path)], root, 2)
            try:
                self.assertEqual(client.request('test', {}), {'content': 'worktree data\n'})
            finally:
                client.close()

    def test_model_selection_refreshes_effort_and_pins_max_on_resume(self):
        model = {'id': 'model', 'category': 'model', 'currentValue': 'opus',
                 'options': [{'name': 'Opus 5.5', 'value': 'opus'}]}
        effort = {'id': 'effort', 'category': 'thought_level', 'currentValue': 'default',
                  'options': [{'name': 'Max', 'value': 'max'}]}
        client = Mock()
        client.request.side_effect = [
            {'configOptions': [model, effort]},
            {'configOptions': [model, {**effort, 'currentValue': 'max'}]},
        ]
        session = {'sessionId': 'test-session', 'configOptions': [model]}
        config = {'model': 'claude-opus-5-5', 'model_display_name': 'Opus 5.5', 'effort': 'max'}
        result = handoff.configure_model_and_effort(client, session, config)
        self.assertEqual(result['configOptions'][1]['currentValue'], 'max')
        self.assertEqual(client.request.call_args_list[1].args, ('session/set_config_option', {
            'sessionId': 'test-session', 'configId': 'effort', 'value': 'max'}))

    def test_unavailable_or_unconfirmed_max_prevents_prompt(self):
        model = {'id': 'model', 'category': 'model', 'currentValue': 'opus',
                 'options': [{'name': 'Opus 5.5', 'value': 'opus'}]}
        effort = {'id': 'effort', 'category': 'thought_level', 'currentValue': 'default',
                  'options': [{'name': 'Max', 'value': 'max'}]}
        config = {'model': 'claude-opus-5-5', 'model_display_name': 'Opus 5.5', 'effort': 'max'}
        session = {'sessionId': 'test-session', 'configOptions': [model, effort]}
        for responses in [
            [{'configOptions': [model]}],
            [{'configOptions': [model, {**effort, 'options': [{'value': 'high'}]}]}],
            [{'configOptions': [model, effort]}, {'configOptions': [model, effort]}],
            [{'configOptions': [{**model, 'currentValue': 'sonnet'}, effort]}],
        ]:
            with self.subTest(responses=responses):
                client = Mock()
                client.request.side_effect = responses
                with self.assertRaises(handoff.HandoffError):
                    handoff.configure_model_and_effort(client, session, config)
                self.assertTrue(all(call.args[0] == 'session/set_config_option'
                                    for call in client.request.call_args_list))

    def test_protocol_errors_do_not_echo_sensitive_body(self):
        adapter = '''import json
request=json.loads(input())
print(json.dumps({'jsonrpc':'2.0','id':request['id'],'error':{'code':-32000,'message':'secret-sentinel'}}),flush=True)
input()
'''
        with tempfile.TemporaryDirectory() as root:
            client = handoff.AcpClient([sys.executable, '-u', '-c', adapter], root, 2)
            try:
                with self.assertRaises(handoff.HandoffError) as error:
                    client.request('test', {})
                self.assertIn('-32000', str(error.exception))
                self.assertNotIn('secret-sentinel', str(error.exception))
            finally:
                client.close()

if __name__ == '__main__':
    unittest.main()
