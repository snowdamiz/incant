#!/usr/bin/env python3
"""Prepare isolated getter-only source copies. Never builds or modifies registry."""
import argparse
import hashlib
import json
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CHANGED = {
    'rquickjs-sys': ['quickjs/quickjs.c', 'quickjs/quickjs.h', 'src/bindings/aarch64-apple-darwin.rs'],
    'rquickjs-core': ['src/runtime.rs', 'src/runtime/base.rs', 'src/runtime/raw.rs'],
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(root):
    return {p.relative_to(root).as_posix(): sha(p) for p in sorted(root.rglob('*')) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('registry', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    out = args.destination.resolve()
    registry = args.registry.resolve()
    assert not out.exists() and ROOT not in out.parents and registry not in out.parents
    pins = json.loads((HERE / 'pins.json').read_text())
    before = {}
    for crate in CHANGED:
        source = registry / (crate + '-0.14.0')
        assert not any(p.is_symlink() for p in source.rglob('*'))
        before[crate] = inventory(source)
        for path, digest in pins[crate].items():
            assert before[crate][path] == digest, (crate, path)
    assert before['rquickjs-sys']['quickjs/quickjs.c'] == '3a6b52a225c21709ef1fd314183efd3af862e867ce4ef8caabcae67922a745b3'
    manifests = {}
    for variant in ['getter', 'wrong-count']:
        package = out / variant
        for crate in CHANGED:
            copied = package / crate
            shutil.copytree(registry / (crate + '-0.14.0'), copied)
            patch = HERE / ('sys.patch' if crate == 'rquickjs-sys' else 'core.patch')
            result = subprocess.run(['patch', '--batch', '--fuzz=0', '-p1', '-i', str(patch)], cwd=copied, capture_output=True, check=True)
            (package / (crate + '.patch.stdout')).write_bytes(result.stdout)
            (package / (crate + '.patch.stderr')).write_bytes(result.stderr)
        if variant == 'wrong-count':
            path = package / 'rquickjs-sys/quickjs/quickjs.c'
            text = path.read_text()
            anchor = '*allocation_count = rt->malloc_state.malloc_count;'
            assert text.count(anchor) == 1
            path.write_text(text.replace(anchor, '*allocation_count = rt->malloc_state.malloc_count + 1;'))
        (package / 'src').mkdir()
        shutil.copy2(HERE / 'main.rs', package / 'src/main.rs')
        shutil.copy2(ROOT / 'Cargo.lock', package / 'Cargo.lock')
        (package / 'Cargo.toml').write_text('''[package]
name = "incant-quickjs-malloc-state-probe"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
rquickjs = "=0.14.0"
serde_json = "=1.0.151"
[patch.crates-io]
rquickjs-sys = { path = "rquickjs-sys" }
rquickjs-core = { path = "rquickjs-core" }
[profile.ci]
inherits = "dev"
opt-level = 1
debug = 1
''')
        manifests[variant] = {}
        for crate, paths in CHANGED.items():
            after = inventory(package / crate)
            changed = sorted(p for p in set(before[crate]) | set(after) if before[crate].get(p) != after.get(p))
            assert changed == sorted(paths), (crate, changed)
            manifests[variant][crate] = after
    for crate in CHANGED:
        assert inventory(registry / (crate + '-0.14.0')) == before[crate]
    manifest = {'diagnostic_only': True, 'root': str(ROOT), 'registry': str(registry),
                'original_crates': before, 'variants': manifests, 'owned_sources': inventory(HERE),
                'root_lock_sha256': sha(ROOT / 'Cargo.lock'), 'target': 'aarch64-apple-darwin',
                'limits': 'Source preparation only; default libc, no custom allocator or Float64 store patch. No build/timing/production claim.'}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'destination': str(out), 'variants': list(manifests)}))


if __name__ == '__main__':
    main()
