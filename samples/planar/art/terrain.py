"""Neighbour-aware 16 px terrain tiles: sand, shade overlays, sea, tide pool,
moss-capped basalt cliffs, stone boulders and the pier.

Each tile depends only on its kind, its eight neighbours and a small variant
index, so the same frames form a reusable autotile set; nothing is baked from a
whole-map image. Neighbour sets use the keys n, ne, e, se, s, sw, w, nw.
"""
import math

from . import palette as P
from .image import Canvas, noise, patches

T = 16
DIRECTIONS = ('n', 'ne', 'e', 'se', 's', 'sw', 'w', 'nw')
# Neighbour squares in tile-local pixels, y growing downward (north is up).
SQUARES = {
    'n': (0, -T, T, 0), 'ne': (T, -T, 2 * T, 0), 'e': (T, 0, 2 * T, T),
    'se': (T, T, 2 * T, 2 * T), 's': (0, T, T, 2 * T), 'sw': (-T, T, 0, 2 * T),
    'w': (-T, 0, 0, T), 'nw': (-T, -T, 0, 0),
}
CORNERS = {'ne': ('n', 'e'), 'se': ('s', 'e'), 'sw': ('s', 'w'), 'nw': ('n', 'w')}
CLEAR = (0, 0, 0, 0)


def reduce(around):
    """Drop diagonal neighbours that cannot change a tile (47-variant blob rule)."""
    around = frozenset(around)
    return frozenset(d for d in around
                     if d not in CORNERS or all(side in around for side in CORNERS[d]))


def reduce_open(open_sides):
    """For other-kind neighbour sets: a diagonal matters only when neither of
    its adjacent sides is already other-kind."""
    open_sides = frozenset(open_sides)
    return frozenset(d for d in open_sides
                     if d not in CORNERS or not any(side in open_sides for side in CORNERS[d]))


def distance(x, y, square):
    x0, y0, x1, y1 = square
    return math.hypot(max(x0 - x, 0, x - x1), max(y0 - y, 0, y - y1))


def nearest(x, y, sides):
    return min((distance(x, y, SQUARES[d]) for d in sides), default=99.0)


def corner_inset(x, y, sides, radius):
    """Signed distance into a tile whose convex corners (two adjacent sides of
    another kind) are rounded with `radius`. Negative means cut away."""
    best = 99.0
    for corner, (a, b) in CORNERS.items():
        if a in sides and b in sides:
            cx = radius if corner in ('nw', 'sw') else T - radius
            cy = radius if corner in ('nw', 'ne') else T - radius
            inside_x = x < cx if corner in ('nw', 'sw') else x > cx
            inside_y = y < cy if corner in ('nw', 'ne') else y > cy
            if inside_x and inside_y:
                best = min(best, radius - math.hypot(x - cx, y - cy))
    return best


def wobble(x, y):
    """Shoreline undulation with a 16 px period, so neighbouring tiles agree."""
    return 0.7 * math.sin(2 * math.pi * 2 * x / T) + 0.6 * math.sin(2 * math.pi * 3 * y / T + 1.3)


def sand(variant):
    tile = Canvas(T, T, (*P.SAND, 255))
    for y in range(T):
        for x in range(T):
            n = noise(x, y, 11 + variant)
            if n < 0.03:
                tile.set(x, y, P.SAND_GRAIN)
            elif n < 0.075:
                tile.set(x, y, P.SAND_DARK)
            elif n > 0.97:
                tile.set(x, y, P.SAND_LIGHT)
    if variant == 3:  # wind ripples: a lit crest above a soft trough
        for row, start, length in ((4, 2, 6), (9, 7, 7), (13, 1, 5)):
            for i in range(length):
                lift = 1 if 1 <= i < length - 1 else 0
                tile.set(start + i, row - lift, P.SAND_DARK)
                tile.set(start + i, row - lift - 1, P.SAND_LIGHT)
    elif variant == 4:  # pebbles
        for px, py, w in ((3, 5, 3), (10, 9, 2), (6, 12, 2)):
            for dx in range(w):
                tile.set(px + dx, py, P.PEBBLE)
                tile.set(px + dx, py + 1, P.PEBBLE)
                tile.over(px + dx + 1, py + 2, P.SHADOW, 0.25)
            tile.set(px, py, P.PEBBLE_LIGHT)
    elif variant == 5:  # a washed-up kelp strand
        for x, y in ((3, 10), (4, 9), (5, 9), (6, 10), (7, 10), (8, 9), (9, 8), (10, 8), (11, 9)):
            tile.set(x, y, P.KELP)
            tile.over(x, y + 1, P.SHADOW, 0.22)
        for x, y in ((5, 8), (9, 7), (12, 10)):
            tile.set(x, y, P.KELP_DARK)
    return tile


