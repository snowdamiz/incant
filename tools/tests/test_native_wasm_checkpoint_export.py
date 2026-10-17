"""Synthetic adversarial data controls; these never count as native proof."""
import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from probes.native_wasm_checkpoint import fixture, oracle
from probes.native_wasm_checkpoint.owner import CheckpointOwner
from probes.native_wasm_checkpoint.replay import authoring, managed_logs


def independent_state(tick):
    # Closed-form intervals, independent of the event-walking production oracle.
    return tick, sum(min(5, max(0, tick - offset)) for offset in (1, 15, 28)), sum(tick >= n for n in (4, 17, 31))


def saved(tick):
    world, audio = b'synthetic-world', b'synthetic-audio'
    state = b'INCWST01' + struct.pack('<II', 1, 24) + bytes([23]) * 32 + bytes(8)
    state += struct.pack('<Qq3Q', tick, tick, *independent_state(tick))
    data = bytearray(b'INCSES01' + struct.pack('<II', 7, 1) + b'p' * 32 + b'w' * 32
                     + struct.pack('<4I', len(world), len(state), 0, len(audio)) + bytes(32) + world + state + audio)
    data[96:128] = hashlib.sha256(data[:96] + data[128:]).digest()
    return bytes(data)


def sample_outputs():
    pcm = b''.join(struct.pack('<f', 0 if 26 <= tick <= 28 else tick / 100) * 1600 for tick in range(1, 41))
    output = {}
    for name, (start, end) in oracle.RUNS.items():
        part = pcm[start * 6400:end * 6400]
        def xyz(tick):
            count, moves, score = independent_state(tick)
            return [moves / 16, score, count / 64]
        checks = [{'name': f'persistent-{tick}', 'tick': tick, 'passed': True, 'actual': xyz(tick),
                   'expect': {'type': 'vector', 'entity': fixture.ENTITY, 'property': 'translation',
                              'value': xyz(tick), 'tolerance': 0}} for tick in range(start, end + 1)]
        report = {'tick': end, 'frames': (end - start) * 800, 'pcm_sha256': oracle.sha(part), 'flipbooks': [],
                  'logs': {'dropped': 0, 'entries': [{'tick': tick, 'level': 'info',
                    'message': 'fresh-guest' if tick == start + 1 else 'continued-guest'} for tick in range(start + 1, end + 1)]},
                  'observations': {'passed': True, 'tick': end, 'tick_rate': 60, 'checks': checks,
                    'entities': {fixture.ENTITY: {'translation': xyz(end), 'rotation': [0, 0, 0, 1],
                        'scale': [1, 1, 1], 'parent': None, 'velocity': None,
                        'world_transform': [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [*xyz(end), 1]]}}}}
        output[name] = {'report': report, 'pcm': part, 'saved': saved(end)}
    return output


