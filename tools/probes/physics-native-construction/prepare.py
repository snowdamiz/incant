#!/usr/bin/env python3
"""Prepare exact vendor copies for private in-crate tests; does not run Cargo.

Cargo excludes the vendored dependency packages from the root workspace. This
keeps constructors crate-private without adding a public testing installer.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(output, root):
    output.mkdir(parents=True, exist_ok=False)
    rows = []
    for dimension in ("2d", "3d"):
        name = "rapier" + dimension
        source = root / "third_party" / name
        destination = output / name
        shutil.copytree(source, destination)
        manifest = destination / "Cargo.toml"
        old = 'path = "../../crates/incant_physics_snapshot"'
        text = manifest.read_text()
        if text.count(old) != 1:
            raise ValueError("expected exactly one optional snapshot dependency")
        manifest.write_text(text.replace(old, "path = " + json.dumps(str(root / "crates/incant_physics_snapshot"))))
        for path in sorted(source.rglob("*")):
            if path.is_symlink():
                raise ValueError("vendor source symlinks are not admitted")
            if path.is_file():
                copy = destination / path.relative_to(source)
                rows.append({"source": str(path), "copy": str(copy), "source_sha256": digest(path), "copy_sha256": digest(copy), "manifest_path_only": path.name == "Cargo.toml"})
    q = lambda path: json.dumps(str(path))
    (output / "Cargo.toml").write_text('''[package]
name = "incant-private-construction-native-tests"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
resolver = "3"
members = ["rapier2d", "rapier3d"]
[lib]
path = "lib.rs"
[dependencies]
rapier2d = { path = "rapier2d", features = ["incant-checked-restore", "enhanced-determinism"] }
rapier3d = { path = "rapier3d", features = ["incant-checked-restore", "enhanced-determinism"] }
[patch.crates-io]
parry2d = { path = ''' + q(root / "third_party/parry2d") + ''' }
parry3d = { path = ''' + q(root / "third_party/parry3d") + ''' }
rapier2d = { path = "rapier2d" }
rapier3d = { path = "rapier3d" }
[profile.ci]
inherits = "dev"
opt-level = 1
debug = 0
''')
    (output / "lib.rs").write_text("// Native unit-test host only.\n")
    shutil.copy2(root / "Cargo.lock", output / "Cargo.lock")
    (output / "source-copies.json").write_text(json.dumps({"root_lock_sha256": digest(root / "Cargo.lock"), "sources": rows}, indent=2) + "\n")
    return len(rows)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(prepare(args.output.resolve(), Path(__file__).resolve().parents[3]))
