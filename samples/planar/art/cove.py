"""Pearl Cove layout, stable identities and per-cell tile assignment.

The map is 20 x 12 one-metre cells drawn at 16 px per metre. Row strings run
from the top (y = 11) to the bottom (y = 0); the engine's cell (0, 0) is the
bottom-left. Legend: '#' grassy cliff, 'b' stone boulder, '~' sea, 'p' tide
pool, '=' pier, '.' sand, ':' sand with pebbles, ';' sand with kelp.
"""
import hashlib

from . import terrain

WIDTH, HEIGHT = 20, 12
PIXELS_PER_METRE = 16
MAP = (
    '####################',
    '####################',
    '##....;.######....##',
    '#........####....:.#',
    '##.................#',
    '#..bb...pp.....bb..#',
    '#..bb....p.....bb.##',
    '#...:.............;#',
    '##.....~~~=~~....~~#',
    '#~~~..~~~~=~~~~~~~~#',
    '~~~~~~~~~~~~~~~~~~~~',
    '~~~~~~~~~~~~~~~~~~~~',
)
ROCK, WATER, SAND = '#b', '~p', '.:;'

CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'


def ulid(kind, number):
    """Readable canonical ULID: 01PS + two-letter kind + zero-padded number."""
    digits = ''
    while True:
        digits = CROCKFORD[number % 32] + digits
        number //= 32
        if not number:
            break
    return '01PS' + kind + digits.rjust(20, '0')


def frame_id(name):
    """Stable atlas frame ULID derived from the frame's descriptive name."""
    value = int.from_bytes(hashlib.sha256(name.encode()).digest()[:13], 'big') >> 4
    return '01PSFR' + ''.join(CROCKFORD[(value >> (5 * i)) & 31] for i in reversed(range(20)))


ATLAS = ulid('AT', 1)
SCRIPT = ulid('SC', 1)
PLAYER, TERRAIN, CAMERA, CONTROLLER, BOAT = (ulid('EN', n) for n in (1, 2, 3, 4, 5))
PEARLS = [ulid('EN', n) for n in (11, 12, 13)]
URCHINS = [ulid('EN', n) for n in (21, 22, 23)]
GLINTS = [ulid('EN', n) for n in (31, 32, 33, 34)]
LAYERS = {name: ulid('TM', n) for n, name in enumerate(
    ('ground', 'shade', 'water', 'rock', 'pier'), start=1)}

START = (6.5, 4.5)
PEARL_AT = [(3.5, 8.5), (16.5, 8.5), (17.5, 4.5)]
URCHIN_AT = [(5.5, 8.5), (14.5, 7.5), (13.5, 3.5)]
# The boat hangs below the pier end; the exit sensor sits on that edge.
EXIT_AT = (10.5, 2.0)
GLINT_AT = [(2.75, 1.25), (6.75, 0.75), (14.25, 1.5), (17.25, 0.75)]
# Pixel-perfect camera: 320 x 180 logical pixels at 16 px per metre.
CAMERA_AT = (10.0, 6.0, 20.0)
REFERENCE_RESOLUTION = (320, 180)


def kind(x, y):
    """Map character at engine cell (x, y); outside cells repeat the edge."""
    x = min(max(x, 0), WIDTH - 1)
    y = min(max(y, 0), HEIGHT - 1)
    return MAP[HEIGHT - 1 - y][x]


OFFSETS = {'n': (0, 1), 'ne': (1, 1), 'e': (1, 0), 'se': (1, -1), 's': (0, -1),
           'sw': (-1, -1), 'w': (-1, 0), 'nw': (-1, 1)}


def around(x, y, accept):
    return {d for d, (dx, dy) in OFFSETS.items() if kind(x + dx, y + dy) in accept}


def variant(x, y, count):
    return (x * 7 + y * 13 + (x * y) % 5) % count


def water_land(x, y):
    """Non-water neighbours of a water or pier cell (piers count as water)."""
    land = set(OFFSETS) - around(x, y, WATER + '=')
    return terrain.reduce_open(land)


def assignments():
    """{layer: {(x, y): frame name}} plus {frame name: tile factory}."""
    layers = {name: {} for name in LAYERS}
    factories = {}

    def use(layer, cell, name, factory):
        layers[layer][cell] = name
        factories.setdefault(name, factory)

    for y in range(HEIGHT):
        for x in range(WIDTH):
            c = kind(x, y)
            cell = (x, y)
            if c in SAND:
                v = {':': 4, ';': 5}.get(c, 3 if variant(x, y, 8) == 0 else variant(x, y, 3))
                use('ground', cell, f'sand/{v}', lambda v=v: terrain.sand(v))
                rocks = frozenset(around(x, y, ROCK) & {'n', 'w', 'nw'})
                wet = frozenset(terrain.reduce_open(around(x, y, WATER)))
                if rocks or wet:
                    key = '-'.join(sorted(rocks)) + '|' + '-'.join(sorted(wet))
                    use('shade', cell, f'shade/{key}',
                        lambda r=rocks, w=wet: terrain.shade(r, w))
            else:
                use('ground', cell, 'sand/0', lambda: terrain.sand(0))
            if c in WATER:
                land = water_land(x, y)
                style = 'sea' if c == '~' else 'pool'
                v = variant(x, y, 3) if not land else 0
                key = '-'.join(sorted(land))
                use('water', cell, f'{style}/{key}/{v}',
                    lambda l=land, v=v, s=style: terrain.water(l, v, s))
            if c in ROCK:
                solid = terrain.reduce(around(x, y, ROCK))
                # The top two cliff rows are the HUD's ground: quiet moss.
                cap = 'stone' if c == 'b' else 'quiet' if y >= HEIGHT - 2 else 'moss'
                v = variant(x, y, 3)
                key = '-'.join(sorted(solid))
                use('rock', cell, f'rock/{cap}/{key}/{v}',
                    lambda s=solid, v=v, k=cap: terrain.rock(s, v, k))
                if around(x, y, WATER):  # sea rocks stand in water, not sand
                    use('water', cell, 'sea//0', lambda: terrain.water(frozenset(), 0))
            if c == '=':
                land = water_land(x, y)
                end = kind(x, y - 1) in WATER
                key = '-'.join(sorted(land)) + ('/end' if end else '')
                use('pier', cell, f'pier/{key}', lambda l=land, e=end: terrain.pier(l, 0, e))
    return layers, factories


def tile_layers():
    """Tilemap layer documents (bottom-to-top rows) and the frames they use."""
    layers, factories = assignments()
    documents = []
    order = {'ground': 0, 'shade': 1, 'water': 2, 'rock': 3, 'pier': 4}
    for name, cells in layers.items():
        documents.append({
            'id': LAYERS[name],
            'cells': [frame_id(cells[(x, y)]) if (x, y) in cells else None
                      for y in range(HEIGHT) for x in range(WIDTH)],
            'layer': 0, 'order': order[name], 'solid': name in ('water', 'rock'),
        })
    return documents, factories
