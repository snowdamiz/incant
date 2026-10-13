"""Lib/bin probe audit controls; synthetic logs are not a shipping result."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from shipping import player_units
from test_native_codegen import invocation

TARGET = 'aarch64-apple-darwin'


def pair():
    return [invocation('incant_game_player', kind) for kind in ('lib', 'bin')]


def verify(lines, features=('project',)):
    return player_units('\n'.join(lines), TARGET, 'packed', features)


class PlayerProbeCodegenTests(unittest.TestCase):
    def test_both_units_keep_separate_shipping_evidence(self):
        flags = verify(pair())
        self.assertEqual(set(flags), {'incant_game_player', 'incant_game_player_library'})
        self.assertEqual(flags['incant_game_player']['lto'], 'fat')
        self.assertNotIn('lto', flags['incant_game_player_library'])

    def test_missing_duplicate_or_wrong_kind_is_not_a_count_relaxation(self):
        for i in (0, 1):
            for lines in (pair()[:i] + pair()[i+1:], pair() + [pair()[i]],
                          [line.replace('--crate-type lib', '--crate-type staticlib') for line in pair()]):
                with self.subTest(i=i, lines=lines), self.assertRaises(ValueError):
                    verify(lines)

    def test_each_unit_must_have_exact_selected_features(self):
        for i in (0, 1):
            for replacement in ('compiler', 'native-window', ''):
                lines = pair(); lines[i] = lines[i].replace('feature="project"', f'feature="{replacement}"')
                with self.subTest(i=i, replacement=replacement), self.assertRaises(ValueError):
                    verify(lines)
        for features in (('project', 'compiler'), ('native-window',)):
            with self.assertRaises(ValueError):
                verify(pair(), features)

    def test_host_wrong_target_and_duplicate_target_cannot_substitute(self):
        for i in (0, 1):
            for replacement in ('', '--target wrong-target', '--target ' + TARGET + ' --target ' + TARGET):
                lines = pair(); lines[i] = lines[i].replace('--target ' + TARGET, replacement)
                with self.subTest(i=i, replacement=replacement), self.assertRaises(ValueError):
                    verify(lines)
        self.assertEqual(verify(pair()), verify(pair() + [pair()[0].replace('--target ' + TARGET, '')]))

    def test_library_and_binary_both_enforce_shipping_and_no_test(self):
        for i in (0, 1):
            for old, new in (('opt-level=3', 'opt-level=1'), ('codegen-units=1', 'codegen-units=16'),
                             ('panic=abort', 'panic=unwind'), ('debuginfo=2', 'debuginfo=0'),
                             ('split-debuginfo=packed', 'split-debuginfo=off'),
                             ('--emit=dep-info,link', '--test --emit=dep-info,link'),
                             ('--emit=dep-info,link', '--emit=dep-info,link -C target-cpu=native')):
                lines = pair(); lines[i] = lines[i].replace(old, new)
                with self.subTest(i=i, flag=old), self.assertRaises(ValueError):
                    verify(lines)
        lines = pair(); lines[1] = lines[1].replace('lto=fat', 'lto=thin')
        with self.assertRaises(ValueError):
            verify(lines)

    def test_quoted_target_and_features_are_read_as_arguments(self):
        lines = [line.replace('--target ' + TARGET, "--target '" + TARGET + "'") for line in pair()]
        self.assertEqual(verify(lines), verify(pair()))


if __name__ == '__main__':
    unittest.main()