class CheckpointOracleTests(unittest.TestCase):
    def test_complete_independent_data_fixture(self):
        oracle.joined(sample_outputs())

    def test_input_interval_oracle_and_rebased_suffix(self):
        for tick in range(41):
            self.assertEqual(fixture.state(tick), independent_state(tick))
        self.assertEqual(fixture.position(13), [.3125, 1, .203125])
        self.assertEqual(fixture.position(40), [.9375, 3, .625])
        suffix = fixture.recording(13, 27)
        self.assertEqual([r['tick'] for r in suffix['frames']], [3, 4, 5, 8, 16, 18, 19, 21])
        self.assertEqual(fixture.recording(0, 13)['frames'][-1]['tick'], 7)

    def test_coherent_state_schema_clock_and_program_mutations(self):
        value = saved(13)
        for kind in oracle.REFUSALS:
            mutant = oracle.mutate_checkpoint(value, kind)
            self.assertNotEqual(value, mutant)
            if kind != 'corrupt':
                self.assertEqual(mutant[96:128], hashlib.sha256(mutant[:96] + mutant[128:]).digest())
            if kind != 'program':  # Program admission is trusted-engine authority, not this structural parser.
                with self.assertRaises(ValueError):
                    oracle.checkpoint(mutant, 13)
        changed = bytearray(saved(40)); changed[-1] ^= 1
        with self.assertRaises(ValueError):
            oracle.checkpoint(changed, 40)

    def test_refuses_repaired_gameplay_bytes_and_length(self):
        for offset in (72, 80, 88):
            content = bytearray(saved(40)); start = 128 + struct.unpack_from('<I', content, 80)[0]
            content[start + offset] ^= 1
            content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
            with self.assertRaises(ValueError):
                oracle.checkpoint(content, 40)
        content = bytearray(saved(40)); struct.pack_into('<I', content, 84, 95)
        content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
        with self.assertRaises(ValueError):
            oracle.checkpoint(content, 40)

    def test_coherent_pcm_digest_cannot_mask_join_difference(self):
        value = sample_outputs()
        value['resumed']['pcm'] = struct.pack('<f', .75) + value['resumed']['pcm'][4:]
        value['resumed']['report']['pcm_sha256'] = oracle.sha(value['resumed']['pcm'])
        with self.assertRaisesRegex(ValueError, 'joined PCM'):
            oracle.joined(value)

    def test_common_false_world_equality_is_not_an_oracle(self):
        value = sample_outputs()
        for row in value.values():
            for check in row['report']['observations']['checks']:
                check['actual'][0] = 0
                check['expect']['value'][0] = 0
        with self.assertRaises(ValueError):
            oracle.joined(value)

    def test_missing_checks_false_success_and_boolean_ticks(self):
        for mutate in (lambda r: r['observations']['checks'].pop(),
                       lambda r: r['observations'].__setitem__('passed', 1),
                       lambda r: r.__setitem__('tick', True),
                       lambda r: r['observations']['checks'][1].__setitem__('actual', [float('nan'), 0, 0])):
            value = sample_outputs(); mutate(value['baseline']['report'])
            with self.assertRaises((ValueError, KeyError)):
                oracle.joined(value)

    def test_fresh_vm_marker_is_required_after_load(self):
        value = sample_outputs()
        value['resumed']['report']['logs']['entries'][0]['message'] = 'continued-guest'
        with self.assertRaisesRegex(ValueError, 'Fresh VM'):
            oracle.joined(value)

    def test_wrong_refusal_or_publication_cannot_pass(self):
        receipt = {'returncode': 1, 'stdout': '', 'stderr': oracle.REFUSALS['schema'],
                   'outputs_present': [False, False], 'staging': []}
        oracle.refusal(receipt, 'schema')
        for key, bad in [('returncode', 0), ('stdout', '{}'), ('stderr', oracle.REFUSALS['corrupt']),
                         ('outputs_present', [False, True]), ('staging', ['.incant-pcm-leftover'])]:
            with self.assertRaises(ValueError):
                oracle.refusal(receipt | {key: bad}, 'schema')

    def test_separate_publication_and_sentinel_identity(self):
        record = {'sha256': 'a' * 64, 'bytes': 17, 'mode': 0o600}
        receipt = {'returncode': 1, 'stdout': '',
                   'stderr': 'Error: Os { code: 17, kind: AlreadyExists, message: "File exists" }',
                   'sentinel_before': record, 'sentinel_after': record,
                   'outputs_present': [True, True], 'staging': []}
        oracle.refusal(receipt, 'save-exists')
        oracle.refusal(receipt | {'outputs_present': [True, False]}, 'pcm-exists')
        with self.assertRaises(ValueError):
            oracle.refusal(receipt | {'sentinel_after': record | {'mode': 0o644}}, 'save-exists')
        with self.assertRaises(ValueError):
            oracle.refusal(receipt | {'outputs_present': [False, True]}, 'save-exists')


class ProcessDataTests(unittest.TestCase):
    def test_exact_nonempty_inventory_includes_wrapper_and_descendants(self):
        inventory = [{'pid': 10, 'pgid': 9, 'source': 'wrapper'},
                     {'pid': 11, 'pgid': 11, 'source': 'managed-leader'},
                     {'pid': 12, 'pgid': 11, 'source': 'sampled-descendant'}]
        owner = {'owned': [{'pid': 11, 'cleanup_confirmed': True}]}
        absence = [{'pid': n, 'pid_and_group_absent': True} for n in (10, 11, 12)]
        oracle.cleanup(inventory, owner, absence, 10)
        for bad in ([], absence[:-1], [absence[0]] * 3,
                    absence[:-1] + [{'pid': 12, 'pid_and_group_absent': 1}]):
            with self.assertRaises(ValueError):
                oracle.cleanup(inventory, owner, bad, 10)
        with self.assertRaises(ValueError):
            oracle.cleanup([], owner, [], 10)

    def test_discovered_inventory_is_append_only_not_final_claim(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary); target = path / 'target'; target.mkdir()
            owner = CheckpointOwner(path / 'output', target)
            owner.discover(100001, 100001, 'managed-leader')
            owner.discover(100002, 100001, 'sampled-descendant')
            owner.discover(100002, 100001, 'sampled-descendant')
            rows = [json.loads(line) for line in (owner.output / 'process-inventory.jsonl').read_text().splitlines()]
            self.assertEqual(len(rows), 3)
            self.assertEqual(rows[-1], {'pid': 100002, 'pgid': 100001, 'source': 'sampled-descendant'})
            with mock.patch('probes.native_wasm_checkpoint.owner.process_rows', return_value=[]):
                self.assertEqual(owner.absent_children(), [{'pid': n, 'pid_and_group_absent': True} for n in (100001, 100002)])
            self.assertEqual(len((owner.output / 'process-inventory.jsonl').read_text().splitlines()), 3)
            with self.assertRaises(ValueError):
                owner.discover(100002, 99999, 'sampled-descendant')