def shade(rock_sides, water_sides):
    """Translucent overlay for sand: wet margin by water, cast shadow by rock.
    Light comes from the north-west, so only n, nw and w rocks shade a cell."""
    tile = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            cx, cy = x + 0.5, y + 0.5
            wet = nearest(cx, cy, water_sides)
            if wet < 1.5:
                tile.over(x, y, P.WET, 0.30)
            elif wet < 3.2:
                tile.over(x, y, P.WET, 0.15)
            cast = 0.0
            if 'n' in rock_sides and distance(cx, cy, SQUARES['n']) < 3.0:
                cast = 0.30
            if 'w' in rock_sides and distance(cx, cy, SQUARES['w']) < 2.0:
                cast = max(cast, 0.24)
            if 'nw' in rock_sides and distance(cx, cy, SQUARES['nw']) < 2.6:
                cast = max(cast, 0.24)
            if cast:
                tile.over(x, y, P.SHADOW, cast)
    return tile


CREST_DASHES = (
    ((3, 4, 3), (10, 11, 4)),
    ((6, 2, 3), (12, 8, 2), (2, 12, 3)),
    ((9, 5, 4), (4, 13, 3)),
)


def water_depth(land, x, y, radius):
    cx, cy = x + 0.5, y + 0.5
    return min(nearest(cx, cy, land), corner_inset(cx, cy, land, radius))


def water(land, variant, style='sea'):
    """Sea or tide-pool water. `land` lists neighbours that are not water.
    Convex water corners are rounded, so bays and pools have soft outlines."""
    sea = style == 'sea'
    base, shallow = (P.WATER, P.WATER_SHALLOW) if sea else (P.POOL, P.POOL_SHALLOW)
    tile = Canvas(T, T, (*base, 255))
    for y in range(T):
        for x in range(T):
            d = water_depth(land, x, y, 5 if sea else 6)
            if d < 0:  # outside a rounded corner: wet sand continues
                tile.set(x, y, P.SAND_DARK if noise(x, y, 41) > 0.3 else P.SAND_GRAIN)
                continue
            d += wobble(x + 0.5, y + 0.5) * (0.6 if sea else 0.3)
            n = noise(x, y, 31 + variant)
            if sea:
                if d < 1.4:
                    color = P.FOAM if n > 0.15 else P.FOAM_SHADE
                elif d < 2.6:
                    color = P.FOAM_SHADE if n > 0.6 else P.WATER_CREST
                elif d < 4.8:
                    color = shallow
                elif d < 6.6:
                    color = shallow if (x + y) % 2 == 0 else base
                else:
                    color = base
            else:
                if d < 1.3:
                    color = P.POOL_RIM
                elif d < 2.4:
                    color = P.POOL_EDGE
                elif d < 4.6:
                    color = shallow
                elif d < 6.0:
                    color = shallow if (x + y) % 2 == 0 else base
                else:
                    color = P.POOL_STONE if n > 0.94 else base
            tile.set(x, y, color)
    for x0, y0, length in CREST_DASHES[variant % len(CREST_DASHES)] if sea else ():
        cells = [(x0 + i, y0) for i in range(length)]
        if all(water_depth(land, x, y, 5) > 7 for x, y in cells):
            for x, y in cells:
                tile.set(x, y, P.WATER_CREST)
    return tile


FACE = 12
SHORT_FACE = 8
STONE_FACE = 8
# Basalt column boundaries per variant; every set starts at 0 and ends at 16,
# so neighbouring variants always meet on a crevice-to-lit-edge seam.
COLUMN_SETS = ((0, 4, 9, 13, 16), (0, 5, 8, 12, 16), (0, 3, 7, 11, 16))


