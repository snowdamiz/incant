"""Pearl Cove sprites: the beachcomber, pearl clam, sea urchin, rowboat and
water glints. Light comes from the upper left; each grounded sprite carries its
own soft contact shadow so no extra entity is needed.
"""
import math

from . import palette as P
from .image import Canvas, grid, noise

SHADOW = (*P.SHADOW, round(255 * P.SHADOW_ALPHA))
LANTERN_LIT = (226, 178, 84)
LANTERN_CORE = (231, 220, 160)

BEACHCOMBER = (
    '................',
    '......oooo......',
    '....oohhRRoo....',
    '...ohhRRRRRro...',
    '..ohhRRRRRRrro..',
    '..ohRRHHHHRRro..',
    '..ohRHssssHRro..',
    '..oRRsesseRrro..',
    '..orRdssssdrro..',
    '...ohRRRRRRro...',
    '..ohRRRyRRRrro..',
    '..ohRRRyRRrrro..',
    '..orrRRRRrrrro..',
    '...oobbooBboo...',
    '..xxoBbooBbox...',
    '....xxxxxxxx....',
)
BEACHCOMBER_KEYS = {
    'o': P.INK, 'R': P.COAT, 'r': P.COAT_DARK, 'h': P.COAT_LIGHT, 's': P.SKIN,
    'd': P.SKIN_DARK, 'e': P.INK, 'H': P.HAIR, 'b': P.BOOT, 'B': P.BOOT_LIGHT,
    'y': P.SAND_LIGHT, 'x': SHADOW, 'w': P.FOAM,
}

# Sparkle on the pearl's upper-left rim, over the shell, so it reads at 1x.
SPARKLES = (
    (),
    ((5, 7),),
    ((5, 7), (4, 7), (6, 7), (5, 6), (5, 8)),
    ((5, 7), (5, 5), (3, 7), (7, 7), (5, 9)),
)

# Hook-ready poses: 0 default, 1 stung by an urchin, 2 cheering at the boat.
STUNG = {7: '..oRRsdssdRrro..', 8: '..orRdseesdrro..', 2: '.w..oohhRRoo..w.', 4: 'w.ohhRRRRRRrro.w'}
CHEER = {
    2: 'os..oohhRRoo..so', 3: 'oR.ohhRRRRRro.Ro', 4: 'oRohhRRRRRRrroRo',
    5: 'oRohRRHHHHRRroRo', 6: '.oohRHssssHRroo.', 8: '..orRdseesdrro..',
}


def beachcomber(pose=0):
    rows = list(BEACHCOMBER)
    for index, row in ({1: STUNG, 2: CHEER}.get(pose, {})).items():
        rows[index] = row
    return grid(rows, BEACHCOMBER_KEYS)


def clam(frame):
    """An open scallop: ribbed fan valve behind, shallow dish, glossy pearl."""
    tile = Canvas(16, 16)
    hx, hy = 7.5, 11.6
    for y in range(16):
        for x in range(16):
            px, py = x + 0.5, y + 0.5
            if ((px - 7.5) / 6.6) ** 2 + ((py - 13.4) / 1.7) ** 2 <= 1.0:
                tile.over(x, y, P.SHADOW, P.SHADOW_ALPHA)
            dx, dy = px - hx, py - hy
            if dy < 0.3:  # the fan valve standing behind the pearl
                theta = math.atan2(-dy, dx)
                theta = min(max(theta, 0.0), math.pi)
                rib = theta / (math.pi / 9)
                edge = 6.6 + 0.8 * abs(math.cos(math.pi * (rib % 1.0 - 0.5)))
                r = math.hypot(dx, dy * 0.92)
                if r <= edge:
                    within = rib % 1.0
                    color = P.SHELL_LIGHT if within > 0.55 else P.SHELL
                    if within < 0.18 or r > edge - 1.0 or r < 1.6:
                        color = P.SHELL_DARK
                    tile.set(x, y, color)
            if py >= hy - 0.2 and ((px - 7.5) / 6.4) ** 2 + ((py - hy) / 2.7) ** 2 <= 1.0:
                inner = ((px - 7.5) / 5.2) ** 2 + ((py - hy + 0.4) / 1.6) ** 2 <= 1.0
                tile.set(x, y, P.SHELL_LIGHT if inner else P.SHELL_DARK)
    for y in range(16):  # the pearl, lit from the upper left
        for x in range(16):
            dx, dy = x + 0.5 - 7.5, y + 0.5 - 10.2
            r = math.hypot(dx, dy)
            if r <= 2.7:
                light = -(dx + dy) / max(r, 0.01)
                color = P.PEARL_MID
                if r > 2.0 and light < 0.2:
                    color = P.PEARL_SHADOW
                elif light > 0.3 and r > 0.6:
                    color = P.PEARL
                tile.set(x, y, color)
    for i, (x, y) in enumerate(SPARKLES[frame]):
        tile.set(x, y, P.SPARKLE if i == 0 else P.PEARL)
    return tile


