"""Authoring/oracle controls. Synthetic data is not native execution evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import stress
from test_bindings import document


def base():
    value = document()
    value['scripts'] = {stress.bindings.SCRIPT: {'id': stress.bindings.SCRIPT, 'path': 'gameplay.ts'}}
    props = value['scenes'][stress.bindings.SCENE]['entities'][stress.bindings.CONTROLLER]['components']['Script']['props']
    props.update(audio=stress.bindings.AUDIO, camera=stress.ids.entity(3))
    return value


def extended(before):
    value = copy.deepcopy(before)
    for _, batch in stress.commands():
        for command in batch:
            if command['op'] == 'upsert_script':
                value['scripts'][command['script']['id']] = command['script']
            if command['op'] == 'create_entity':
                value['scenes'][stress.bindings.SCENE]['entities'][command['entity']['id']] = command['entity']
    return value


class Stress(unittest.TestCase):
    def test_exact_count_batches_unique_ids_and_isolated_source(self):
        batches = list(stress.commands())
        self.assertEqual(len(batches), 41)
        self.assertEqual([r['op'] for r in batches[0][1]], ['write_script', 'upsert_script'])
        entities = [r['entity'] for _, batch in batches[1:] for r in batch]
        self.assertEqual(len(entities), 9999)
        self.assertEqual(len({e['id'] for e in entities}), 9999)
        self.assertTrue(all(len(batch) <= 256 for _, batch in batches[1:]))
        self.assertEqual([e['id'] for e in entities], sorted(e['id'] for e in entities))
        self.assertTrue(all(set(e['components']) == {'Transform', 'Script'} for e in entities))

    def test_full_extension_preserves_original_content(self):
        before = base(); after = extended(before)
        membership = stress.verify_extension(before, after)
        self.assertEqual(len(membership), 9999)
        self.assertEqual(membership[-1]['ordinal'], 9998)
        invalid = copy.deepcopy(after)
        invalid['scenes'][stress.bindings.SCENE]['entities'][stress.bindings.CONTROLLER]['components']['Script']['props']['speed'] = 90
        with self.assertRaisesRegex(ValueError, 'original project'):
            stress.verify_extension(before, invalid)

    def test_membership_and_visibility_mutations_refuse(self):
        before = base(); after = extended(before)
        for edit in ('ordinal', 'source', 'component', 'parent', 'transform'):
            invalid = copy.deepcopy(after)
            entity = invalid['scenes'][stress.bindings.SCENE]['entities'][stress.owner(13)]
            if edit == 'ordinal': entity['components']['Script']['props']['ordinal'] = 12
            if edit == 'source': entity['components']['Script']['source'] = stress.bindings.SCRIPT
            if edit == 'component': entity['components']['Mesh'] = {}
            if edit == 'parent': entity['parent'] = stress.bindings.CONTROLLER
            if edit == 'transform': entity['components']['Transform']['translation'][0] = 1
            with self.subTest(edit=edit), self.assertRaises(ValueError):
                stress.verify_extension(before, invalid)

    def test_original_controller_cardinality_and_reserved_ids_refuse(self):
        for edit in ('duplicate', 'collision', 'camera'):
            invalid = base(); entities = invalid['scenes'][stress.bindings.SCENE]['entities']
            if edit == 'duplicate': entities['extra'] = copy.deepcopy(entities[stress.bindings.CONTROLLER])
            if edit == 'collision': entities[stress.owner(0)] = {'components': {}}
            if edit == 'camera': entities[stress.bindings.CONTROLLER]['components']['Script']['props']['camera'] = 'wrong'
            with self.subTest(edit=edit), self.assertRaises(ValueError): stress.validate_base(invalid)

    def test_closed_form_checksum_and_phase_wrap(self):
        # Independent integer residue sums; production writes/reads each typed row.
        def residues(count, modulus):
            q, r = divmod(count, modulus)
            return q * modulus * (modulus - 1) // 2 + r * (r - 1) // 2
        for tick in (1, 63, 64, 65, 3600):
            phase = (tick % 64 - 32) / 64
            expected = [9999, residues(9999, 97) / 9999 - 48 + phase,
                        residues(9999, 13) / 9999 - 6 - phase]
            for actual, target in zip(stress.sentinel(tick), expected): self.assertAlmostEqual(actual, target, places=12)
        for invalid in (True, 0, 3601, 1.5):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError): stress.sentinel(invalid)

    def test_observation_is_bounded_exact_and_includes_population(self):
        plan = stress.observation(64, 46, 4)
        self.assertEqual(plan['format'], 'incant-native-observation')
        self.assertEqual(len(plan['checks']), 7)
        self.assertEqual(plan['checks'][0]['expect']['value'], 10041)
        self.assertLess(len(json.dumps(plan)), 256 * 1024)
        with self.assertRaises(ValueError): stress.observation(2, 46, 47)
        for ordinal in (-1, 9999, True):
            with self.assertRaises(ValueError): stress.owner(ordinal)

    def test_inventory_does_not_follow_symbolic_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'source').write_text('kept')
            self.assertEqual(set(stress.inventory(root)), {'source'})
            (root / 'link').symlink_to(root / 'source')
            with self.assertRaises(ValueError): stress.inventory(root)

if __name__ == '__main__': unittest.main()