def column(x, bounds):
    for start, end in zip(bounds, bounds[1:]):
        if start <= x < end:
            return start, end
    raise ValueError(x)


def cliff_face(tile, top_end, variant, open_, moss):
    """Basalt columns lit from the left and darker toward the ground, with moss
    trailing over the lip. Plain facets only: no joints, cracks or speckle, so
    the face never reads as bricks or letters."""
    bounds = COLUMN_SETS[variant % len(COLUMN_SETS)]
    for x in range(T):
        start, end = column(x, bounds)
        width = end - start
        bright = noise(start, 7, 91 + variant) > 0.62  # some columns stand forward
        lip = 1 + (noise(start, 9, 93 + variant) > 0.6)  # and some sit deeper
        for y in range(top_end, T):
            depth = y - top_end
            if x == end - 1:
                color = P.ROCK_DEEP
            elif x == start:
                color = P.STONE if bright else P.ROCK_LIGHT
            elif x == end - 2 and width >= 4:
                color = P.ROCK_DARK
            else:
                color = P.ROCK_LIGHT if bright else P.ROCK
            if depth < lip:
                color = P.ROCK_DEEP  # the cap's overhang shadow
            elif depth == lip and x != end - 1:
                color = P.ROCK_LIGHT if x != end - 2 else P.ROCK  # lit column tops
            elif y >= T - 3 and x != end - 1:
                color = P.ROCK_DARK
            if y == T - 1:
                color = P.OUTLINE
            tile.set(x, y, color)
    if moss:  # moss trailing over the lip
        for x in range(T):
            length = int(noise(x, 2, 83 + variant) * 7) - 3
            for y in range(top_end, top_end + max(0, length)):
                tile.set(x, y, P.MOSS_DARK if y == top_end else P.MOSS_DEEP)
    for side, x in (('w', 0), ('e', T - 1)):
        if open_[side]:
            for y in range(top_end, T):
                tile.set(x, y, P.OUTLINE)


def dome(x, y, open_):
    """Lambert shade of a boulder dome centred where this tile meets its rock
    neighbours, so a multi-tile boulder is lit as one rounded mass."""
    cx = T if not open_['e'] else 0 if not open_['w'] else T / 2
    cy = T if not open_['s'] else 0 if not open_['n'] else T / 2
    nx, ny, nz = (x + 0.5 - cx) / T, (y + 0.5 - cy) / T, 0.9
    length = math.sqrt(nx * nx + ny * ny + nz * nz)
    return (-0.62 * nx - 0.62 * ny + 0.48 * nz) / length


def cap(tile, top_end, variant, moss, open_, quiet=False):
    """The top of a rock: moss on cliffs, bare stone on boulders. Quiet moss has
    no light patches or tufts; it is the calm ground under the HUD line."""
    for y in range(top_end):
        for x in range(T):
            n = noise(x, y, 51 + variant)
            if moss:
                patch = patches(x, y, variant, 53)
                color = P.MOSS_DARK if patch > 0.6 else P.MOSS_LIGHT if patch < 0.2 and not quiet else P.MOSS
                if n > 0.965 and not quiet:
                    color = P.MOSS_LIGHT
            else:  # one dome per boulder, lit from the upper left
                light = dome(x, y, open_) + (n - 0.5) * 0.08
                color = (P.STONE_LIGHT if light > 0.62 else P.STONE if light > 0.4
                         else P.ROCK_LIGHT if light > 0.2 else P.ROCK)
            tile.set(x, y, color)
    if moss and not quiet:  # grass tufts, each with a one-pixel shadow
        for y in range(top_end - 1):
            for x in range(1, T - 1):
                n = noise(x, y, 55 + variant)
                if n > 0.97:
                    tile.set(x, y, P.MOSS_TUFT)
                    tile.set(x, y + 1, P.MOSS_DEEP)
    else:  # lichen flecks
        for y in range(2, top_end - 1):
            for x in range(2, T - 2):
                if noise(x, y, 59 + variant) > 0.975:
                    tile.set(x, y, P.MOSS_TUFT)
                    tile.set(x + 1, y, P.MOSS_LIGHT)
    if top_end < T:  # the lip above a face
        for x in range(T):
            tile.set(x, top_end - 1, P.MOSS_DEEP if moss else P.ROCK_LIGHT)
    rim_light, rim_dark = (P.MOSS_LIGHT, P.MOSS_DARK) if moss else (P.STONE_LIGHT, P.ROCK_LIGHT)
    if open_['n']:
        for x in range(T):
            tile.set(x, 0, P.OUTLINE)
            tile.set(x, 1, rim_light)
    for y in range(min(top_end, T)):
        if open_['w']:
            tile.set(0, y, P.OUTLINE)
            if y > 0:
                tile.set(1, y, rim_light)
        if open_['e']:
            tile.set(T - 1, y, P.OUTLINE)
            if y > 0:
                tile.set(T - 2, y, rim_dark)


