"""Bounded immutable routine metadata; never execute guest code at build time."""
import re
import struct
from wasm_persistent import custom_section

SECTION = b'incant_coroutines_v1'


def catalog(data):
    value = custom_section(data, SECTION)
    if value is None:
        return []
    if (len(value) < 16 or value[:8] != b'INCCAT01'
            or struct.unpack_from('<I', value, 8)[0] != 1):
        raise ValueError('Invalid coroutine catalog')
    count = struct.unpack_from('<I', value, 12)[0]
    if not 1 <= count <= 128 or len(value) != 16 + 104 * count:
        raise ValueError('Invalid coroutine catalog count')
    result = []
    names = set()
    for offset in range(16, len(value), 104):
        row = value[offset:offset + 104]
        length, size = struct.unpack_from('<II', row, 64)
        name = row[:length]
        if (not 1 <= length <= 64 or any(row[length:64])
                or not re.fullmatch(rb'[A-Za-z0-9_.:-]+', name)
                or name in names or not 1 <= size <= 16384 or not any(row[72:])):
            raise ValueError('Invalid or duplicate coroutine routine')
        names.add(name)
        result.append({'name': name.decode('ascii'), 'bytes': size,
                       'schema_sha256': row[72:].hex()})
    return result


def linked_suffix(row, output):
    original = catalog((output / f'{row["prefix"]}.original.wasm').read_bytes())
    if original != row.get('coroutines', []):
        raise ValueError('Original coroutine catalog mismatch')
    if catalog((output / f'{row["prefix"]}.wasm').read_bytes()) != original:
        raise ValueError('Prepared coroutine catalog mismatch')
    if not original:
        return ''
    routines = []
    for item in original:
        schema = ','.join(str(b) for b in bytes.fromhex(item['schema_sha256']))
        routines.append('incant_wasm_abi::coroutines::Routine::new('
                        f'"{item["name"]}",{item["bytes"]},[{schema}])'
                        '.expect("validated coroutine routine")')
    return '.with_coroutines(&[' + ','.join(routines) + '])'
