"""Observable report decisions; every decision still runs all credential commands."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import credential_impact as impact

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'registry+https://example.invalid/index'


def package(name, dependencies=(), source=None, checksum=None, version='1.0.0'):
    values = {'name': name, 'version': version}
    if source:
        values['source'] = source
    if checksum:
        values['checksum'] = checksum
    values['dependencies'] = list(dependencies)
    return '\n[[package]]\n' + '\n'.join(f'{key} = {json.dumps(value)}' for key, value in values.items())


def fixture(extra='', checksum='a' * 64):
    return ('version = 4\n' + package('incant_agent', ['local'])
            + package('local', ['external'])
            + package('external', source=SOURCE, checksum=checksum) + extra)


class CredentialImpact(unittest.TestCase):
    def classify(self, before, after, paths=('Cargo.lock',)):
        report = impact.classify(paths, before, after)
        self.assertTrue(report['report_only'])
        self.assertEqual(report['decision'], 'run_all')
        return report

    def test_unrelated_lock_entry_is_reported_without_skipping(self):
        report = self.classify(fixture(), fixture(package('unrelated')))
        self.assertEqual(report['classification'], 'lock_only_unaffected')
        self.assertEqual(report['head_graph']['packages'], 3)

    def test_reachable_transitive_checksum_changes_run_full(self):
        self.assertEqual(self.classify(fixture(), fixture(checksum='b' * 64))['classification'], 'full')

    def test_git_revision_change_runs_full(self):
        before = fixture().replace(SOURCE, 'git+https://example.invalid/project#' + 'a' * 40)
        after = before.replace('#' + 'a' * 40, '#' + 'b' * 40)
        self.assertEqual(self.classify(before, after)['classification'], 'full')

    def test_every_lock_edge_counts_including_optional_target_build_and_dev(self):
        # Cargo.lock has no edge-kind labels. We retain them all, including
        # transitive dev edges that a particular package test may not compile.
        dependencies = ['optional', 'target_only', 'build_only', 'transitive_dev']
        before = 'version = 4\n' + package('incant_agent', ['local'])
        before += package('local', dependencies)
        for name in dependencies:
            before += package(name, source=SOURCE, checksum='a' * 64)
        for name in dependencies:
            with self.subTest(edge=name):
                after = before.replace(package(name, source=SOURCE, checksum='a' * 64),
                                       package(name, source=SOURCE, checksum='b' * 64))
                self.assertEqual(self.classify(before, after)['classification'], 'full')

    def test_reachable_edge_changes_run_full_even_if_packages_already_exist(self):
        before = fixture(package('other'))
        after = before.replace('dependencies = ["external"]', 'dependencies = ["external", "other"]')
        self.assertEqual(self.classify(before, after)['classification'], 'full')

    def test_unknown_and_ambiguous_graphs_never_report_unaffected(self):
        cases = [fixture() + '\n[metadata]\na = "b"\n',
                 fixture().replace('version = 4', 'version = 99', 1),
                 fixture().replace('"external"', '"missing"'),
                 fixture(package('external', source=SOURCE, checksum='b' * 64, version='2.0.0')),
                 fixture() + package('unused', ['missing']),
                 fixture(checksum='invalid'), fixture().replace('incant_agent', 'other_root')]
        for after in cases:
            with self.subTest(after=after):
                self.assertEqual(self.classify(fixture(), after)['classification'], 'full')

    def test_source_config_manifest_fixture_and_unknown_changes_run_full(self):
        for path in ['crates/incant_nav/tests/fixtures/precision-polygons.json',
                     'Cargo.toml', 'crates/new/Cargo.toml', '.cargo/config.toml',
                     'rust-toolchain.toml', 'README.md', 'unknown/new-input']:
            with self.subTest(path=path):
                self.assertEqual(self.classify(fixture(), fixture(), ['Cargo.lock', path])['classification'], 'full')

    def test_current_repository_lock_is_conservative_and_sensitive(self):
        before = (ROOT / 'Cargo.lock').read_text()
        graph = impact.locked_graph(before)
        self.assertIn('incant_doc', graph['local_packages'])
        self.assertIn('incant_import', graph['local_packages'])
        self.assertIn('rerecast', graph['local_packages'])
        after = before + package('unrelated_impact_fixture')
        self.assertEqual(self.classify(before, after)['classification'], 'lock_only_unaffected')

    def test_historical_physics_union_requires_full_run(self):
        fixture_path = Path(__file__).with_name('fixtures') / 'credential-impact-main56-main57.json'
        historical = json.loads(fixture_path.read_text())
        self.assertIn('crates/incant_doc/src/lib.rs', historical['changed_paths'])
        self.assertEqual(self.classify(None, None, historical['changed_paths'])['reason'], 'non_lock_inputs')

    def test_workflow_covers_every_reachable_local_crate_directory(self):
        # A new local edge cannot silently outgrow the manually listed triggers.
        names = impact.locked_graph((ROOT / 'Cargo.lock').read_text())['local_packages']
        paths = impact.git(ROOT, 'ls-files', '-z', '*Cargo.toml').split(b'\0')
        directories = {}
        for raw in paths:
            if not raw:
                continue
            path = Path(raw.decode())
            package = tomllib.loads((ROOT / path).read_text()).get('package', {})
            if package.get('name') in names:
                self.assertNotIn(package['name'], directories)
                directories[package['name']] = path.parent.as_posix()
        self.assertEqual(set(directories), set(names))
        workflow = (ROOT / '.github/workflows/credentials.yml').read_text()
        for directory in directories.values():
            self.assertIn(f"- '{directory}/**'", workflow)


class GitInputs(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Impact fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        (self.repo / 'Cargo.lock').write_text(fixture())
        self.base = self.commit()

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.repo), *args], stderr=subprocess.DEVNULL).decode().strip()

    def commit(self):
        self.git('add', '.')
        self.git('-c', 'commit.gpgsign=false', 'commit', '-qm', 'Test fixture')
        return self.git('rev-parse', 'HEAD')

    def test_exact_commits_and_lock_only_changes(self):
        (self.repo / 'Cargo.lock').write_text(fixture(package('unrelated')))
        report = impact.report(self.repo, self.base, self.commit(), 'pull_request')
        self.assertEqual(report['classification'], 'lock_only_unaffected')
        self.assertEqual(report['decision'], 'run_all')

    def test_rename_reports_both_paths_and_embedded_newline(self):
        (self.repo / 'Cargo.lock').rename(self.repo / 'renamed\nlock')
        report = impact.report(self.repo, self.base, self.commit(), 'pull_request')
        self.assertEqual(report['classification'], 'full')
        self.assertEqual(report['changed_paths'], ['Cargo.lock', 'renamed\nlock'])

    def test_missing_baseline_and_non_pr_events_keep_full_run(self):
        for base, event in [('0' * 40, 'pull_request'), ('--help', 'pull_request'),
                            (self.base, 'push'), (self.base, 'workflow_dispatch')]:
            with self.subTest(base=base, event=event):
                report = impact.report(self.repo, base, self.base, event)
                self.assertEqual(report['classification'], 'full')
                self.assertEqual(report['decision'], 'run_all')


if __name__ == '__main__':
    unittest.main()
