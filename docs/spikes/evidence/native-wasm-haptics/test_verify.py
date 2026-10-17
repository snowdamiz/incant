"""Coherent data-only mutations; archive bytes are never executed."""
import copy
import json
import unittest
import verify


def replace_record(packet, name, text):
    record = next(r for r in packet['records'] if r['name'] == name)
    old_hash, old_size = record['sha256'], record['bytes']
    raw = text.encode();new_hash = verify.sha(raw)
    record.update(text=text, bytes=len(raw), sha256=new_hash)
    for other in packet['records']:
        if other is record or old_hash not in other['text']:
            continue
        try:
            value = json.loads(other['text'])
        except ValueError:
            continue

        def repair(item):
            if isinstance(item, dict):
                if item.get('sha256') == old_hash and item.get('bytes') == old_size:
                    item['bytes'] = len(raw)
                return {key: repair(v) for key, v in item.items()}
            if isinstance(item, list):
                return [repair(v) for v in item]
            return new_hash if item == old_hash else item
        replace_record(packet, other['name'], json.dumps(repair(value), indent=2) + '\n')


class Evidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = verify.load()

    def change(self, name, change):
        packet = copy.deepcopy(self.packet)
        value = json.loads(next(r['text'] for r in packet['records'] if r['name'] == name))
        change(value)
        replace_record(packet, name, json.dumps(value, indent=2) + '\n')
        return packet

    def test_actual_records(self):
        self.assertEqual(verify.replay(self.packet)['actual_tests'], 47)

    def test_missing_or_changed_stream_refuses(self):
        packet = copy.deepcopy(self.packet)
        packet['records'] = [r for r in packet['records'] if r['name'] != 'cpu002/output/023.stdout.log']
        with self.assertRaises(KeyError):
            verify.replay(packet)
        packet = copy.deepcopy(self.packet)
        name = 'cpu002/output/023.stdout.log'
        text = next(r['text'] for r in packet['records'] if r['name'] == name)
        replace_record(packet, name, text.replace('6 passed; 0 failed;', '5 passed; 0 failed;', 1))
        with self.assertRaisesRegex(ValueError, 'accepted test counts'):
            verify.replay(packet)

    def test_lint_cannot_be_relaxed(self):
        packet = self.change('cpu004/output/owner.json', lambda v: v['calls'][0]['argv'].__setitem__(-2, '-A'))
        with self.assertRaisesRegex(ValueError, 'successful command'):
            verify.replay(packet)

    def test_empty_or_omitted_identity_refuses(self):
        for change in [lambda v: v.update(absence=[]), lambda v: v['absence'].pop()]:
            packet = self.change('cpu004/terminal-closeout.json', change)
            with self.assertRaisesRegex(ValueError, 'absence inventory'):
                verify.replay(packet)

    def test_failed_charge_cannot_disappear(self):
        packet = self.change('cpu003/result.json', lambda v: v.update(prior_seconds=0))
        with self.assertRaisesRegex(ValueError, 'earlier charges'):
            verify.replay(packet)

    def test_reused_producer_cannot_be_relabelled(self):
        packet = self.change('cpu002/source-compatibility.json', lambda v: v.update(native_producer_head=verify.SOURCES[1]))
        with self.assertRaisesRegex(ValueError, 'producer provenance'):
            verify.replay(packet)

    def test_declared_guest_cannot_be_dropped(self):
        packet = self.change('cpu002/output/prepared-manifest.json', lambda v: v['modules'][0].update(persistent_state=None))
        with self.assertRaisesRegex(ValueError, 'declared guest state'):
            verify.replay(packet)


if __name__ == '__main__':
    unittest.main()
