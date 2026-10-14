#!/usr/bin/env python3
"""Copy two pinned interpreters and a deliberate wrong-store control; never build."""
import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PIN = '3a6b52a225c21709ef1fd314183efd3af862e867ce4ef8caabcae67922a745b3'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(path):
    return {p.relative_to(path).as_posix(): sha(p) for p in sorted(path.rglob('*')) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('registry_sys', type=Path, help='Exact cached rquickjs-sys-0.14.0 directory')
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    source, out = args.registry_sys.resolve(), args.destination.resolve()
    assert source.name == 'rquickjs-sys-0.14.0' and sha(source / 'quickjs/quickjs.c') == PIN
    assert not out.exists() and source not in out.parents and ROOT not in out.parents
    assert not any(p.is_symlink() for p in source.rglob('*'))
    assert (source / 'quickjs/LICENSE').is_file()
    before = inventory(source)
    out.mkdir(parents=True)
    manifests = {}
    for variant in ('baseline', 'candidate', 'wrong-store'):
        package = out / variant
        vendor = package / 'rquickjs-sys'
        shutil.copytree(source, vendor)
        if variant != 'baseline':
            result = subprocess.run(['patch', '--batch', '--fuzz=0', '-p1', '-i', str(HERE / 'numeric-store.patch')],
                                    cwd=vendor, capture_output=True, check=True)
            (package / 'patch.stdout').write_bytes(result.stdout)
            (package / 'patch.stderr').write_bytes(result.stderr)
        if variant == 'wrong-store':
            path = vendor / 'quickjs/quickjs.c'
            text = path.read_text()
            anchor = '                            p->u.array.u.double_ptr[idx] = d;'
            assert text.count(anchor) == 1
            path.write_text(text.replace(anchor, '                            (void)d; /* deliberate omitted store */'))
        (package / 'src').mkdir()
        shutil.copyfile(HERE / 'main.rs', package / 'src/main.rs')
        shutil.copyfile(ROOT / 'Cargo.lock', package / 'Cargo.lock')
        (package / 'Cargo.toml').write_text('''[package]
name = "incant-quickjs-f64-store-probe"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
rquickjs = "=0.14.0"
serde_json = "=1.0.151"
[patch.crates-io]
rquickjs-sys = { path = "rquickjs-sys" }
[profile.ci]
inherits = "dev"
opt-level = 1
debug = 1
''')
        after = inventory(vendor)
        changed = [name for name in sorted(set(before) | set(after)) if before.get(name) != after.get(name)]
        assert changed == ([] if variant == 'baseline' else ['quickjs/quickjs.c'])
        manifests[variant] = {'copied_files': after, 'changed_from_registry': changed,
                             'rust_main_sha256': sha(package / 'src/main.rs'),
                             'initial_lock_sha256': sha(package / 'Cargo.lock'),
                             'cargo_sha256': sha(package / 'Cargo.toml')}
    assert inventory(source) == before
    manifest = {'diagnostic_only': True, 'root': str(ROOT), 'registry': str(source),
                'registry_files': before, 'patch_sha256': sha(HERE / 'numeric-store.patch'),
                'variants': manifests, 'root_lock_sha256': sha(ROOT / 'Cargo.lock'),
                'source_files': inventory(HERE),
                'limits': 'Standalone diagnostic copies only; normal dependency graph unchanged. No build/timing performed.'}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'destination': str(out), 'registry_files': len(before), 'variants': list(manifests)}))


if __name__ == '__main__':
    main()
