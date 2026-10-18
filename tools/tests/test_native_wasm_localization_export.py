"""Pure data/admission controls. No Rust, guest, player, GPU or exporter runs."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from probes.native_wasm_localization import author, fixture as f, inputs, oracle


def report(start=0, end=f.TICKS):
    # Synthetic inputs qualify only the checker, never the native behavior.
    pcm = struct.pack('<f', .125) * ((end - start) * f.FRAMES * 2)
    xyz = f.position(end)
    value = {'tick': end, 'frames': (end - start) * f.FRAMES, 'pcm_sha256': oracle.sha(pcm),
        'flipbooks': [], 'logs': {'dropped': 0, 'entries': [
            {'tick': n, 'level': 'info', 'message': f.LOCALES[(n - 1) % 4]}
            for n in range(start + 1, end + 1)]},
        'observations': {'passed': True, 'tick': end, 'tick_rate': 60,
            'checks': [row | {'passed': True, 'actual': f.position(row['tick'])}
                       for row in f.observation(start, end)['checks']],
            'entities': {f.ENTITY: {'translation': xyz, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1],
                'parent': None, 'velocity': None,
                'world_transform': [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [*xyz, 1]]}}}}
    return value, pcm


def saved(tick=f.TICKS):
    state = b'INCWST01' + struct.pack('<II', 1, 8) + f.SCHEMA + bytes(8) + struct.pack('<QqQ', tick, tick, tick)
    elapsed = 0.
    for _ in range(tick):
        elapsed += 1. / 60.
    state = b'INCWPS02' + state + b'INCTMS01' + struct.pack('<QdII', tick, elapsed, 0, 80)
    header = b'INCSES01' + struct.pack('<II', 7, 1) + bytes([2]) * 64 + struct.pack('<4I', 1, len(state), 0, 1)
    payload = b'w' + state + b'a'
    return header + hashlib.sha256(header + payload).digest() + payload


class LocalizationExportTests(unittest.TestCase):
    def test_catalog_resolves_real_fallbacks_and_empty_is_present(self):
        self.assertEqual([(len(f.rows(i)), f.digest(f.rows(i))) for i in range(4)],
                         [(0, 2166136261), (130, 3509572390), (132, 999871359), (64, 2505266705)])
        for phase in range(4):
            rows = f.rows(phase)
            self.assertEqual([(r['table_id'], r['key']) for r in rows],
                             sorted((r['table_id'], r['key']) for r in rows))
            self.assertTrue(all(r['kind'] == 'translation' and r['resolved_locale'] is not None for r in rows))
            self.assertEqual(sum(r['key'] == 'empty' for r in rows), 2 if phase == 2 else 0)
        self.assertEqual(f.rows(1)[127]['table_id'], f.TABLE_B)
        self.assertEqual([r['key'] for r in f.rows(1)[128:]], ['k063', 'k064'])

    def test_row_identity_or_order_cannot_keep_witness(self):
        rows = f.rows(1)
        for field, bad in [('table_id', f.TABLE_B), ('key', 'wrong'), ('requested_locale', 'fr'),
                           ('resolved_locale', 'en'), ('kind', 'key')]:
            changed = copy.deepcopy(rows)
            changed[0][field] = bad
            self.assertNotEqual(f.digest(changed), f.digest(rows), field)
        self.assertNotEqual(f.digest(list(reversed(rows))), f.digest(rows))

    def test_public_authoring_rpc_stays_bounded(self):
        commands = author.commands('0' * 26, '1' * 26, '2' * 26)
        self.assertEqual([c['op'] for c in commands], ['upsert_string_table'] * 2 +
                         ['set_locale', 'create_entity', 'create_entity', 'set_scene_program'])
        self.assertLess(len(json.dumps({'id': 1, 'method': 'command.execute', 'params': {
            'commands': commands, 'expected_revision': 10, 'description': 'fixed proof'}}).encode()), 16 * 1024)

    def test_complete_and_resumed_synthetic_data_admitted(self):
        for start, end in oracle.RUNS.values():
            oracle.gameplay(*report(start, end), start, end)
            self.assertEqual(len(oracle.checkpoint(saved(end), end)), 64)

    def test_missing_reordered_and_false_tick_predicates_refused(self):
        base, pcm = report()
        for mode in ('missing', 'reordered', 'false', 'wrong-name'):
            value = copy.deepcopy(base)
            checks = value['observations']['checks']
            if mode == 'missing':
                checks.pop()
            elif mode == 'reordered':
                checks.reverse()
            elif mode == 'false':
                checks[2]['passed'] = False
            else:
                checks[2]['name'] = 'unrelated'
            with self.assertRaises(ValueError, msg=mode):
                oracle.gameplay(value, pcm, 0, f.TICKS)

    def test_wrong_total_digest_or_nan_refused(self):
        base, pcm = report()
        for component, bad in ((0, 7), (1, 129), (2, 0), (2, float('nan'))):
            value = copy.deepcopy(base)
            value['observations']['checks'][2]['actual'][component] = bad
            with self.assertRaises(ValueError):
                oracle.gameplay(value, pcm, 0, f.TICKS)

    def test_logs_precommit_and_boolean_numbers_refused(self):
        base, pcm = report()
        for mode in ('early-locale', 'drop', 'boolean-tick', 'boolean-check'):
            value = copy.deepcopy(base)
            if mode == 'early-locale':
                value['logs']['entries'][0]['message'] = 'fr-CA'
            elif mode == 'drop':
                value['logs']['dropped'] = 1
            elif mode == 'boolean-tick':
                value['tick'] = True
            else:
                value['observations']['checks'][1]['tick'] = True
            with self.assertRaises(ValueError, msg=mode):
                oracle.gameplay(value, pcm, 0, f.TICKS)

    def test_pcm_truncation_silence_nonfinite_and_wrong_hash_refused(self):
        value, pcm = report()
        for content in (pcm[:-4], bytes(len(pcm)), struct.pack('<f', float('inf')) * (len(pcm) // 4)):
            changed = value | {'pcm_sha256': oracle.sha(content)}
            with self.assertRaises(ValueError):
                oracle.gameplay(changed, content, 0, f.TICKS)
        with self.assertRaises(ValueError):
            oracle.gameplay(value | {'pcm_sha256': '0' * 64}, pcm, 0, f.TICKS)

    def test_coherent_wrong_saved_state_schema_and_clock_refused(self):
        for offset in (8, 137 + 16, 137 + 56, 137 + 72):
            content = bytearray(saved())
            content[offset] ^= 1
            content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
            with self.assertRaises(ValueError):
                oracle.checkpoint(bytes(content), f.TICKS)
        content = bytearray(saved())
        struct.pack_into('<q', content, 137 + 64, f.TICKS - 1)
        content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
        with self.assertRaises(ValueError):
            oracle.checkpoint(bytes(content), f.TICKS)
        with self.assertRaises(ValueError):
            oracle.checkpoint(saved()[:-1], f.TICKS)

    def test_timer_wrapper_and_empty_schedule_are_checked_not_ignored(self):
        # A coherent outer digest cannot hide changed timer framing, clock,
        # elapsed time, a fabricated row count or a wrong timer row stride.
        for offset in (129, 217, 225, 233, 241, 245):
            content = bytearray(saved())
            content[offset] ^= 1
            content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
            with self.assertRaises(ValueError, msg=str(offset)):
                oracle.checkpoint(bytes(content), f.TICKS)
        # The prior synthetic fixture used legacy framing and missed the actual
        # current host wrapper. Keep it as an explicit refused control.
        content = bytearray(saved())
        content[84:88] = struct.pack('<I', 80)
        del content[217:249]
        del content[129:137]
        content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
        with self.assertRaises(ValueError):
            oracle.checkpoint(bytes(content), f.TICKS)

    def test_exact_refusal_not_unrelated_startup_or_success(self):
        for kind in f.FAILURES.values():
            detail = 'Boundary(Limit("localization query quota"))' if kind == 'quota' else 'Trap(1)'
            value = {'returncode': 1, 'stdout': '', 'stderr': f'Error: Session(Module({detail}))\n',
                     'outputs_present': [False, False], 'staging': []}
            oracle.refusal(value, kind)
            for key, bad in [('returncode', 0), ('stdout', '{}'), ('stderr', 'Error: missing input'),
                             ('outputs_present', [True, False]), ('staging', ['.incant-partial'])]:
                with self.assertRaises(ValueError):
                    oracle.refusal(value | {key: bad}, kind)

    def test_refusal_input_is_explicit_first_tick_key(self):
        for key in f.FAILURES:
            self.assertEqual(f.recording(1, key)['frames'][0], {'tick': 1,
                'events': [{'type': 'key', 'code': key, 'down': True}]})
        with self.assertRaises(ValueError):
            f.recording(1, 'Unknown')

    def test_all_ambient_compiler_overrides_refused_before_child(self):
        keys = ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTDOC', 'RUSTDOCFLAGS',
                'CARGO_ENCODED_RUSTDOCFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
                'CARGO_BUILD_RUSTC', 'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER',
                'CARGO_PROFILE_SHIPPING_LTO', 'CARGO_FEATURE_COMPILER',
                'CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS']
        with patch('subprocess.run', side_effect=AssertionError('No native child permitted')) as child:
            for key in keys:
                with self.assertRaises(ValueError, msg=key):
                    inputs.environment_admission({key: 'unapproved'})
            inputs.environment_admission({'HOME': '/trusted', 'RUSTFLAGS': ''})
            child.assert_not_called()


if __name__ == '__main__':
    unittest.main()
