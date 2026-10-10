"""Platform crash-symbol packaging. Inputs are trusted, locally built artifacts.

PDB/MSF layouts: https://llvm.org/docs/PDB/MsfFile.html and PdbStream.html.
PE debug directory: https://learn.microsoft.com/windows/win32/debug/pe-format.
Unknown formats fail closed instead of producing an unverified symbol archive.
"""
from pathlib import Path
import re
import shutil
import struct
import subprocess
import uuid
import zlib


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(command):
    try:
        return subprocess.check_output([str(arg) for arg in command], text=True, stderr=subprocess.STDOUT)
    except subprocess.CalledProcessError as error:
        raise ValueError(f'{Path(command[0]).name} failed: {error.output[-4000:]}') from error


def backend(target):
    arch = target.split('-')[0]
    require(arch in ('aarch64', 'x86_64'), f'Unsupported symbol-packaging target: {target}')
    for ending, kind in (('-apple-darwin', 'dsym'), ('-unknown-linux-gnu', 'dwarf'), ('-pc-windows-msvc', 'pdb')):
        if target == arch + ending:
            return kind
    raise ValueError(f'Unsupported symbol-packaging target: {target}; the shipping Cargo profile remains usable separately')


def unpack(data, offset, fmt):
    require(offset >= 0 and offset + struct.calcsize(fmt) <= len(data), 'Truncated symbol metadata')
    return struct.unpack_from(fmt, data, offset)


def pe_identity(data):
    require(data[:2] == b'MZ', 'Not a PE executable')
    pe, = unpack(data, 0x3c, '<I')
    require(data[pe:pe + 4] == b'PE\0\0', 'Missing PE signature')
    sections, = unpack(data, pe + 6, '<H')
    optional_size, = unpack(data, pe + 20, '<H')
    optional = pe + 24
    magic, = unpack(data, optional, '<H')
    require(magic in (0x10b, 0x20b), 'Unsupported PE optional header')
    directories = optional + (112 if magic == 0x20b else 96)
    require(directories + 7 * 8 <= optional + optional_size, 'Missing PE debug directory')
    debug_rva, debug_size = unpack(data, directories + 6 * 8, '<II')

    def file_offset(rva, size):
        for index in range(sections):
            section = optional + optional_size + index * 40
            _, start, raw_size, raw = unpack(data, section + 8, '<IIII')
            if start <= rva and rva + size <= start + raw_size:
                require(raw + rva - start + size <= len(data), 'Truncated PE section')
                return raw + rva - start
        raise ValueError('PE debug directory is outside file-backed sections')

    require(debug_size > 0 and debug_size % 28 == 0, 'Missing or invalid PE debug directory')
    offset = file_offset(debug_rva, debug_size)
    identities = []
    for entry in range(offset, offset + debug_size, 28):
        kind, size, _, raw = unpack(data, entry + 12, '<IIII')
        if kind != 2:
            continue
        require(size >= 25 and raw + size <= len(data), 'Truncated CodeView record')
        require(data[raw:raw + 4] == b'RSDS', 'Unsupported CodeView record')
        age, = unpack(data, raw + 20, '<I')
        identities.append({'guid': str(uuid.UUID(bytes_le=bytes(data[raw + 4:raw + 20]))), 'age': age})
    require(len(identities) == 1, 'Expected exactly one PE/PDB identity')
    return identities[0]


