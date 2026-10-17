"""Data-only opt-in metadata and trusted build-linked state identity.

This parser never validates or executes a module. The ordinary cook must already
validate raw input and regenerate metering before translation.
"""
import hashlib
import struct

SECTION = b'incant_state_v1'
MAGIC = b'INCWST01'
MAX_BYTES = 65_536


def custom_section(data, wanted):
    if len(data) > 1024 * 1024 or data[:8] != b'\0asm\x01\0\0\0':
        raise ValueError('Persistent declaration requires core Wasm <= 1 MiB')
    cursor = 8

    def unsigned(end):
        nonlocal cursor
        value = 0
        for index in range(5):
            if cursor >= end:
                raise ValueError('Truncated Wasm length')
            byte = data[cursor]
            cursor += 1
            if index == 4 and byte > 15:
                raise ValueError('Wasm length overflow')
            value |= (byte & 127) << (index * 7)
            if byte < 128:
                return value
        raise ValueError('Wasm length overflow')

    found = None
    while cursor < len(data):
        section = data[cursor]
        cursor += 1
        length = unsigned(len(data))
        end = cursor + length
        if end > len(data):
            raise ValueError('Truncated Wasm section')
        if section == 0:
            length = unsigned(end)
            name = data[cursor:cursor + length]
            cursor += length
            if cursor > end:
                raise ValueError('Truncated custom section name')
            if name == wanted:
                if found is not None:
                    raise ValueError('Duplicate persistent declaration')
                found = data[cursor:end]
        cursor = end
    return found


def declaration(data):
    value = custom_section(data, SECTION)
    if value is None:
        return None
    if (len(value) != 56 or value[:8] != MAGIC
            or struct.unpack_from('<I', value, 8)[0] != 1
            or not 1 <= struct.unpack_from('<I', value, 12)[0] <= MAX_BYTES
            or not any(value[16:48]) or any(value[48:])):
        raise ValueError('Invalid persistent declaration')
    return {'bytes': struct.unpack_from('<I', value, 12)[0],
            'schema_sha256': value[16:48].hex()}


def linked_expression(row, output, root):
    from wasm_coroutines import linked_suffix
    suffix = linked_suffix(row, output)
    module = row['prefix']
    arguments = f'incant_{module}_create, incant_{module}_update, incant_{module}_free'
    state = row['persistent_state']
    if declaration((output / f'{module}.wasm').read_bytes()) != state:
        raise ValueError('Prepared persistent declaration mismatch')
    if state is None:
        return f'ModuleExports::from_linked({arguments})' + suffix
    digest = hashlib.sha256(b'Incant trusted wasm2c persistent program v1\0WABT 1.0.42\0')
    # Bind original input, actual generated units, policy, registry and facade.
    # The session separately binds the current runtime, backend and target.
    for path in [output / f'{module}{suffix}' for suffix in
                 ('.original.wasm', '.wasm', '.c', '.h', '-facade.c')] + [
                     root / path for path in ('sdk/abi/native-v1.json',
                         'crates/incant_wasm_cook/src/lib.rs',
                         'crates/incant_wasm_cook/src/captured.rs',
                         'tools/wasm2c.py', 'tools/wasm_persistent.py',
                         'tools/wasm_coroutines.py')]:
        value = path.read_bytes()
        digest.update(len(value).to_bytes(8, 'little'))
        digest.update(value)
    fingerprint = digest.digest()
    row['persistent_fingerprint'] = fingerprint.hex()
    def rust_array(value):
        return '[' + ','.join(str(byte) for byte in value) + ']'
    return (f'ModuleExports::from_linked_persistent({arguments}, '
            'crate::persistent::PersistentProgram { declaration: '
            f'crate::persistent::Declaration::new({state["bytes"]}, '
            f'{rust_array(bytes.fromhex(state["schema_sha256"]))}).expect("validated state declaration"), '
            f'fingerprint: {rust_array(fingerprint)} }})') + suffix
