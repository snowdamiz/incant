"""Checked font data copy for the existing native exporter; no source reads.

The runtime additionally checks the recipe and parses the sfnt face. This step
verifies both envelope and raw-content digest before putting packaged bytes.
"""
import hashlib
import json
import struct
from native_package import key, require, ulid_hex


def package_fonts(cache, assets, put, read):
    require(isinstance(assets, list) and len(assets) <= 32, 'Font catalog count')
    seen, total = set(), 0
    for asset in assets:
        ident = ulid_hex(asset['id'])
        require(ident not in seen, 'Duplicate font asset ID')
        seen.add(ident)
        fingerprint = key(asset['fingerprint'])
        name = fingerprint + '.json'
        data = read(cache / name, 8192)
        metadata = json.loads(data)
        require(metadata['version'] == 1 and metadata['fingerprint'] == fingerprint,
                'Font metadata version/fingerprint mismatch')
        length = metadata['bytes']
        require(type(length) is int and 1 <= length <= 32 * 1024**2, 'Font byte count')
        # Logical IDs are counted independently even when their blobs deduplicate
        # on disk: the currently loaded catalog retains one face per stable ID.
        total += length
        require(total <= 64 * 1024**2, 'Total font bytes')
        content = key(metadata['content_sha256'])
        require(metadata['dependency']['sha256'] == content, 'Font dependency checksum mismatch')
        payload = read(cache / (content + '.ifont'), length + 48)
        require(len(payload) == length + 48 and payload[:8] == b'INCTFNT\0'
                and struct.unpack('<II', payload[8:16]) == (1, length), 'Font envelope mismatch')
        digest = hashlib.sha256(payload[48:]).digest()
        require(payload[16:48] == digest and digest.hex() == content, 'Font checksum mismatch')
        put('fonts/' + name, data)
        put('fonts/' + content + '.ifont', payload)
