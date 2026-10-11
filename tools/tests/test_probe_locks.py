"""Fast structural guard; Cargo's locked builds still validate full resolution."""
from pathlib import Path
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return tomllib.loads(path.read_text(encoding='utf-8'))


def verify_probe(root, manifest):
    """Check required local edges and inherited source patches, without Cargo."""
    workspace = read(root / 'Cargo.toml')
    probe = read(manifest)
    packages = read(manifest.with_name('Cargo.lock'))['package']
    local = {}
    for package in packages:
        if 'source' not in package:
            if package['name'] in local:
                raise ValueError('ambiguous local package')
            local[package['name']] = package
    pending = [(manifest, True)]
    seen = set()
    while pending:
        path, is_root = pending.pop()
        path = path.resolve()
        if path in seen:
            continue
        seen.add(path)
        data = read(path)
        name = data['package']['name']
        if name not in local:
            raise ValueError(f'missing local package {name}')
        locked_edges = {dep.split()[0] for dep in local[name].get('dependencies', [])}
        sections = [data, *data.get('target', {}).values()]
        kinds = ('dependencies', 'build-dependencies', 'dev-dependencies') if is_root else ('dependencies', 'build-dependencies')
        for section in sections:
            for kind in kinds:
                for alias, spec in section.get(kind, {}).items():
                    if not isinstance(spec, dict):
                        continue
                    base = path.parent
                    if spec.get('workspace'):
                        inherited = workspace['workspace']['dependencies'][alias]
                        if not isinstance(inherited, dict):
                            continue
                        spec = {**inherited, **spec}
                        base = root
                    if spec.get('optional') or 'path' not in spec:
                        continue
                    child = (base / spec['path'] / 'Cargo.toml').resolve()
                    child_name = read(child)['package']['name']
                    if child_name not in locked_edges:
                        raise ValueError(f'missing required local edge {name} -> {child_name}')
                    pending.append((child, False))
    # A path-dependent engine crate does not carry the engine root's patches
    # into an independent workspace. Keep any reachable patched source exact.
    names = {p['name'] for p in packages}
    for name, spec in workspace.get('patch', {}).get('crates-io', {}).items():
        if name not in names or 'path' not in spec:
            continue
        actual = probe.get('patch', {}).get('crates-io', {}).get(name, {})
        if ('path' not in actual or
                (manifest.parent / actual['path']).resolve() != (root / spec['path']).resolve() or
                name not in local):
            raise ValueError(f'probe must preserve engine source patch for {name}')


class ProbeLockTests(unittest.TestCase):
    def test_committed_isolated_probe_locks_preserve_required_local_edges(self):
        locks = sorted((ROOT / 'tools/probes').glob('*/Cargo.lock'))
        self.assertTrue(locks)
        for lock in locks:
            with self.subTest(probe=lock.parent.name):
                verify_probe(ROOT, lock.with_name('Cargo.toml'))

    def fixture(self, edge=True, package=True):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        (root / 'Cargo.toml').write_text('[workspace]\n[workspace.dependencies]\nengine = { path = "engine" }\n')
        (root / 'engine').mkdir()
        (root / 'engine/Cargo.toml').write_text('[package]\nname="engine"\nversion="1.0.0"\n')
        (root / 'probe').mkdir()
        manifest = root / 'probe/Cargo.toml'
        manifest.write_text('[package]\nname="probe"\nversion="1.0.0"\n[workspace]\n[target.\'cfg(unix)\'.dependencies]\nrenamed = { package="engine", path="../engine" }\n')
        lock = '[[package]]\nname="probe"\nversion="1.0.0"\n'
        if edge:
            lock += 'dependencies=["engine"]\n'
        if package:
            lock += '[[package]]\nname="engine"\nversion="1.0.0"\n'
        manifest.with_name('Cargo.lock').write_text(lock)
        return root, manifest

    def test_stale_local_dependency_edge_is_rejected_even_if_package_exists(self):
        root, manifest = self.fixture(edge=False)
        with self.assertRaisesRegex(ValueError, 'missing required local edge probe -> engine'):
            verify_probe(root, manifest)

    def test_missing_transitive_local_package_is_rejected(self):
        root, manifest = self.fixture(package=False)
        with self.assertRaisesRegex(ValueError, 'missing local package engine'):
            verify_probe(root, manifest)

    def test_target_renaming_and_workspace_inheritance_are_followed(self):
        root, manifest = self.fixture()
        (root / 'engine/Cargo.toml').write_text('[package]\nname="engine"\nversion="1.0.0"\n[dependencies]\nengine_child.workspace=true\n')
        with (root / 'Cargo.toml').open('a') as f:
            f.write('engine_child = { path="child" }\n')
        (root / 'child').mkdir()
        (root / 'child/Cargo.toml').write_text('[package]\nname="engine_child"\nversion="1.0.0"\n')
        with self.assertRaisesRegex(ValueError, 'engine -> engine_child'):
            verify_probe(root, manifest)
        with manifest.with_name('Cargo.lock').open('a') as f:
            f.write('dependencies=["engine_child"]\n[[package]]\nname="engine_child"\nversion="1.0.0"\n')
        verify_probe(root, manifest)

    def test_registry_substitution_cannot_replace_engine_path_patch(self):
        root, manifest = self.fixture()
        with (root / 'Cargo.toml').open('a') as f:
            f.write('[patch.crates-io]\npatched={path="vendor"}\n')
        with manifest.with_name('Cargo.lock').open('a') as f:
            f.write('[[package]]\nname="patched"\nversion="1.0.0"\nsource="registry+https://github.com/rust-lang/crates.io-index"\n')
        with self.assertRaisesRegex(ValueError, 'source patch for patched'):
            verify_probe(root, manifest)
        with manifest.open('a') as f:
            f.write('[patch.crates-io]\npatched={path="../vendor"}\n')
        # Merely declaring the patch still fails while the lock selects registry.
        with self.assertRaisesRegex(ValueError, 'source patch for patched'):
            verify_probe(root, manifest)
        lock = manifest.with_name('Cargo.lock')
        lock.write_text(lock.read_text().replace('source="registry+https://github.com/rust-lang/crates.io-index"\n', ''))
        verify_probe(root, manifest)


if __name__ == '__main__':
    unittest.main()
