#!/usr/bin/env python3
"""Prepare private world tests with the existing one-way native observation code.

No runtime or public test installer is added. The generated file is included only
by cfg(test) vendor unit tests; all original/copy hashes and mechanical changes
are recorded. Supply the printed path as INCANT_PRIVATE_WORLD_CAPTURE to Cargo.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
from prepare import prepare


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def prepare_world(output, root):
    prepare(output, root)
    original = root / 'crates/incant_physics_snapshot/tests/trusted_manifolds.rs'
    text = original.read_text()
    for dim, names in [('dim2', ['bits2', 'data2', 'vector2', 'box2', 'planar_bits']),
                       ('dim3', ['bits3', 'data3', 'vector3', 'box3', 'spatial_bits'])]:
        for name in names:
            old = 'fn ' + name + '('
            assert text.count(old) == 1
            text = text.replace(old, '#[cfg(feature = "' + dim + '")]\n' + old)
    for name, dim in [('planar', 'dim2'), ('spatial', 'dim3')]:
        old = 'cases!(\n    ' + name + ','
        assert text.count(old) == 1
        text = text.replace(old, '#[cfg(feature = "' + dim + '")]\n' + old)
    text = text.replace('rapier2d::', 'crate::').replace('rapier3d::', 'crate::')
    text = text.replace('    rapier2d,', '    native,').replace('    rapier3d,', '    native,')
    text = text.replace('#[cfg(feature = "native-admission")]', '#[cfg(feature = "incant-checked-restore")]')
    for name in ('island_fixture', 'narrow_fixture'):
        old = f'include!("../../../third_party/rapier2d/src/pipeline/checked_restore/{name}.rs");'
        assert text.count(old) == 1
        text = text.replace(old, 'include!(' + json.dumps(str(root / f'third_party/rapier3d/src/pipeline/checked_restore/{name}.rs')) + ');')
    old = '            world_cases!($rapier, $vector, $cuboid);'
    assert text.count(old) == 1
    body = root / 'third_party/rapier3d/src/pipeline/checked_restore/world_cases.rs'
    text = text.replace(old, old + '\n            use $vector as world_vector;\n            use $cuboid as world_cuboid;\n            include!(' + json.dumps(str(body)) + ');')
    # Strip inner documentation only because include! is in an existing test module.
    text = '\n'.join('//'+line[3:] if line.startswith('//!') else line for line in text.splitlines()) + '\n'
    text = 'use crate as native;\n#[allow(unused_imports)] use std::{vec::Vec,string::{String,ToString},vec,eprintln,println,format};\n' + text
    generated = output / 'capture.rs'
    generated.write_text(text)
    support = root / 'crates/incant_physics_snapshot/tests/support'
    shutil.copytree(support, output / 'support')
    rows = [{'source': str(original), 'copy': str(generated), 'source_sha256': digest(original), 'copy_sha256': digest(generated), 'mechanical_test_adaptation': True}]
    for p in sorted(support.glob('*.rs')):
        q = output / 'support' / p.name
        lines=q.read_text().splitlines();i=0
        while i<len(lines) and lines[i].startswith('//!'): i+=1
        lines.insert(i, '#[allow(unused_imports)] use std::{vec::Vec,string::{String,ToString},vec,eprintln,println,format};')
        q.write_text('\n'.join(lines)+'\n')
        rows.append({'source': str(p), 'copy': str(q), 'source_sha256': digest(p), 'copy_sha256': digest(q), 'mechanical_test_adaptation': True})
    for p in [body, *(root / 'third_party/rapier3d/src/pipeline/checked_restore').glob('*_fixture.rs'), *(root / 'third_party/rapier3d/src/pipeline/checked_restore/world').glob('*.rs')]:
        rows.append({'source': str(p), 'source_sha256': digest(p), 'included_unchanged': True})
    (output / 'capture-sources.json').write_text(json.dumps({'scope': __doc__, 'sources': rows}, indent=2) + '\n')
    return generated


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(prepare_world(args.output.resolve(), Path(__file__).resolve().parents[3]))
