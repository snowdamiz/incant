"""Coherent data mutations: repair hash bindings, then require semantic refusal."""
import copy
import json
import unittest
import verify


def replace_record(packet, name, text):
    record = next(r for r in packet['records'] if r['name'] == name)
    old_hash, old_size = record['sha256'], record['bytes']
    raw = text.encode()
    new_hash = verify.sha(raw)
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

    def changed(self, name, change):
        packet = copy.deepcopy(self.packet)
        record = next(r for r in packet['records'] if r['name'] == name)
        value = json.loads(record['text'])
        change(value)
        replace_record(packet, name, json.dumps(value, indent=2) + '\n')
        return packet

    def test_actual_records(self):
        self.assertEqual(verify.replay(self.packet)['test_executions'], 69)

    def test_repeated_pid_cannot_claim_cleanup(self):
        packet = self.changed('cpu006/result.json', lambda r: r.update(absence=[r['absence'][0]] * len(r['absence'])))
        with self.assertRaisesRegex(ValueError, 'Distinct observed'):
            verify.replay(packet)

    def test_empty_pid_inventory_cannot_claim_cleanup(self):
        packet = self.changed('cpu006/result.json', lambda r: r.update(absence=[]))
        with self.assertRaisesRegex(ValueError, 'Distinct observed'):
            verify.replay(packet)

    def test_omitted_sampled_descendant_cannot_claim_cleanup(self):
        def omit(r):
            owner = next(x for x in self.packet['records'] if x['name'] == 'cpu006/output/owner.json')
            managed = {x['pid'] for x in json.loads(owner['text'])['owned']}
            omitted = next(x['pid'] for x in r['absence'] if x['pid'] not in managed)
            r['absence'] = [x for x in r['absence'] if x['pid'] != omitted]
        packet = self.changed('cpu006/result.json', omit)
        with self.assertRaisesRegex(ValueError, 'Exact observed identity inventory'):
            verify.replay(packet)

    def test_failed_terminal_cannot_be_a_success(self):
        packet = self.changed('cpu006/terminal-closeout.json', lambda r: r.update(actual_wrapper_exit=1))
        with self.assertRaisesRegex(ValueError, 'wrapper outcome'):
            verify.replay(packet)

    def test_cumulative_failure_cost_cannot_disappear(self):
        packet = self.changed('cpu006/result.json', lambda r: r.update(prior_charged_seconds=0))
        with self.assertRaisesRegex(ValueError, 'Cumulative failures'):
            verify.replay(packet)

    def test_guest_without_declaration_cannot_count_as_opted_in(self):
        packet = self.changed('cpu005/output/prepared-manifest.json', lambda r: r['modules'][0].update(persistent_state=None))
        with self.assertRaisesRegex(ValueError, 'Opt-in and legacy'):
            verify.replay(packet)

    def test_lower_actual_test_count_is_not_accepted(self):
        packet = copy.deepcopy(self.packet)
        name = 'cpu005/output/031.stdout.log'
        text = next(r['text'] for r in packet['records'] if r['name'] == name)
        replace_record(packet, name, text.replace('6 passed; 0 failed;', '5 passed; 0 failed;', 1))
        with self.assertRaisesRegex(ValueError, 'Executed test groups'):
            verify.replay(packet)

    def test_loosened_lint_cannot_pass(self):
        packet = self.changed('cpu006/output/owner.json', lambda r: r['calls'][1]['argv'].__setitem__(-2, '-A'))
        with self.assertRaisesRegex(ValueError, 'Exact matrix command'):
            verify.replay(packet)


if __name__ == '__main__':
    unittest.main()