def pdb_identity(data):
    require(data[:32] == b'Microsoft C/C++ MSF 7.00\r\n\x1aDS\0\0\0', 'Unsupported PDB/MSF format')
    block_size, _, block_count, directory_size, _, block_map = unpack(data, 32, '<6I')
    require(block_size in (512, 1024, 2048, 4096, 8192, 16384, 32768), 'Unsupported MSF block size')
    require(block_count * block_size == len(data), 'Truncated MSF block storage')

    def blocks(indices, size):
        require(all(0 <= index < block_count for index in indices), 'MSF block index out of range')
        return b''.join(data[index * block_size:(index + 1) * block_size] for index in indices)[:size]

    def block_indices(offset, size):
        count = (size + block_size - 1) // block_size
        return unpack(data, offset, f'<{count}I')

    require(directory_size > 0 and directory_size <= block_size * (block_size // 4), 'Unsupported MSF directory size')
    require(block_map < block_count, 'MSF directory block map out of range')
    directory = blocks(block_indices(block_map * block_size, directory_size), directory_size)
    stream_count, = unpack(directory, 0, '<I')
    require(4 <= stream_count <= len(directory) // 4, 'Invalid PDB stream count')
    sizes = unpack(directory, 4, f'<{stream_count}I')
    cursor = 4 + stream_count * 4
    streams = []
    for size in sizes:
        if size == 0xffffffff:
            streams.append(b'')
            continue
        count = (size + block_size - 1) // block_size
        indices = unpack(directory, cursor, f'<{count}I')
        streams.append(blocks(indices, size))
        cursor += count * 4
    info = streams[1]
    _, _, age = unpack(info, 0, '<III')
    require(len(info) >= 28, 'Missing PDB GUID')
    # DBI must contain module, source-file and symbol-record information. A GUID
    # alone is not proof that this is a useful crash-symbol artifact.
    dbi = streams[3]
    signature, version, dbi_age = unpack(dbi, 0, '<iII')
    symbol_stream, = unpack(dbi, 20, '<H')
    modules, _, _, files = unpack(dbi, 24, '<4i')
    require(signature == -1 and version == 19990903 and dbi_age == age, 'Unsupported or inconsistent PDB DBI header')
    require(modules > 0 and files > 0 and len(dbi) >= 64 + modules + files, 'PDB has no module/source-file debug information')
    require(symbol_stream < len(streams) and len(streams[symbol_stream]) > 0, 'PDB has no symbol records')
    return {'guid': str(uuid.UUID(bytes_le=info[12:28])), 'age': age}


def mach_uuids(text):
    entries = re.findall(r'UUID: ([0-9A-Fa-f-]{36}) \(([^)]+)\)', text)
    require(entries and len({arch for _, arch in entries}) == len(entries), 'Missing or duplicate Mach-O UUIDs')
    return {arch: value.lower() for value, arch in entries}


def elf_build_id(text):
    ids = re.findall(r'Build ID: ([0-9a-fA-F]+)', text)
    require(len(ids) == 1, 'Expected exactly one ELF build ID')
    return ids[0].lower()


def has_section(text, name):
    return re.search(r'Name: ' + re.escape(name) + r'(?:\s|\()', text) is not None


def section_size(text, name):
    selected = False
    for line in text.splitlines():
        if line.strip().startswith('Name:'):
            selected = has_section(line, name)
        elif selected and line.strip().startswith('Size:'):
            return int(line.split(':', 1)[1].strip(), 0)
    return 0


def package(executable, output, kind, llvm_bin):
    player_dir, symbol_dir = output / 'player', output / 'symbols'
    player_dir.mkdir()
    symbol_dir.mkdir()
    player = player_dir / executable.name
    shutil.copy2(executable, player)
    suffix = '.exe' if kind == 'pdb' else ''
    readobj = llvm_bin / f'llvm-readobj{suffix}'
    require(readobj.is_file(), f'Missing Rust llvm-tools component: {readobj}')

    if kind == 'dsym':
        source = executable.with_name(executable.name + '.dSYM')
        require(source.is_dir(), f'Compiler did not produce a packed dSYM: {source}')
        symbols = symbol_dir / source.name
        shutil.copytree(source, symbols)
        # Cargo renames the bundle but preserves rustc's hash suffix inside it.
        dwarf_files = list((symbols / 'Contents/Resources/DWARF').iterdir())
        require(len(dwarf_files) == 1 and dwarf_files[0].is_file(), 'Expected one linked DWARF image in the dSYM')
        dwarf = dwarf_files[0]
        sections = run([readobj, '--sections', dwarf])
        require(section_size(sections, '__debug_info') > 0 and section_size(sections, '__debug_line') > 0, 'dSYM lacks DWARF info/line sections')
        run(['xcrun', 'dwarfdump', '--verify', symbols])
        identity = mach_uuids(run(['xcrun', 'dwarfdump', '--uuid', executable]))
        require(identity == mach_uuids(run(['xcrun', 'dwarfdump', '--uuid', symbols])), 'dSYM UUID mismatch')
        run(['xcrun', 'strip', '-S', '-x', player])
        require(identity == mach_uuids(run(['xcrun', 'dwarfdump', '--uuid', player])), 'Stripping changed Mach-O UUID')
        require('__debug_info' not in run([readobj, '--sections', player]), 'Player still contains DWARF information')
        return player, {'format': 'Mach-O/dSYM', 'uuids': identity}

    if kind == 'dwarf':
        objcopy = llvm_bin / 'llvm-objcopy'
        symbols = symbol_dir / (executable.name + '.debug')
        run([objcopy, '--only-keep-debug', executable, symbols])
        run([llvm_bin / 'llvm-strip', '--strip-all', player])
        run([objcopy, f'--add-gnu-debuglink={symbols}', player])
        identity = elf_build_id(run([readobj, '--notes', player]))
        require(identity == elf_build_id(run([readobj, '--notes', symbols])), 'ELF build-ID mismatch')
        sections = run([readobj, '--sections', symbols])
        require(section_size(sections, '.debug_info') > 0 and section_size(sections, '.debug_line') > 0, 'Symbols lack DWARF info/line sections')
        require(not has_section(run([readobj, '--sections', player]), '.debug_info'), 'Player still contains DWARF information')
        link = output / 'debuglink.bin'
        run([objcopy, f'--dump-section=.gnu_debuglink={link}', player])
        payload = link.read_bytes()
        name_end = payload.index(0)
        require(payload[:name_end].decode() == symbols.name, 'Incorrect GNU debuglink filename')
        crc, = unpack(payload, (name_end + 4) & ~3, '<I')
        require(crc == zlib.crc32(symbols.read_bytes()), 'GNU debuglink CRC mismatch')
        link.unlink()
        return player, {'format': 'ELF/DWARF', 'build_id': identity, 'debuglink_crc32': f'{crc:08x}'}

    require(kind == 'pdb', f'Unsupported symbol format: {kind}')
    source = executable.with_suffix('.pdb')
    require(source.is_file(), f'Compiler did not produce a PDB: {source}')
    symbols = symbol_dir / source.name
    shutil.copy2(source, symbols)
    identity = pe_identity(player.read_bytes())
    require(identity == pdb_identity(symbols.read_bytes()), 'PE/PDB GUID or age mismatch')
    require(not has_section(run([readobj, '--sections', player]), '.debug_info'), 'Player contains unexpected DWARF information')
    return player, {'format': 'PE/PDB', **identity}
