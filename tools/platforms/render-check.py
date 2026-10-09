#!/usr/bin/env python3
"""Run the real headless renderer and validate PNG transport, not appearance."""
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import zlib

ROOT = Path(__file__).resolve().parents[2]


def validate_png(data, width, height):
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('Renderer did not produce a PNG')
    offset, kinds = 8, []
    while offset < len(data):
        length = struct.unpack('>I', data[offset:offset + 4])[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        crc = struct.unpack('>I', data[offset + 8 + length:offset + 12 + length])[0]
        if zlib.crc32(kind + payload) != crc:
            raise ValueError('Incomplete or corrupt renderer PNG')
        if kind == b'IHDR' and struct.unpack('>II', payload[:8]) != (width, height):
            raise ValueError('Renderer returned incorrect dimensions')
        kinds.append(kind)
        offset += length + 12
    if not kinds or kinds[0] != b'IHDR' or kinds[-1] != b'IEND' or b'IDAT' not in kinds or offset != len(data):
        raise ValueError('Renderer PNG has missing chunks')


def main():
    executable = ROOT / 'target/release' / ('incant_headless.exe' if sys.platform == 'win32' else 'incant_headless')
    output = ROOT / 'artifacts/render-smoke'
    output.mkdir(parents=True, exist_ok=True)
    project, image = output / 'sample.incant.json', output / 'viewport.png'
    subprocess.run([str(executable), 'init', str(project), '--name', 'CI native renderer probe', '--entities', '3'], check=True, timeout=30)
    response = subprocess.check_output([str(executable), 'screenshot', str(project), str(image), '--width', '320', '--height', '180'], text=True, timeout=120)
    engine = json.loads(response)
    data = image.read_bytes()
    validate_png(data, 320, 180)
    report = {'platform': sys.platform, 'native_renderer_executed': True, 'png_transport_validated': True, 'width': 320, 'height': 180, 'sha256': hashlib.sha256(data).hexdigest(), 'engine': engine, 'visual_review': False, 'native_window_composition_review': False}
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
