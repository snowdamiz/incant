#!/usr/bin/env python3
"""Create a private, pinned source-only probe; never edit registry or engine graph."""
import argparse
import hashlib
import json
import shutil
import subprocess
import tarfile
from pathlib import Path

if not __debug__:
    raise RuntimeError('source admission requires assertions enabled')

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
ADDED = ('presentation.rs', 'presentation_native.rs', 'presentation_tests.rs')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(root):
    result = {}
    for p in sorted(root.rglob('*')):
        if p.is_symlink():
            raise ValueError('source symlink')
        if p.is_file():
            result[p.relative_to(root).as_posix()] = sha(p)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('crate_archive', type=Path)
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    archive = args.crate_archive.resolve()
    out = args.destination.resolve()
    assert not out.exists() and ROOT not in out.parents and out != ROOT
    pins = json.loads((HERE / 'pins.json').read_text())
    assert archive.stat().st_size < 2 * 1024 * 1024
    assert sha(archive) == pins['registry_checksum']
    # Verify members before creating output; no symlinks, special nodes or paths.
    with tarfile.open(archive, 'r:gz') as tar:
        members = tar.getmembers()
        assert len(members) < 1024 and sum(m.size for m in members) < 20 * 1024 * 1024
        prefix = f"{pins['package']}-{pins['version']}/"
        for member in members:
            assert member.isfile() and member.name.startswith(prefix)
            rel = Path(member.name[len(prefix):])
            assert rel.parts and not rel.is_absolute() and all(x not in ('.', '..') for x in rel.parts)
        out.mkdir()
        copied = out / 'wgpu-hal'
        copied.mkdir()
        for member in members:
            p = copied / member.name[len(prefix):]
            p.parent.mkdir(parents=True, exist_ok=True)
            with p.open('xb') as f:
                f.write(tar.extractfile(member).read())
    before = inventory(copied)
    for path, expected in pins['preimages'].items():
        assert before[path] == expected, path
    r = subprocess.run(['patch', '--batch', '--fuzz=0', '-p1', '-i', str(HERE / 'wgpu-hal.patch')], cwd=copied, capture_output=True, timeout=10, check=True)
    (out / 'patch.stdout').write_bytes(r.stdout)
    (out / 'patch.stderr').write_bytes(r.stderr)
    for name in ADDED:
        target = copied / 'src/metal' / name
        assert not target.exists()
        shutil.copyfile(HERE / name, target)
    after = inventory(copied)
    assert sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k)) == sorted([*pins['preimages'], *('src/metal/' + n for n in ADDED)])
    (out / 'src').mkdir()
    shutil.copyfile(HERE / 'host.rs', out / 'src/lib.rs')
    shutil.copyfile(ROOT / 'Cargo.lock', out / 'Cargo.lock')
    shutil.copyfile(ROOT / 'Cargo.lock', out / 'engine-Cargo.lock')
    (out / 'Cargo.toml').write_text('''[package]
name = "incant-native-presentation-observer-probe"
version = "0.1.0"
# Match the injected HAL's edition for the exact collector CPU model too.
edition = "2021"
[workspace]
[features]
default = []
observer = ["wgpu-hal/diagnostic-presentations"]
[dependencies]
wgpu = { version = "=29.0.4", default-features = false, features = ["metal", "std", "wgsl"] }
wgpu-hal = { version = "=29.0.4", default-features = false, features = ["metal"] }
parking_lot = "=0.12.5"
[patch.crates-io]
wgpu-hal = { path = "wgpu-hal" }
[profile.ci]
inherits = "release"
opt-level = 1
''')
    record = {'diagnostic_only': True, 'built': False, 'root_lock_sha256': sha(ROOT / 'Cargo.lock'),
              'archive_sha256': sha(archive), 'upstream': before, 'patched': after,
              'owned_sources': inventory(HERE), 'root': str(ROOT), 'prepared': str(out)}
    (out / 'manifest.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({'prepared': str(out), 'source_only': True}))


if __name__ == '__main__':
    main()
