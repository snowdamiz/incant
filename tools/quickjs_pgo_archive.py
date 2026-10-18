"""Bounded inspection of rustc's bundled native profile runtime; no extraction."""
from pathlib import Path
import hashlib
import re
import struct

from native_package import read, require


def bundled_payload(data):
    """Read only the observed arm64 Mach-O .bundled_lib object container."""
    require(32 <= len(data) <= 64 * 1024**2, 'Bounded bundled native object')
    header = struct.unpack_from('<8I', data)
    require(header[:4] == (0xfeedfacf, 0x100000c, 0, 1)
            and 1 <= header[4] <= 16 and header[5] <= 4096,
            'Expected arm64 Mach-O bundled native object')
    end = 32 + header[5]
    require(end <= len(data), 'Truncated bundled load commands')
    offset, sections = 32, []
    for _ in range(header[4]):
        require(offset + 8 <= end, 'Truncated bundled load command')
        command, size = struct.unpack_from('<II', data, offset)
        require(size >= 8 and size % 8 == 0 and offset + size <= end,
                'Bounded bundled load command')
        if command == 0x19:  # LC_SEGMENT_64, one data-only bundled archive section.
            require(size == 152, 'One bundled native section required')
            segment = struct.unpack_from('<II16sQQQQiiII', data, offset)
            require(segment[2] == bytes(16) and segment[9:] == (1, 0),
                    'Expected bundled native segment')
            section = struct.unpack_from('<16s16sQQIIIIIIII', data, offset + 72)
            require(section[0].rstrip(b'\0') == b'.bundled_lib'
                    and section[1].rstrip(b'\0') == b'__DWARF'
                    and section[2] == 0 and section[4] == end
                    and segment[3:7] == (0, section[3], end, section[3]),
                    'Expected exact bundled archive section')
            sections.append(section)
        else:
            require((command, size) in ((0x32, 24), (0x2, 24), (0xb, 80)),
                    'Unknown bundled native load command')
        offset += size
    require(offset == end and len(sections) == 1, 'Unique bundled archive section')
    length = sections[0][3]
    require(0 < length <= 64 * 1024**2 and end + length <= len(data)
            and len(data) - end - length <= 15 and not any(data[end + length:]),
            'Exact bounded archive payload and padding')
    return data[end:end + length]


def archive_member(data, wanted):
    """Inspect bounded BSD ar members, including rustc's padded extended names."""
    require(data.startswith(b'!<arch>\n'), 'Expected regular runtime rlib archive')
    offset, members, found = 8, 0, []
    while offset < len(data):
        require(offset + 60 <= len(data) and members < 128, 'Bounded runtime rlib members')
        header = data[offset:offset + 60]
        require(header[58:] == b'`\n' and re.fullmatch(rb' *[0-9]+ *', header[48:58]),
                'Invalid runtime rlib member header')
        size = int(header[48:58])
        start, end = offset + 60, offset + 60 + size
        require(end + size % 2 <= len(data), 'Truncated runtime rlib member')
        name = header[:16].decode('ascii').strip()
        payload = data[start:end]
        if name.startswith('#1/'):
            require(re.fullmatch(r'#1/[0-9]{1,3}', name), 'Bounded extended member name')
            count = int(name[3:])
            require(0 < count <= min(256, len(payload)), 'Truncated extended member name')
            name = payload[:count].rstrip(b'\0').decode('ascii')
            payload = payload[count:]
        else:
            name = name.removesuffix('/')
        require(name and '/' not in name and '\0' not in name, 'Unexpected runtime rlib member name')
        if name == wanted:
            found.append(payload)
        offset, members = end + size % 2, members + 1
    require(offset == len(data) and len(found) == 1, 'Unique bundled profile archive member')
    return found[0]


def runtime_archive(messages, runtime, target, root, triple):
    """Bind the archive payload to Cargo's exact target-role rquickjs rlib."""
    require(root is not None and type(runtime) is dict, 'Explicit runtime artifact owner and pinned identity')
    target, root = Path(target), Path(root)
    require(target.is_absolute() and target.resolve(strict=True) == target,
            'Canonical owned runtime target')
    require(set(runtime) == {'path', 'bytes', 'sha256'}
            and Path(runtime['path']).name == 'libclang_rt.profile_osx.a', 'Pinned profile runtime identity')
    original = read(runtime['path'], 64 * 1024**2)
    require(len(original) == runtime['bytes'] and hashlib.sha256(original).hexdigest() == runtime['sha256'],
            'Pinned profile runtime changed')
    require(type(messages) is list and len(messages) <= 16384, 'Bounded actual Cargo artifact messages')
    package = 'path+' + (root / 'third_party/rquickjs-sys').as_uri() + '#0.14.0'
    directory = target / triple / 'shipping/deps'
    candidates = []
    for row in messages:
        if (row.get('reason'), row.get('package_id'), row.get('target', {}).get('kind')) != (
                'compiler-artifact', package, ['lib']):
            continue
        paths = [Path(p) for p in row['filenames']]
        if not any(p.is_relative_to(target / triple) for p in paths):
            continue  # The distinct host cooker archive is not the linked runtime.
        require(row['target']['name'] == 'rquickjs_sys'
                and 'diagnostic-quickjs-generate' in row['features']
                and 'diagnostic-quickjs-use' not in row['features']
                and paths and all(p.parent == directory for p in paths), 'Exact generation runtime artifact role')
        libraries = [p for p in paths if p.suffix == '.rlib']
        require(len(libraries) == 1 and re.fullmatch(r'librquickjs_sys-[0-9a-f]{16}\.rlib', libraries[0].name)
                and all(p.stem == libraries[0].stem and p.suffix in ('.rlib', '.rmeta') for p in paths),
                'Exact runtime rlib artifact filenames')
        candidates.append(libraries[0])
    require(len(candidates) == 1, 'Unique Cargo-owned runtime rlib')
    library = candidates[0]
    raw = read(library, 64 * 1024**2)
    member = archive_member(raw, 'libclang_rt.profile_osx.a')
    require(bundled_payload(member) == original, 'Bundled profile runtime payload differs from pinned archive')
    return {'runtime_rlib': {'path': str(library), 'bytes': len(raw),
                            'sha256': hashlib.sha256(raw).hexdigest()},
            'member_sha256': hashlib.sha256(member).hexdigest(), 'pinned_archive': runtime}
