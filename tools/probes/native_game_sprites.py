"""Numerical atlas fixtures for the source-free native game player proof."""
import json
import struct
import zlib


FRAME = f'{41:026d}'
MIDDLE_FRAME = f'{43:026d}'
LAST_FRAME = f'{44:026d}'


def import_atlas(inputs, project, headless, run, flipbook=False):
    def chunk(kind, data):
        return (struct.pack('>I', len(data)) + kind + data +
                struct.pack('>I', zlib.crc32(kind + data)))

    # Distinct numerical texels exercise atlas sampling and nine-slice geometry.
    rows = b''.join(b'\0' + b''.join(bytes((32 + x * 64, 32 + y * 64, 128, 255))
                                   for x in range(4)) for y in range(4))
    png = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 4, 4, 8, 6, 0, 0, 0)) +
           chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))
    (inputs / 'numeric.png').write_bytes(png)
    atlas = {'version': 1, 'id': f'{42:026d}', 'maximum_size': [64, 64], 'padding': 2,
             'frames': {FRAME: {'name': 'Numeric fixture', 'path': 'numeric.png'}}}
    if flipbook:
        atlas['frames'][MIDDLE_FRAME] = {'name': 'Middle numerical frame', 'path': 'numeric.png', 'region': [0, 0, 2, 4]}
        atlas['frames'][LAST_FRAME] = {'name': 'Final numerical frame', 'path': 'numeric.png', 'region': [2, 2, 2, 2]}
    (inputs / 'numeric.atlas.json').write_text(json.dumps(atlas), encoding='utf-8')
    return json.loads(run([headless, 'import', project, 'numeric.atlas.json']))


def component(texture, sliced):
    return {'texture': texture, 'region': {'frame': FRAME}, 'size': [2, 2],
            'pivot': [.5, .5], 'tint': [1, 1, 1, 1], 'flip_x': sliced,
            'sampling': 'linear' if sliced else 'nearest',
            'mode': {'kind': 'nine_slice', 'border': [1, 1, 1, 1], 'pixels_per_unit': 4}
                    if sliced else {'kind': 'simple'}}
