"""Regression checks for the playtest oracle; real player evidence is separate."""
import copy
import json
from pathlib import Path
import unittest

import playtest


class LockedExitOracleTests(unittest.TestCase):
    def setUp(self):
        self.receipt = {'entities': {'player': 'player', 'exit': 'exit'},
                        'enemies': ['chaser'], 'samples': ['s1', 's2', 's3', 's4']}
        self.tick = playtest.LOCKED_ARRIVAL + 29
        requested = playtest.observe(self.receipt, self.tick, locked=True)
        rows = []
        for check in requested['checks']:
            expect = check['expect']
            actual = expect['value'] if expect['type'] == 'exists' else list(playtest.level.EXIT)
            rows.append({**check, 'actual': actual, 'passed': True})
        # The actual native observer canonicalizes these instead of retaining request order.
        rows.sort(key=lambda row: (row['tick'], row['name']))
        self.report = {'logs': {'dropped': 0, 'entries': [
            {'tick': 127, 'message': 'core.pickup s1 1/4'},
            {'tick': 508, 'message': 'core.pickup s2 2/4'},
            {'tick': 664, 'message': 'core.damage health=2'},
            {'tick': 1266, 'message': 'core.pickup s3 3/4'}]},
            'observations': {'passed': True, 'checks': rows}}

    def check(self, report=None):
        return playtest.locked_exit_checks(report or self.report, self.receipt, self.tick)

    def test_live_incomplete_inventory_inside_exit_for_all_thirty_ticks(self):
        result = self.check()
        self.assertTrue(result['passed'])
        self.assertEqual(len(result['exit_distances']), 30)
        self.assertEqual(result['health'], 2)

    def test_old_no_win_no_pickup_assertion_admitted_dead_actor_before_exit(self):
        self.report['logs']['entries'] = [
            {'tick': 608, 'message': 'core.damage health=0'},
            {'tick': 608, 'message': 'core.lost tick=608 pickups=0'}]
        self.report['observations']['checks'] = [self.report['observations']['checks'][-1]]
        result = self.check()
        self.assertFalse(result['passed'])
        self.assertFalse(result['checks']['live_player'])
        self.assertFalse(result['checks']['inside_exit_for_entire_dwell'])

    def test_actual_distance_checked_even_if_pass_flag_claims_success(self):
        for row in self.report['observations']['checks']:
            if row['name'] == f'locked-player-{playtest.LOCKED_ARRIVAL + 10}':
                row['actual'] = [playtest.level.EXIT[0] + 1.01, *playtest.level.EXIT[1:]]
        self.assertFalse(self.check()['checks']['inside_exit_for_entire_dwell'])

    def test_reject_terminal_incomplete_logs_and_complete_inventory(self):
        for message in ('core.lost tick=1588 pickups=3', 'core.won tick=1588 health=2 pickups=3',
                        'core.pickup s4 4/4'):
            report = copy.deepcopy(self.report)
            report['logs']['entries'].append({'tick': self.tick, 'message': message})
            self.assertFalse(self.check(report)['passed'], message)
        self.report['logs']['dropped'] = 1
        self.assertFalse(self.check()['passed'])

    def test_reject_missing_or_wrong_owner_observation(self):
        report = copy.deepcopy(self.report)
        report['observations']['checks'].pop(5)
        self.assertFalse(self.check(report)['passed'])
        self.report['observations']['checks'][0]['expect']['entity'] = 'replacement-exit'
        self.assertFalse(self.check()['passed'])

    def test_corrupt_actual_exists_cannot_hide_behind_success_flag(self):
        for name in ('runs-to-end', f'locked-inventory-0-{self.tick}',
                     f'locked-inventory-3-{self.tick}'):
            report = copy.deepcopy(self.report)
            row = next(r for r in report['observations']['checks'] if r['name'] == name)
            row['actual'] = not row['expect']['value']
            self.assertFalse(self.check(report)['passed'], name)

    def test_recording_keeps_original_win_prefix_and_stops_for_entire_dwell(self):
        root = Path(__file__).parent / 'routes'
        locked = json.loads((root / 'locked-input.json').read_text())
        win = json.loads((root / 'win-input.json').read_text())
        self.assertEqual([f for f in locked['frames'] if f['tick'] < 1274],
                         [f for f in win['frames'] if f['tick'] < 1274])
        stop = locked['frames'][-1]
        self.assertEqual(stop['tick'], playtest.LOCKED_ARRIVAL)
        self.assertEqual(stop['events'], [
            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_x', 'value': 0},
            {'type': 'gamepad_axis', 'id': 0, 'axis': 'left_y', 'value': 0}])
        self.assertEqual(locked['ticks'], self.tick)


if __name__ == '__main__':
    unittest.main()