def urchin(frame):
    """A radially symmetric urchin; the second frame shifts the spines."""
    tile = Canvas(16, 16)
    cx, cy = 7.5, 8.0
    for y in range(16):
        for x in range(16):
            if ((x + 0.5 - cx) / 6.0) ** 2 + ((y + 0.5 - 13.4) / 1.7) ** 2 <= 1.0:
                tile.over(x, y, P.SHADOW, P.SHADOW_ALPHA)
    spines = 12
    offset = (math.pi / spines) * (0.35 if frame else 0.0)
    for i in range(spines):
        angle = offset + 2 * math.pi * i / spines
        length = 7.2 if i % 2 == 0 else 5.6
        for step in range(30):
            r = 3.0 + (length - 3.0) * step / 29
            x = int(math.floor(cx + math.cos(angle) * r))
            y = int(math.floor(cy - math.sin(angle) * r * 0.92))
            if not tile.inside(x, y):
                continue
            tip = r > length - 1.2
            lit = math.cos(angle - math.radians(135)) > 0.3
            color = P.URCHIN_TIP if tip else (P.URCHIN_SPINE if lit else P.URCHIN)
            tile.set(x, y, color)
    for y in range(16):
        for x in range(16):
            dx, dy = x + 0.5 - cx, y + 0.5 - cy
            r = math.hypot(dx, dy * 1.08)
            if r <= 3.9:
                light = -(dx + dy) / max(r, 0.01)
                color = P.URCHIN if 1.0 < r <= 3.0 and light > 0.4 else P.URCHIN_BODY
                if 0.9 < r < 2.2 and dx < -0.3 and dy < -0.3:
                    color = P.URCHIN_SHINE if noise(x, y, 7) > 0.3 else P.URCHIN_TIP
                tile.set(x, y, color)
    return tile


def boat(lit=False):
    """A small moored rowboat, bow to the east, its hull painted the coat's red.
    A bow lantern is dark by default; `lit` is the hook-ready "boat ready" frame."""
    tile = Canvas(32, 16)
    centre = 7.0

    def half(x):
        u = (x + 0.5 - 1.0) / 29.5
        if u < 0 or u > 1:
            return 0.0
        if u < 0.16:
            return 5.3 * math.sqrt(max(0.0, 1 - ((0.16 - u) / 0.16) ** 2))
        if u < 0.55:
            return 5.3
        return 5.3 * (1 - ((u - 0.55) / 0.45) ** 2)

    def inside(x, y):
        return 0 <= x < 32 and 0 <= y < 16 and abs(y + 0.5 - centre) < half(x)

    for y in range(16):  # shadow on the water, offset south-east
        for x in range(32):
            if not inside(x, y) and inside(x - 1, y - 2):
                tile.over(x, y, P.SHADOW, 0.30)
    for y in range(16):
        for x in range(32):
            if not inside(x, y):
                continue
            h = half(x)
            offset = y + 0.5 - centre
            edge = not all(inside(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
            if edge:
                color = P.WOOD_DEEP
            elif offset > h - 2.4:
                color = P.COAT if offset > h - 1.5 else P.COAT_LIGHT  # painted hull side
            elif offset < -h + 2.0 or abs(offset) > h - 3.2:
                color = P.WOOD_LIGHT  # gunwale
            elif offset < -h + 3.2:
                color = P.WOOD_DARK  # inner hull in shadow
            else:
                color = P.WOOD_DARK if (y - 2) % 3 == 0 else P.WOOD
            tile.set(x, y, color)
    for x0 in (9, 18):  # thwart seats, shaded on their east side
        for y in range(16):
            if inside(x0, y) and abs(y + 0.5 - centre) < half(x0) - 2.2:
                tile.set(x0, y, P.WOOD_LIGHT)
                tile.set(x0 + 1, y, P.WOOD_LIGHT)
                tile.set(x0 + 2, y, P.WOOD_DEEP)
    for y in range(3, 7):  # lantern post at the bow
        tile.set(25, y, P.WOOD_DEEP)
    for x, y in ((24, 0), (25, 0), (26, 0), (23, 1), (27, 1), (23, 2), (27, 2), (24, 3), (25, 3), (26, 3)):
        tile.set(x, y, P.OUTLINE)
    for x, y in ((24, 1), (25, 1), (26, 1), (24, 2), (25, 2), (26, 2)):
        glow = LANTERN_CORE if (x, y) == (25, 2) else LANTERN_LIT
        tile.set(x, y, glow if lit else P.ROCK_DEEP)
    return tile


def glint(frame):
    """Water sparkle: dot, small cross, larger cross; frame 3 is empty."""
    tile = Canvas(8, 8)
    shapes = (
        ((3, 4),),
        ((3, 4), (2, 4), (4, 4), (3, 3), (3, 5)),
        ((3, 4), (2, 4), (4, 4), (3, 3), (3, 5), (1, 4), (5, 4), (3, 2), (3, 6)),
        (),
    )
    for i, (x, y) in enumerate(shapes[frame]):
        tile.set(x, y, P.FOAM if i == 0 else P.WATER_CREST)
    return tile