def rock(solid, variant, kind='moss'):
    """Rock seen in three-quarter view. `solid` lists rock neighbours. A south
    edge shows a cliff face inside this cell, so the collider and art agree.
    Kinds: 'moss' cliff, 'quiet' moss cliff without tufts, 'stone' boulder."""
    tile = Canvas(T, T)
    quiet = kind == 'quiet'
    moss = kind in ('moss', 'quiet')
    open_ = {d: d not in solid for d in DIRECTIONS}
    face = 0
    if open_['s']:
        face = STONE_FACE if not moss else SHORT_FACE if open_['n'] else FACE
    top_end = T - face
    if face:
        cliff_face(tile, top_end, variant, open_, moss)
    cap(tile, top_end, variant, moss, open_, quiet)
    # Rounded convex corners reveal the ground layer beneath.
    sides = {d for d in ('n', 'e', 's', 'w') if open_[d]}
    radius = 3 if moss else 5
    for y in range(T):
        for x in range(T):
            d = corner_inset(x + 0.5, y + 0.5, sides, radius)
            if d < 0:
                tile.set(x, y, CLEAR)
            elif d < 1:
                tile.set(x, y, P.OUTLINE)
    if not moss:  # lumpy boulder rims: shallow notches along open top edges
        for i in range(4, T - 4):
            if open_['n'] and noise(i, 0, 63 + variant) > 0.7:
                tile.set(i, 0, CLEAR)
                tile.set(i, 1, P.OUTLINE)
            for side, x, inward in (('w', 0, 1), ('e', T - 1, -1)):
                if open_[side] and i < top_end - 2 and noise(x, i, 67 + variant) > 0.7:
                    tile.set(x, i, CLEAR)
                    tile.set(x + inward, i, P.OUTLINE)
    # Inner corners: a lower diagonal neighbour notches the rim.
    for corner, (a, b) in (('nw', ('n', 'w')), ('ne', ('n', 'e'))):
        if not open_[a] and not open_[b] and open_[corner]:
            tile.set(0 if corner == 'nw' else T - 1, 0, P.OUTLINE)
    return tile


def pier(land, variant, end):
    """Pier planks over sea water. `land` lists non-water neighbours, so the
    water edges match the surrounding sea tiles. `end` adds the seaward posts."""
    tile = water(land, variant)
    left, right = 2, 13
    for y in range(T):
        tile.over(right + 1, y, P.SHADOW, 0.35)  # the deck shades the water east
        tile.over(right + 2, y, P.SHADOW, 0.15)
    for y in range(T):
        board = (y // 4) % 2
        for x in range(left, right + 1):
            if y % 4 == 3:
                color = P.WOOD_DEEP
            else:
                color = P.WOOD if board else P.WOOD_LIGHT
                if noise(x, y, 101 + variant) > 0.9:
                    color = P.WOOD_DARK
            tile.set(x, y, color)
        tile.set(left, y, P.WOOD_DARK)
        tile.set(right, y, P.WOOD_DEEP)
        if y % 4 == 1:
            tile.set(left + 1, y, P.WOOD_DEEP)  # nails
            tile.set(right - 1, y, P.WOOD_DEEP)
    if end:  # posts where the pier ends over water
        for px in (left - 1, right):
            for y in range(T - 5, T):
                tile.set(px, y, P.WOOD_DEEP)
                tile.set(px + 1, y, P.WOOD_DARK if y > T - 5 else P.WOOD_LIGHT)
            tile.over(px + 2, T - 1, P.SHADOW, 0.3)
    return tile
