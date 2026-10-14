#!/usr/bin/env python3
"""Verify exact vendored upstream restoration and additive ABI source inventory.

Read-only source audit; never builds or runs a VM. Native parity/limits and target
compilation remain separate required evidence. Does not download dependencies.
"""
import hashlib
import json
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[3]
ARCHIVES = {
    'rquickjs-core': '6657979a24d543e5fc66f32cef2f47ca82ce8eb55cec5f7d59a12b7b570bbf24',
    'rquickjs-sys': 'cee271d0eeba64f0915b846cb7ae02e16faf3dfdffdca91731101d9d30fe3423',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def restore(current, patch):
    """Reverse exact unified hunks without fuzzy offset matching."""
    lines = patch.splitlines(keepends=True)
    result, i = {}, 0
    while i < len(lines):
        assert lines[i].startswith('--- a/')
        name = lines[i][6:].strip()
        assert name in current and name not in result
        i += 1
        assert lines[i] == '+++ b/' + name + '\n'
        i += 1
        after = current[name].decode().splitlines(keepends=True)
        before, cursor = [], 0
        while i < len(lines) and not lines[i].startswith('--- '):
            match = re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@.*\n', lines[i])
            assert match, lines[i]
            start = int(match[3]) - 1
            assert start >= cursor
            before.extend(after[cursor:start]); cursor = start; i += 1
            consumed = produced = 0
            while i < len(lines) and not lines[i].startswith(('@@ ', '--- ')):
                sign, text = lines[i][0], lines[i][1:]
                assert sign in ' +-'
                if sign in ' +':
                    assert after[cursor] == text, name
                    cursor += 1; consumed += 1
                if sign in ' -':
                    before.append(text); produced += 1
                i += 1
            assert consumed == int(match[4] or 1) and produced == int(match[2] or 1)
        before.extend(after[cursor:])
        result[name] = ''.join(before).encode()
    return result


def audit(root=ROOT):
    total = changed = 0
    for name, archive in ARCHIVES.items():
        directory = root / 'third_party' / name
        metadata = json.loads((directory / 'INCANT_UPSTREAM.json').read_text())
        assert metadata['archive_sha256'] == archive
        assert metadata['git'] == 'd7ef5eeae702fea24c03643064de454f1c1dd4b0'
        assert metadata['version'] == '0.14.0' and metadata['crate'] == name
        current = {p.relative_to(directory).as_posix(): p.read_bytes()
                   for p in directory.rglob('*') if p.is_file()}
        assert set(current) == set(metadata['files']) | {
            'LICENSE', 'INCANT_UPSTREAM.json', 'INCANT_PATCH.diff', 'INCANT_PATCH.md'}
        assert sha(current['LICENSE']) == metadata['license_sha256']
        original = restore(current, current['INCANT_PATCH.diff'].decode())
        patches = {row['path']: row for row in metadata['patched_files']}
        assert len(patches) == len(metadata['patched_files']) and set(original) == set(patches)
        for path, expected in metadata['files'].items():
            assert sha(original.get(path, current[path])) == expected, (name, path, 'upstream')
            if path in patches:
                assert patches[path]['upstream_sha256'] == expected
                assert sha(current[path]) == patches[path]['current_sha256'], (name, path, 'current')
        total += len(metadata['files']); changed += len(patches)
    bindings = sorted((root / 'third_party/rquickjs-sys/src/bindings').glob('*.rs'))
    assert len(bindings) == 20
    declaration = '''pub fn JS_GetMallocState(
        rt: *mut JSRuntime,
        allocated_bytes: *mut size_t,
        allocation_count: *mut size_t,
        limit_bytes: *mut size_t,
    );'''
    for path in bindings:
        text = path.read_text()
        assert text.count(declaration) == 1 and text.count('pub fn JS_GetMallocState') == 1, path
        assert 'pub type size_t = ' in text
    packages = tomllib.loads((root / 'Cargo.lock').read_text())['package']
    manifest = tomllib.loads((root / 'Cargo.toml').read_text())
    for name in ARCHIVES:
        entries = [p for p in packages if p['name'] == name]
        assert len(entries) == 1 and entries[0]['version'] == '0.14.0'
        assert 'source' not in entries[0] and 'checksum' not in entries[0]
        assert manifest['patch']['crates-io'][name] == {'path': 'third_party/' + name}
    return {'upstream_files': total, 'patched_files': changed, 'bundled_abis': len(bindings),
            'execution': False, 'performance_claim': False}


if __name__ == '__main__':
    print(json.dumps(audit(), sort_keys=True))
