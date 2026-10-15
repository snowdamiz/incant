import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from macos_session import admit


class MacSessionPreflight(unittest.TestCase):
    def setUp(self):
        self.value = [{'IOConsoleLocked': False, 'IOConsoleUsers': [{
            'kCGSSessionOnConsoleKey': True, 'kCGSSessionUserIDKey': 501,
            'kCGSessionLoginDoneKey': True,
        }]}]

    def test_unlocked_login_with_absent_optional_secondary_flag(self):
        self.assertEqual(admit(self.value, 501), {'console_unlocked': True,
                         'own_console_login_complete': True, 'secondary_lock_field_present': False})

    def test_explicit_unlocked_secondary_flag(self):
        self.value[0]['IOConsoleUsers'][0]['CGSSessionScreenIsLocked'] = False
        self.assertTrue(admit(self.value, 501)['secondary_lock_field_present'])

    def test_selected_root_dictionary_has_the_same_admission(self):
        self.assertEqual(admit(self.value, 501), admit(self.value[0], 501))

    def test_lock_and_nonboolean_values_refuse(self):
        for node in ('root', 'user'):
            for flag in (True, 0, 1, 'false', None):
                value = copy.deepcopy(self.value)
                target = value[0] if node == 'root' else value[0]['IOConsoleUsers'][0]
                target['IOConsoleLocked' if node == 'root' else 'CGSSessionScreenIsLocked'] = flag
                with self.subTest(node=node, flag=flag), self.assertRaises(ValueError):
                    admit(value, 501)

    def test_missing_primary_or_login_metadata_refuses(self):
        for key in ('IOConsoleLocked', 'IOConsoleUsers'):
            value = copy.deepcopy(self.value); del value[0][key]
            with self.subTest(key=key), self.assertRaises(ValueError): admit(value, 501)
        for key in ('kCGSSessionOnConsoleKey', 'kCGSSessionUserIDKey', 'kCGSessionLoginDoneKey'):
            value = copy.deepcopy(self.value); del value[0]['IOConsoleUsers'][0][key]
            with self.subTest(key=key), self.assertRaises(ValueError): admit(value, 501)

    def test_different_user_incomplete_login_or_ambiguous_console_refuses(self):
        with self.assertRaises(ValueError): admit(self.value, 502)
        for key in ('kCGSSessionOnConsoleKey', 'kCGSessionLoginDoneKey'):
            value = copy.deepcopy(self.value); value[0]['IOConsoleUsers'][0][key] = False
            with self.subTest(key=key), self.assertRaises(ValueError): admit(value, 501)
        self.value[0]['IOConsoleUsers'].append(copy.deepcopy(self.value[0]['IOConsoleUsers'][0]))
        with self.assertRaises(ValueError): admit(self.value, 501)

    def test_contradictory_root_secondary_lock_refuses(self):
        self.value[0]['CGSSessionScreenIsLocked'] = True
        with self.assertRaises(ValueError): admit(self.value, 501)

    def test_malformed_top_level_or_user_list_refuses(self):
        for value in (None, {}, [], [{}, {}], [None], [{'IOConsoleLocked': False,
                                                      'IOConsoleUsers': [None]}]):
            with self.subTest(value=value), self.assertRaises(ValueError): admit(value, 501)


if __name__ == '__main__':
    unittest.main()
