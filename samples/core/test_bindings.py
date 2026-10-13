"""Binding/proof boundary tests; synthetic data is not native execution evidence."""
import copy
import struct
import unittest

import bindings
import binding_oracle as oracle
from verify_bindings import same_source


def document():
    entities = {owner: {'components': {'UiNode': {'kind': {'role':
        'container' if role in ('result', 'inactive') else 'label'}}}}
        for role, owner in bindings.HUD.items() if role != 'table'}
    entities.update({owner: {'components': {'AudioSource': {'gain_db': -4}}}
                     for owner in bindings.AUDIO.values()})
    entities[bindings.CONTROLLER] = {'components': {'Script': {
        'source': bindings.SCRIPT, 'props': {'speed': 3.4, 'enemySpeed': 1.7}}}}
    return {'scenes': {bindings.SCENE: {'entities': entities}},
            'string_tables': {bindings.HUD['table']: {}},
            'settings': {'input_actions': {'move': {'kind': 'axis2', 'dead_zone': .1}}}}


class Bindings(unittest.TestCase):
    def test_undo_changes_revision_but_must_restore_source_and_history(self):
        before = {'revision': 0, 'source': 'old', 'version': {'sha256': 'abc'}, 'matches_history': True}
        self.assertTrue(same_source({**before, 'revision': 2}, before))
        self.assertFalse(same_source({**before, 'revision': 2, 'source': 'other'}, before))
        self.assertFalse(same_source({**before, 'revision': 2, 'matches_history': False}, before))

    def test_existing_data_and_source_version_remain_owned(self):
        value = document(); original = copy.deepcopy(value)
        version = {'state': 'present', 'sha256': 'a' * 64}
        result = bindings.commands(value, {'version': version}, 'trusted source')
        self.assertEqual(value, original)
        self.assertEqual(len(result), 3)
        self.assertEqual(result[0]['expected_source'], version)
        self.assertEqual(result[1]['value']['props']['speed'], 3.4)
        result[1]['value']['props']['speed'] = 99
        self.assertEqual(value, original)
        self.assertEqual(result[2]['actions']['move'], value['settings']['input_actions']['move'])

    def test_wrong_role_is_not_silently_bound(self):
        value = document()
        value['scenes'][bindings.SCENE]['entities'][bindings.HUD['health']]['components']['UiNode']['kind']['role'] = 'container'
        with self.assertRaisesRegex(ValueError, '^Core HUD role mismatch: health$'):
            bindings.commands(value, {}, '')

    def test_wrong_source_is_not_silently_replaced(self):
        value = document()
        value['scenes'][bindings.SCENE]['entities'][bindings.CONTROLLER]['components']['Script']['source'] = 'foreign'
        with self.assertRaisesRegex(ValueError, '^Core controller source identity mismatch$'):
            bindings.commands(value, {}, '')

    def test_missing_table_is_not_silently_literalized(self):
        value = document(); value['string_tables'].clear()
        with self.assertRaisesRegex(ValueError, '^Core HUD string table is missing$'):
            bindings.commands(value, {}, '')
        del value['string_tables']  # Canonical document serialization omits empty tables.
        with self.assertRaisesRegex(ValueError, '^Core HUD string table is missing$'):
            bindings.commands(value, {}, '')

    def test_restart_timeline_preserves_original_route_and_separate_host_steps(self):
        one = oracle.cases()['win']; original = copy.deepcopy(one)
        two = oracle.cases()['idle']
        value = oracle.restarted(one, two, held=True)
        self.assertEqual(one, original)
        self.assertEqual(value['ticks'], one['ticks'] + two['ticks'])
        events = {r['tick']: r['events'] for r in value['frames']}
        self.assertEqual(events[one['ticks']], [{'type': 'key', 'code': 'KeyR', 'down': True}])
        self.assertNotIn(one['ticks'] + 1, events)
        receipt = {'entities': {'player': 'p'}, 'enemies': [], 'samples': ['s']}
        plan = oracle.restart_plan(receipt, 'win', 'idle', one, two, held=True)
        self.assertEqual([r['tick'] for r in plan['checks'] if r['name'].startswith('fresh-pickup-')],
                         [1, one['ticks'] + 1])
        self.assertEqual(len([r for r in plan['checks'] if r['name'].startswith('held-neutral-')]), 3)

    def test_pcm_oracle_requires_each_cue_and_rejects_unrelated_changes(self):
        size = 8 * oracle.TICK_BYTES
        silent = bytes(size); actual = bytearray(silent)
        actual[oracle.TICK_BYTES:oracle.TICK_BYTES + 4] = struct.pack('<f', .25)
        report = {'logs': {'entries': [{'tick': 2, 'message': 'core.pickup id 1/4'}]}}
        result = oracle.cue_difference(bytes(actual), silent, report, 'pickup', .02)
        self.assertEqual(result['ticks'], [2])
        with self.assertRaises(AssertionError):
            oracle.cue_difference(silent, silent, report, 'pickup', .02)
        actual[0:4] = struct.pack('<f', .25)
        with self.assertRaises(AssertionError):
            oracle.cue_difference(bytes(actual), silent, report, 'pickup', .02)

    def test_complete_restart_routes_fit_ordinary_admission_without_shortening_win(self):
        recordings, routes = oracle.restart_cases()
        self.assertEqual(recordings['win'], oracle.cases()['win'])
        self.assertEqual(recordings['lose-short']['ticks'], 900)
        self.assertGreater(recordings['lose-short']['ticks'], 616 + 60)
        self.assertEqual({(a, b) for _, a, b, _, _ in routes if b != 'idle'},
                         {('win', 'lose-short'), ('lose-short', 'win')})
        for _, a, b, pad, held in routes:
            self.assertLessEqual(oracle.restarted(recordings[a], recordings[b], pad=pad, held=held)['ticks'], 3600)

    def test_restart_recording_keeps_new_pad_connection_but_omits_existing_one(self):
        two = oracle.timeline([{'tick': 1, 'events': [{'type': 'gamepad_connected', 'id': 0}]},
                              {'tick': 2, 'events': [{'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': 1}]}], 30)
        for first, expected in [(oracle.timeline([], 900), 1),
                                (oracle.timeline([{'tick': 1, 'events': [{'type': 'gamepad_connected', 'id': 0}]}], 900), 0)]:
            value = oracle.restarted(first, two)
            connects = [e for row in value['frames'] if row['tick'] > 900 for e in row['events']
                        if e['type'] == 'gamepad_connected']
            self.assertEqual(len(connects), expected)
            self.assertEqual(next(row['events'] for row in value['frames'] if row['tick'] == 902), two['frames'][1]['events'])


if __name__ == '__main__':
    unittest.main()
