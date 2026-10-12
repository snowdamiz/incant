"""Bounded data packaging for the trusted native project exporter.

No project file chooses a tool, output path or executable artifact. Referenced
caches contain data only and are validated again by the real player before a
package is published. Publication is create-only; existing outputs are preserved.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def key(value):
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value), 'Invalid cooked asset key')
    return value


def read(path, limit):
    """Reject linked/special files and bound the read even after metadata races."""
    path = Path(path)
    for parent in (path, *path.parents):
        require(not parent.is_symlink() and not getattr(parent, 'is_junction', lambda: False)(),
                f'Linked package input: {path.name}')
    before = path.stat(follow_symlinks=False)
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= limit,
            f'Invalid or oversized package input: {path.name}')
    fd = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0))
    with os.fdopen(fd, 'rb') as stream:
        actual = os.fstat(stream.fileno())
        require(stat.S_ISREG(actual.st_mode) and (before.st_dev, before.st_ino) == (actual.st_dev, actual.st_ino),
                f'Package input changed while opening: {path.name}')
        data = stream.read(limit + 1)
        after = os.fstat(stream.fileno())
    require(len(data) <= limit and (actual.st_size, actual.st_mtime_ns) == (after.st_size, after.st_mtime_ns),
            f'Package input changed or exceeded its bound: {path.name}')
    return data


def ulid_hex(value):
    alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
    require(isinstance(value, str) and len(value) == 26 and all(c in alphabet for c in value), 'Expected canonical ULID')
    number = 0
    for char in value:
        number = number * 32 + alphabet.index(char)
    require(number < 1 << 128, 'ULID exceeds 128 bits')
    return f'{number:032x}'


def select_camera(receipt, selected, window):
    cameras = receipt['cameras']
    require(isinstance(cameras, list) and len(cameras) <= 100_000, 'Invalid camera catalog')
    for camera in cameras:
        ulid_hex(camera)
    if selected is not None:
        require(selected in cameras, 'Selected camera is absent from the cooked scene')
    elif len(cameras) == 1:
        selected = cameras[0]
    elif window or receipt['has_spatial_content']:
        raise ValueError('Choose --camera ULID when a spatial scene does not have exactly one camera')
    return ulid_hex(selected) if selected is not None else None


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def data_package(root, producer, receipt, destination, camera):
    """Copy only referenced cooked data. All names below are engine-owned."""
    require(receipt.get('format') == 'incant-native-project-build' and receipt.get('version') == 1,
            'Unsupported project build receipt')
    destination.mkdir()
    for directory in ('models', 'textures', 'audio', 'fonts'):
        (destination / directory).mkdir()
    manifest = {}
    total = 0

    def put(name, data):
        nonlocal total
        path = destination / name
        if path.exists():
            require(read(path, len(data)) == data, 'Conflicting cooked package entries')
        else:
            total += len(data)
            require(total <= 2 * 1024**3, 'Cooked package exceeds 2 GiB')
            with path.open('xb') as stream:
                stream.write(data)
        manifest[name] = sha256(data)

    scene = read(producer / 'world.scene', 128 * 1024**2)
    require(sha256(scene) == key(receipt['scene_sha256']), 'Cooked scene receipt mismatch')
    put('world.scene', scene)
    put('input-actions.json', read(producer / 'input-actions.json', 256 * 1024))
    tick_rate = receipt['tick_rate']
    require(type(tick_rate) is int and 1 <= tick_rate <= 240, 'Invalid cooked tick rate')
    put('empty-input.json', json.dumps({'format': 'incant-input', 'version': 1, 'tick_rate': tick_rate,
                                       'start_tick': 0, 'ticks': 3600, 'frames': []}).encode())
    cache = root / '.incant/cache'
    visual = receipt['visual_assets']
    require(isinstance(visual, list) and len(visual) <= 4096, 'Visual asset count exceeds export capacity')
    for asset in visual:
        fingerprint = key(asset['fingerprint'])
        if asset['kind'] == 'model':
            name = fingerprint + '.incmodel'
            put('models/' + name, read(cache / 'models' / name, 128 * 1024**2))
        elif asset['kind'] == 'texture':
            name = fingerprint + '.json'
            data = read(cache / 'textures' / name, 2 * 1024**2)
            metadata = json.loads(data)
            require(metadata['fingerprint'] == fingerprint, 'Texture fingerprint mismatch')
            put('textures/' + name, data)
            # Sprite and data-texture roles may retain an unweighted color mip
            # variant. The typed runtime loader checks its exact role/version.
            for field in ('content_sha256', 'unweighted_content_sha256'):
                if field not in metadata or metadata[field] is None:
                    require(field != 'content_sha256', 'Missing texture content hash')
                    continue
                content = key(metadata[field])
                payload = read(cache / 'textures' / (content + '.ktx2'), 128 * 1024**2)
                require(sha256(payload) == content, 'Texture checksum mismatch')
                put('textures/' + content + '.ktx2', payload)
        else:
            raise ValueError('Unsupported visual asset kind')
    from native_fonts import package_fonts
    package_fonts(cache / 'fonts', receipt.get('font_assets', []), put, read)
    audio = receipt['audio_clips']
    require(isinstance(audio, dict) and len(audio) <= 128, 'Audio clip count exceeds export capacity')
    clips = bytearray(b'NCLIPS01' + struct.pack('<I', len(audio)))
    for ident, fingerprint in sorted(audio.items()):
        fingerprint = key(fingerprint)
        clips.extend(bytes.fromhex(ulid_hex(ident)) + bytes.fromhex(fingerprint))
        name = fingerprint + '.json'
        data = read(cache / 'audio' / name, 2 * 1024**2)
        metadata = json.loads(data)
        require(metadata['fingerprint'] == fingerprint, 'Audio fingerprint mismatch')
        content = key(metadata['content_sha256'])
        payload = read(cache / 'audio' / (content + '.ipcm'), 256 * 1024**2)
        require(sha256(payload) == content, 'Audio checksum mismatch')
        put('audio/' + name, data)
        put('audio/' + content + '.ipcm', payload)
    put('clips.bin', clips)
    startup = {name: manifest[name] for name in ('world.scene', 'input-actions.json', 'clips.bin', 'empty-input.json')}
    write_json(destination / 'package.json', {'format': 'incant-native-package', 'version': 1,
                                             'camera': camera, 'files': startup})
    package_bytes = read(destination / 'package.json', 8192)
    total += len(package_bytes)
    require(total <= 2 * 1024**3, 'Cooked package exceeds 2 GiB')
    manifest['package.json'] = sha256(package_bytes)
    return {'bytes': total, 'files': manifest}


def publish(staging, output):
    """Reserve output without replacement; a receipt is copied last.

    A failed copy leaves the new incomplete directory for inspection, never
    recursively removes user content. Re-running requires a new destination.
    """
    output.mkdir(exist_ok=False)
    paths = sorted(staging.rglob('*'), key=lambda p: (p.name == 'export.json', p.as_posix()))
    for source in paths:
        target = output / source.relative_to(staging)
        require(not source.is_symlink(), 'Unexpected link in private export staging')
        if source.is_dir():
            target.mkdir(exist_ok=True)
        else:
            with source.open('rb') as incoming, target.open('xb') as outgoing:
                shutil.copyfileobj(incoming, outgoing)
            shutil.copystat(source, target)
