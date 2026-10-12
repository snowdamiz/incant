"""Small RGBA canvas and a minimal PNG writer (no third-party imaging)."""
import struct
import zlib

CLEAR = (0, 0, 0, 0)


class Canvas:
    def __init__(self, width, height, fill=CLEAR):
        self.width, self.height = width, height
        self.pixels = [list(fill) for _ in range(width * height)]

    def inside(self, x, y):
        return 0 <= x < self.width and 0 <= y < self.height

    def get(self, x, y):
        return tuple(self.pixels[y * self.width + x])

    def set(self, x, y, color):
        """Opaque or straight-alpha write; y grows downward as in the image."""
        if self.inside(x, y):
            self.pixels[y * self.width + x] = list(color if len(color) == 4 else (*color, 255))

    def over(self, x, y, color, alpha):
        """Straight-alpha source-over, used for soft shadows and overlays."""
        if not self.inside(x, y) or alpha <= 0:
            return
        r, g, b, a = self.get(x, y)
        a0 = a / 255
        out = alpha + a0 * (1 - alpha)
        if out <= 0:
            return
        mix = [(c * alpha + d * a0 * (1 - alpha)) / out for c, d in zip(color[:3], (r, g, b))]
        self.pixels[y * self.width + x] = [round(v) for v in mix] + [round(out * 255)]

    def blit(self, other, dx, dy):
        for y in range(other.height):
            for x in range(other.width):
                color = other.get(x, y)
                if color[3]:
                    self.set(dx + x, dy + y, color)

    def png(self):
        def chunk(kind, data):
            return (struct.pack('>I', len(data)) + kind + data
                    + struct.pack('>I', zlib.crc32(kind + data) & 0xFFFFFFFF))
        rows = bytearray()
        for y in range(self.height):
            rows.append(0)
            for x in range(self.width):
                rows.extend(self.get(x, y))
        # The importer reads color PNG sheets as sRGB; no color chunks are needed.
        header = struct.pack('>2I5B', self.width, self.height, 8, 6, 0, 0, 0)
        return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header)
                + chunk(b'IDAT', zlib.compress(bytes(rows), 9)) + chunk(b'IEND', b''))


def grid(rows, palette):
    """Canvas from equal-length text rows; '.' is transparent."""
    canvas = Canvas(len(rows[0]), len(rows))
    for y, row in enumerate(rows):
        if len(row) != canvas.width:
            raise ValueError(f'row {y} has width {len(row)}, expected {canvas.width}')
        for x, key in enumerate(row):
            if key != '.':
                color = palette[key]
                if len(color) == 4 and color[3] < 255:
                    canvas.over(x, y, color[:3], color[3] / 255)
                else:
                    canvas.set(x, y, color)
    return canvas


def noise(x, y, seed):
    """Deterministic value in [0, 1) for a pixel and seed."""
    n = (x * 374761393 + y * 668265263 + seed * 2147483647) & 0xFFFFFFFF
    n = ((n ^ (n >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((n ^ (n >> 16)) & 0xFFFFFF) / 0x1000000


def patches(x, y, variant, seed, step=4, border=0.4):
    """Smooth value noise in [0, 1) for one 16 px tile. The lattice is held at a
    neutral `border` value on the tile edge, so tiles of any variant join without
    seams and no motif repeats on the 16 px grid; patches form inside tiles."""
    cells = 16 // step

    def lattice(i, j):
        if i in (0, cells) or j in (0, cells):
            return border
        return noise(i, j, seed + 3 + variant * 17)

    gx, gy = x / step, y / step
    i, j = min(int(gx), cells - 1), min(int(gy), cells - 1)
    fx, fy = gx - i, gy - j
    fx, fy = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy)
    top = lattice(i, j) * (1 - fx) + lattice(i + 1, j) * fx
    bottom = lattice(i, j + 1) * (1 - fx) + lattice(i + 1, j + 1) * fx
    return top * (1 - fy) + bottom * fy
