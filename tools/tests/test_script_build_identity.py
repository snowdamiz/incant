"""Source-list/byte-sensitivity controls; no compiler or engine is executed."""
import hashlib
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'crates/incant_script'


def identity_inputs():
    source = (SCRIPT / 'build.rs').read_text()
    # Bind the explicit conservative input loop and its length+byte framing.
    # Vendor/toolchain identity precedes this suffix and is not reconstructed.
    start = source.index('    for path in [')
    end = source.index('    println!(\n        "cargo:rustc-env=INCANT_SCRIPT_BUILD_ID=', start)
    loop = source[start:end]
    framing = ('identity.update((bytes.len() as u64).to_le_bytes());\n'
               '        identity.update(bytes);')
    if framing not in loop:
        raise AssertionError('Re-audit conservative identity framing')
    items = loop.split('    ] {', 1)[0].splitlines()[1:]
    result = []
    for item in items:
        match = re.fullmatch(r'        (root\.join|Path::new)\("([^"]+)"\)(?:\.into\(\))?,', item)
        if match is None:
            raise AssertionError(f'Re-audit identity input: {item}')
        result.append((ROOT if match[1] == 'root.join' else SCRIPT) / match[2])
    if len(result) != len(set(result)):
        raise AssertionError('Duplicate conservative identity input')
    return result


def suffix_digest(inputs, replacement=None):
    digest = hashlib.sha256(b'fixed preceding vendor identity for source-list control')
    for path in inputs:
        data = replacement[1] if replacement and path == replacement[0] else path.read_bytes()
        digest.update(len(data).to_bytes(8, 'little'))
        digest.update(data)
    return digest.hexdigest()


class ScriptBuildIdentityTests(unittest.TestCase):
    def test_enforcement_sources_are_actual_unique_compatibility_inputs(self):
        inputs = identity_inputs()
        for relative in ('src/budget.rs', 'src/diagnostics.rs'):
            self.assertIn(SCRIPT / relative, inputs)

    def test_changed_enforcement_bytes_change_framed_identity_suffix(self):
        inputs = identity_inputs()
        original = suffix_digest(inputs)
        self.assertEqual(original, suffix_digest(inputs))
        for relative in ('src/budget.rs', 'src/diagnostics.rs'):
            path = SCRIPT / relative
            data = path.read_bytes()
            changed = bytes([data[0] ^ 1]) + data[1:]
            self.assertEqual(len(changed), len(data))
            with self.subTest(input=relative):
                self.assertNotEqual(original, suffix_digest(inputs, (path, changed)))


if __name__ == '__main__':
    unittest.main()