class ManagedStreamDataTests(unittest.TestCase):
    def fixture(self, root):
        dwarf = '/private/build/symbols/incant_game_player.dSYM/Contents/Resources/DWARF/incant_game_player-abc'
        symbols = '/private/build/symbols/incant_game_player.dSYM'
        executable, player = '/private/target/incant_game_player', '/private/build/player/incant_game_player'
        commands = [['cargo', 'build'], ['/trusted/llvm-readobj', '--sections', dwarf],
                    ['xcrun', 'dwarfdump', '--verify', symbols], ['xcrun', 'dwarfdump', '--uuid', executable],
                    ['xcrun', 'dwarfdump', '--uuid', symbols], ['xcrun', 'strip', '-S', '-x', player],
                    ['xcrun', 'dwarfdump', '--uuid', player], ['/trusted/llvm-readobj', '--sections', player],
                    ['incant_game_player', '--package', '/private/data']]
        calls = []
        for index, command in enumerate(commands):
            row = {'argv': command, 'finished': True, 'cleanup_confirmed': True, 'exit': 0}
            for stream in (('stdout', 'stderr') if index in (0, 8) else ('stdout',)):
                path = f'{index}.{stream}.log'
                content = (f'{index}: merged diagnostics\n' if 0 < index < 8 else 'ordinary\n').encode()
                (root / path).write_bytes(content)
                row[stream] = {'path': path, 'sha256': oracle.sha(content)}
            calls.append(row)
        return calls

    def test_exact_merged_symbol_streams_remain_hash_checked(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); calls = self.fixture(root)
            managed_logs(root, calls)
            (root / calls[2]['stdout']['path']).write_text('changed merged stderr')
            with self.assertRaisesRegex(ValueError, 'log identity'):
                managed_logs(root, calls)

    def test_missing_ordinary_stderr_is_not_treated_as_merged(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); calls = self.fixture(root)
            del calls[-1]['stderr']
            with self.assertRaisesRegex(ValueError, 'Seven merged'):
                managed_logs(root, calls)

    def test_missing_symbol_call_or_output_refuses(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); calls = self.fixture(root)
            with self.assertRaisesRegex(ValueError, 'Seven merged'):
                managed_logs(root, calls[:3] + calls[4:])
            del calls[2]['stdout']
            with self.assertRaises(KeyError):
                managed_logs(root, calls)

    def test_changed_symbol_command_or_linked_path_refuses(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); calls = self.fixture(root)
            calls[2]['argv'][2] = '--uuid'
            with self.assertRaisesRegex(ValueError, 'command inventory'):
                managed_logs(root, calls)
            calls = self.fixture(root)
            calls[1]['argv'][-1] = '/unrelated/incant_game_player-abc'
            with self.assertRaisesRegex(ValueError, 'symbol paths'):
                managed_logs(root, calls)

    def test_failed_merged_tool_is_not_a_valid_record(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); calls = self.fixture(root)
            calls[3]['exit'] = 1
            with self.assertRaisesRegex(ValueError, 'Successful merged'):
                managed_logs(root, calls)


class PublicTransactionDataTests(unittest.TestCase):
    def traces(self):
        before = {'revision': 2, 'project': {'scenes': {}}}
        selected = {'revision': 3, 'project': {'scenes': {'selected': {'entities': ['mover', 'audio']}}}}
        saved = selected | {'revision': 5}
        operations = ['project.read', 'command.execute', 'project.read', 'history.undo', 'project.read',
                      'history.redo', 'project.read', 'project.save', 'project.read']
        values = [before, {'revision': 3}, selected, {}, before | {'revision': 4}, {}, saved,
                  {'saved': True}, saved]
        rows = [{'request': {'id': n, 'method': method, 'params': {}},
                 'response': {'id': n, 'result': result}}
                for n, (method, result) in enumerate(zip(operations, values), 1)]
        rows[1]['request']['params'] = {'expected_revision': 2, 'commands': [
            {'op': op} for op in ['create_entity', 'create_entity', 'set_scene_program']]}
        return [{'exit': 0, 'requests': rows}, {'exit': 0, 'requests': [
            {'request': {'id': 1, 'method': 'project.read'}, 'response': {'id': 1, 'result': saved}}]}], saved

    def test_observed_atomic_undo_redo_save_reopen(self):
        traces, saved = self.traces()
        authoring(traces, saved)

    def test_undo_and_reopen_cannot_be_replaced_by_current_read(self):
        traces, saved = self.traces()
        traces[0]['requests'][4]['response']['result'] = saved
        with self.assertRaisesRegex(ValueError, 'Undo'):
            authoring(traces, saved)
        traces, saved = self.traces()
        traces[1]['requests'][0]['response']['result'] = {'revision': 6, 'project': saved['project']}
        with self.assertRaisesRegex(ValueError, 'reopened'):
            authoring(traces, saved)

    def test_private_or_nonatomic_authoring_refuses(self):
        traces, saved = self.traces()
        traces[0]['requests'][1]['request']['params']['commands'][-1]['op'] = 'write_project_json'
        with self.assertRaises(ValueError):
            authoring(traces, saved)
        traces, saved = self.traces()
        traces[0]['requests'][1]['response']['result']['revision'] = 99
        with self.assertRaises(ValueError):
            authoring(traces, saved)


if __name__ == '__main__':
    unittest.main()
