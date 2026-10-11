"""Offline refusal tests for the local fixture boundary; live probes are separate."""
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import local_services as local


class LocalState(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.addCleanup(patch.stopall)
        patch.object(local, 'STATE_HOME', self.root / 'private-services').start()
        patch.object(local.Stack, 'check_runtime').start()

    def stack(self):
        s = local.Stack('test', 1000)
        s.prepare()
        return s

    def test_generated_values_are_private_idempotent_and_distinct(self):
        a = self.stack()
        b = local.Stack('other', 2000); b.prepare()
        before = {f: f.read_bytes() for f in a.state.rglob('*') if f.is_file()}
        a.prepare()
        self.assertEqual(before, {f: f.read_bytes() for f in a.state.rglob('*') if f.is_file()})
        self.assertNotEqual(a.values['DB_PASSWORD'], b.values['DB_PASSWORD'])
        self.assertNotEqual(a.values['SECRET_KEY'], b.values['SECRET_KEY'])
        if os.name != 'nt':
            for f in before:
                self.assertEqual(f.stat().st_mode & 0o077, 0)
        self.assertEqual(a.values['PG_PORT'], '16432')

    def test_partial_initialization_is_rolled_back_and_can_retry(self):
        s = local.Stack('test', 1000)
        with patch.object(local, 'private_write', side_effect=OSError('synthetic write failure')):
            with self.assertRaises(OSError): s.prepare()
        self.assertFalse(s.state.exists())
        s.prepare()
        self.assertTrue((s.state/'settings.json').is_file())

    def test_incomplete_existing_state_refuses_without_overwrite(self):
        s = self.stack(); (s.state/'.env').unlink()
        preserved = (s.state/'settings.json').read_bytes()
        with self.assertRaisesRegex(ValueError, 'incomplete'): s.prepare()
        self.assertEqual(preserved, (s.state/'settings.json').read_bytes())

    def test_mismatched_offsets_and_modified_environment_refuse(self):
        s = self.stack()
        with self.assertRaisesRegex(ValueError, 'port offset'): local.Stack('test', 1001).prepare()
        (s.state/'.env').write_text('INCANT_LOCAL_STATE=/tmp/elsewhere\n')
        with self.assertRaisesRegex(ValueError, 'differ'): s.prepare()

    def test_moved_state_and_unexpected_fields_refuse(self):
        s = self.stack(); p = s.state/'settings.json'; data = json.loads(p.read_text())
        data['values']['STATE'] = '/tmp/elsewhere'; p.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'paths'): s.prepare()
        data['values']['STATE'] = str(s.state); data['values']['UNEXPECTED'] = 'value'; p.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'fields'): s.prepare()

    def test_newline_or_interpolation_in_generated_values_refuses(self):
        for value in ['bad\nVALUE=1', '${HOME}']:
            s = self.stack(); p = s.state/'settings.json'; data = json.loads(p.read_text()); original=p.read_bytes()
            data['values']['DB_PASSWORD'] = value; p.write_text(json.dumps(data))
            with self.assertRaises(ValueError): s.prepare()
            p.write_bytes(original)

    @unittest.skipIf(os.name == 'nt', 'POSIX mode/symlink refusal')
    def test_symlink_parents_and_private_file_refuse(self):
        (self.root/'private-services').symlink_to(self.root, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'parents'): local.Stack('test').prepare()
        (self.root/'private-services').unlink()
        s = self.stack(); target = self.root/'outside'; target.write_text('unchanged')
        (s.state/'.env').unlink(); (s.state/'.env').symlink_to(target)
        with self.assertRaisesRegex(ValueError, 'symlink'): s.prepare()
        self.assertEqual(target.read_text(), 'unchanged')

    @unittest.skipIf(os.name == 'nt', 'POSIX permissions')
    def test_world_readable_state_refuses(self):
        s = self.stack(); (s.state/'settings.json').chmod(0o644)
        with self.assertRaisesRegex(ValueError, 'private'): s.prepare()

    def test_invalid_names_and_ports_never_create_state(self):
        for name, offset in [('../other',0),('Upper',0),('ok',-1),('ok',40001)]:
            with self.assertRaises(ValueError): local.Stack(name,offset)
        self.assertFalse((self.root/'private-services').exists())

    def test_generated_secrets_are_redacted_from_failure_text(self):
        s = self.stack()
        for key in ['DB_PASSWORD','OIDC_DB_PASSWORD','ADMIN_PASSWORD','ACCESS_KEY','SECRET_KEY']:
            self.assertNotIn(s.values[key], s.redact('failed '+s.values[key]))


class LocalBoundary(unittest.TestCase):
    def config(self):
        lock = local.image_policy()
        return {'networks': {'local': {'internal': True}, 'edge': {}}, 'services': {
            name: {'image': item['image'], 'networks': {'local': {}, **({'edge': {}} if name=='gateway' else {})},
                   'ports': [{'host_ip':'127.0.0.1'}] if name=='gateway' else []}
            for name,item in lock['images'].items()}}

    def check(self, config):
        with patch.object(local.Stack, 'compose', return_value=json.dumps(config)):
            return local.Stack('test').validate_config()

    def test_manifest_retains_five_digest_pins_and_license_texts(self):
        self.assertEqual(len(local.image_policy()['images']),5)
        self.check(self.config())

    def test_public_ports_backend_egress_and_host_privileges_refuse(self):
        changes = [lambda c:c['services']['gateway']['ports'][0].update(host_ip='0.0.0.0'),
            lambda c:c['services']['oidc']['networks'].update(edge={}),
            lambda c:c['services']['postgres'].update(privileged=True),
            lambda c:c['services']['mail'].update(ports=[{'host_ip':'127.0.0.1'}]),
            lambda c:c['services']['objects'].update(volumes=[{'source':'/var/run/docker.sock'}]),
            lambda c:c['networks']['local'].update(internal=False),
            lambda c:c['services']['gateway'].update(cap_add=['NET_ADMIN']),
            lambda c:c['services']['gateway'].update(image='haproxy:latest')]
        for change in changes:
            config = self.config(); change(config)
            with self.assertRaises(ValueError): self.check(config)

    def test_context_override_cannot_hide_remote_endpoint_behind_local_host(self):
        with patch.dict(os.environ, {'DOCKER_CONTEXT':'remote','DOCKER_HOST':'unix:///local'}):
            stack = local.Stack('test')
            with patch.object(stack, 'command', return_value='"tcp://remote:2375"') as command:
                with self.assertRaisesRegex(ValueError, 'remote Docker'): stack.check_runtime()
                self.assertIn('remote', command.call_args.args[0])

    def test_remote_docker_and_ambient_service_override_refuse(self):
        with patch.dict(os.environ, {'DOCKER_HOST':'tcp://remote:2375','INCANT_LOCAL_STATE':'/outside'}):
            stack=local.Stack('test')
            self.assertNotIn('INCANT_LOCAL_STATE', stack.child_env)
            with self.assertRaisesRegex(ValueError, 'remote Docker'): stack.check_runtime()


if __name__ == '__main__':
    unittest.main()
