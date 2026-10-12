"""Write the Pearl Cove source sheets and the atlas descriptor.

    python3 -B -m art.build <new-directory>      (from samples/planar)

Outputs cove-tiles.png, cove-sprites.png, cove.atlas.json and art.json (frame
names, IDs, regions and SHA-256 of every file). Pixels and the descriptor are
deterministic; PNG bytes can differ only with the platform's zlib. The engine
importer performs packing, extrusion and cooking.
"""
import argparse
import hashlib
import json
from pathlib import Path

from . import cove, sprites
from .image import Canvas

TILE = 16
TILE_COLUMNS = 16
SPRITE_SHEET_WIDTH = 128


def sprite_frames():
    frames = {f'beachcomber/{pose}': sprites.beachcomber(pose) for pose in range(3)}
    frames.update({'boat/0': sprites.boat(), 'boat/1': sprites.boat(lit=True)})
    frames.update({f'clam/{i}': sprites.clam(i) for i in range(4)})
    frames.update({f'urchin/{i}': sprites.urchin(i) for i in range(2)})
    frames.update({f'glint/{i}': sprites.glint(i) for i in range(4)})
    return frames


def shelf(images, width):
    """Deterministic shelf layout: tallest first, then name. Returns regions."""
    order = sorted(images, key=lambda n: (-images[n].height, -images[n].width, n))
    regions, x, y, row = {}, 0, 0, 0
    for name in order:
        image = images[name]
        if x + image.width > width:
            x, y, row = 0, y + row, 0
        regions[name] = (x, y, image.width, image.height)
        x += image.width
        row = max(row, image.height)
    return regions, y + row


def build(out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=False)
    documents, factories = cove.tile_layers()
    tile_names = sorted(factories)
    rows = -(-len(tile_names) // TILE_COLUMNS)
    tiles = Canvas(TILE_COLUMNS * TILE, rows * TILE)
    frames = {}
    for index, name in enumerate(tile_names):
        x, y = (index % TILE_COLUMNS) * TILE, (index // TILE_COLUMNS) * TILE
        tiles.blit(factories[name](), x, y)
        frames[cove.frame_id(name)] = {'name': name, 'path': 'cove-tiles.png',
                                       'region': [x, y, TILE, TILE]}
    images = sprite_frames()
    regions, height = shelf(images, SPRITE_SHEET_WIDTH)
    sheet = Canvas(SPRITE_SHEET_WIDTH, height)
    for name, (x, y, w, h) in regions.items():
        sheet.blit(images[name], x, y)
        frames[cove.frame_id(name)] = {'name': name, 'path': 'cove-sprites.png',
                                       'region': [x, y, w, h]}
    if len(frames) != len(tile_names) + len(images):
        raise ValueError('frame identity collision')
    descriptor = {'version': 1, 'id': cove.ATLAS, 'maximum_size': [256, 256], 'padding': 1,
                  'frames': dict(sorted(frames.items()))}
    files = {
        'cove-tiles.png': tiles.png(),
        'cove-sprites.png': sheet.png(),
        'cove.atlas.json': (json.dumps(descriptor, indent=1) + '\n').encode(),
    }
    for name, data in files.items():
        (out / name).write_bytes(data)
    manifest = {
        'atlas': cove.ATLAS,
        'files': {name: hashlib.sha256(data).hexdigest() for name, data in files.items()},
        'tile_frames': len(tile_names), 'sprite_frames': len(images),
        'frames': {name: cove.frame_id(name) for name in tile_names + sorted(images)},
        'sprite_sizes': {name: [images[name].width, images[name].height] for name in sorted(images)},
    }
    (out / 'art.json').write_text(json.dumps(manifest, indent=1) + '\n')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    manifest = build(parser.parse_args().output)
    print(json.dumps({k: manifest[k] for k in ('files', 'tile_frames', 'sprite_frames')}, indent=1))


if __name__ == '__main__':
    main()
