"""Stage-bound inspection of the nonshipping arm64 generation image."""
from pathlib import Path
import hashlib
import os
import re
import stat
import struct

from native_package import require

PROFILE_SECTIONS = ('__llvm_prf_cnts', '__llvm_prf_data', '__llvm_prf_vals', '__llvm_prf_vnds',
                    '__llvm_prf_names', '__llvm_prf_bits', '__llvm_prf_vns', '__llvm_prf_vtab')
PROFILE_APIS = ('___llvm_profile_write_file', '___llvm_profile_initialize', '___llvm_profile_dump')


def image(path):
    """Read the compiler's possibly hardlinked image; never execute or extract it."""
    path = Path(path)
    require(path.is_absolute() and not any(p.is_symlink() for p in (path, *path.parents)),
            'Unredirected absolute generation image')
    before = path.stat(follow_symlinks=False)
    require(stat.S_ISREG(before.st_mode) and 32 <= before.st_size <= 256 * 1024**2,
            'Bounded regular generation image')
    sections, uuids = {}, []
    descriptor = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0))
    with os.fdopen(descriptor, 'rb') as stream:
        actual = os.fstat(stream.fileno())
        require((actual.st_dev, actual.st_ino) == (before.st_dev, before.st_ino), 'Generation image replaced')
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        stream.seek(0)
        header = struct.unpack('<8I', stream.read(32))
        require(header[:4] == (0xfeedfacf, 0x100000c, 0, 2)
                and 1 <= header[4] <= 256 and header[5] <= 1024**2, 'Expected bounded arm64 executable')
        commands = stream.read(header[5])
        require(len(commands) == header[5], 'Truncated image load commands')
        offset = 0
        for _ in range(header[4]):
            require(offset + 8 <= len(commands), 'Truncated image load command')
            command, size = struct.unpack_from('<II', commands, offset)
            require(size >= 8 and offset + size <= len(commands), 'Bounded image load command')
            if command == 0x1b:
                require(size == 24, 'Exact image UUID command')
                uuids.append(commands[offset + 8:offset + 24].hex())
            elif command == 0x19:
                require(size >= 72, 'Truncated image segment')
                segment = struct.unpack_from('<II16sQQQQiiII', commands, offset)
                require(segment[9] <= 256 and size == 72 + 80 * segment[9], 'Bounded image sections')
                for index in range(segment[9]):
                    row = struct.unpack_from('<16s16sQQIIIIIIII', commands, offset + 72 + 80 * index)
                    name, owner = (v.rstrip(b'\0').decode('ascii') for v in row[:2])
                    if name not in (*PROFILE_SECTIONS, '__text', '__mod_init_func'):
                        continue
                    require(name not in sections and row[3] <= 64 * 1024**2, 'Unique bounded profile/text section')
                    if (row[8] & 255) in (1, 12, 18):
                        payload = 'zero-fill'
                    else:
                        require(row[4] + row[3] <= actual.st_size, 'Section outside generation image')
                        stream.seek(row[4]); data = stream.read(row[3])
                        require(len(data) == row[3], 'Truncated generation section')
                        payload = hashlib.sha256(data).hexdigest()
                    sections[name] = {'segment': owner, 'address': row[2], 'bytes': row[3],
                                      'flags': row[8], 'content_sha256_or_zerofill': payload}
            offset += size
        require(offset == len(commands) and len(uuids) == 1
                and set(sections) == {*PROFILE_SECTIONS, '__text', '__mod_init_func'}
                and sections['__text']['segment'] == '__TEXT'
                and all(sections[name]['bytes'] > 0 for name in ('__text', '__mod_init_func',
                    '__llvm_prf_cnts', '__llvm_prf_data', '__llvm_prf_names')),
                'Complete generation image identity/sections')
        after = os.fstat(stream.fileno())
        require((actual.st_size, actual.st_mtime_ns) == (after.st_size, after.st_mtime_ns), 'Generation image changed')
    return {'path': str(path), 'bytes': actual.st_size, 'sha256': digest, 'uuid': uuids[0], 'sections': sections}


def before_strip(path, symbols):
    require(len(symbols.encode()) <= 8 * 1024**2, 'Bounded generation symbol evidence')
    native = image(path)
    text = native['sections']['__text']
    addresses = {}
    for name in PROFILE_APIS:
        matches = re.findall(r'(?m)^([0-9a-fA-F]+) [Tt] ' + re.escape(name) + r'$', symbols)
        require(len(matches) == 1, 'Actual pre-strip profile API required: ' + name)
        address = int(matches[0], 16)
        require(text['address'] <= address < text['address'] + text['bytes'], 'Profile API outside code section')
        addresses[name] = address
    require(re.search(r'(?m)^[0-9a-fA-F]+ [bBdDsS] ___llvm_profile_runtime$', symbols),
            'Actual pre-strip profile runtime data required')
    return {'stage': 'unstripped-generation', 'image': native, 'symbols': symbols, 'profile_API_addresses': addresses}


def after_strip(proof, packaged, symbols, section_output):
    require(type(proof) is dict and set(proof) == {'stage', 'image', 'symbols', 'profile_API_addresses'}
            and proof['stage'] == 'unstripped-generation' and packaged is not None,
            'Explicit pre-strip generation proof required')
    current = before_strip(proof['image']['path'], proof['symbols'])
    require(current == proof, 'Pre-strip artifact/symbol identity changed')
    final = image(packaged)
    require(final['path'] != current['image']['path'] and final['uuid'] == current['image']['uuid']
            and final['sections'] == current['image']['sections'], 'Packaged generation code/counters differ')
    require(all('Name: ' + name + ' ' in section_output for name in PROFILE_SECTIONS),
            'Actual packaged profile section inspection required')
    return {'symbol_stage': proof['stage'], 'section_stage': 'packaged-generation',
            'symbol_artifact': current['image'], 'packaged_artifact': final,
            'symbols_sha256': hashlib.sha256(proof['symbols'].encode()).hexdigest(),
            'packaged_symbols_sha256': hashlib.sha256(symbols.encode()).hexdigest(),
            'profile_API_addresses': proof['profile_API_addresses'], 'identical_code_initializer_and_profile_sections': True}
