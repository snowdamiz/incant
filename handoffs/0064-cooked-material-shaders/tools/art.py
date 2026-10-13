#!/usr/bin/env python3
"""Deterministic review art for handoff 0064: textures, meshes and glTF materials.

Numerical review fixtures, not game art. Standard library only; writes a new
directory and refuses to overwrite. Every glTF uses only core glTF 2.0
metallic-roughness fields (the importer rejects extensions) and omits TANGENT,
so the importer's MikkTSpace generation supplies the normal-map basis.

  python3 -I -B art.py <new-directory>
"""

import hashlib
import json
import math
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

NEAREST, LINEAR, TRILINEAR, REPEAT, CLAMP = 9728, 9729, 9987, 10497, 33071


def image(width, height, texel):
    rows = []
    for y in range(height):
        row = bytearray()
        for x in range(width):
            row.extend(bytes(max(0, min(255, int(round(v)))) for v in texel(x, y)))
        rows.append(bytes(row))
    return width, height, rows


def checker(x, y):
    quadrant = (x >= 32) + 2 * (y >= 32)
    light = ((x // 8) + (y // 8)) % 2 == 0
    colors = [((220, 40, 30), (245, 245, 245)), ((40, 170, 60), (245, 245, 245)),
              ((40, 80, 210), (245, 245, 245)), ((235, 200, 30), (20, 20, 20))]
    rgb = colors[quadrant][0 if light else 1]
    if x < 4 and y < 4:
        rgb = (0, 0, 0)  # orientation mark in the image's top-left corner
    return (*rgb, 255)


def metallic_roughness(x, y):
    # glTF: G = roughness (0.03..1 left to right), B = metallic (top half metal).
    return (0, 8 + 247 * x / 63, 255 if y < 32 else 0, 255)


def normal(x, y):
    cx, cy = (x // 16) * 16 + 8, (y // 16) * 16 + 8
    dx, dy = (x + 0.5 - cx) / 6.5, -(y + 0.5 - cy) / 6.5  # +Y up in tangent space
    n = (0.0, 0.0, 1.0)
    if dx * dx + dy * dy < 1.0:
        n = (0.8 * dx, 0.8 * dy, 1.0)
    length = math.sqrt(sum(v * v for v in n))
    return (*[(v / length * 0.5 + 0.5) * 255 for v in n], 255)


def occlusion(x, y):
    groove = x % 16 in (0, 1) or y % 16 in (0, 1)
    value = 40 if groove else 255
    return (value, value, value, 255)


def emissive(x, y):
    r = math.hypot(x + 0.5 - 32, y + 0.5 - 32)
    if r < 5:
        return (255, 255, 255, 255)
    if 11 <= r < 14:
        return (255, 70, 190, 255)
    if 21 <= r < 24:
        return (60, 240, 230, 255)
    return (0, 0, 0, 255)


def mask(x, y):
    base = (240, 140, 40)
    if x % 16 == 15 or y % 16 == 15:
        base = (250, 240, 220)
    if y >= 56:  # alpha ramp; cutoff 0.5 makes a sharp edge near x = 32
        return (*base, 255 * x / 63)
    cx, cy = (x // 16) * 16 + 7.5, (y // 16) * 16 + 7.5
    if math.hypot(x - cx, y - cy) < 4.5:
        return (120, 70, 20, 0)
    return (*base, 255)


def arrow(x, y):
    rgb = (225, 225, 225)
    if 12 <= y <= 19 and 5 <= x <= 20:
        rgb = (30, 50, 150)  # shaft pointing toward +U
    if 21 <= x <= 28 and abs(y - 15.5) <= (28 - x) + 0.5:
        rgb = (30, 50, 150)  # head
    if 2 <= x <= 6 and 2 <= y <= 6:
        rgb = (210, 30, 30)  # red: image top-left
    if 2 <= x <= 6 and 25 <= y <= 29:
        rgb = (30, 160, 50)  # green: image bottom-left
    return (*rgb, 255)


def stripes(x, y):
    if y % 16 == 0:
        return (220, 30, 30, 255)
    if x % 16 == 0:
        return (30, 60, 220, 255)
    value = 255 if (x + y) % 2 == 0 else 0
    return (value, value, value, 255)


def panel_v1(x, y):
    rgb = (245, 245, 245) if (x + y) % 8 < 3 else (40, 90, 220)
    if 4 <= x <= 7 and 4 <= y <= 27:
        rgb = (10, 10, 10)  # one bar: version 1
    return (*rgb, 255)


def panel_v2(x, y):
    ring = max(abs(x - 15.5), abs(y - 15.5))
    rgb = (230, 120, 30) if int(ring) % 6 < 3 else (20, 20, 20)
    if 4 <= y <= 27 and (4 <= x <= 6 or 9 <= x <= 11):
        rgb = (245, 245, 245)  # two bars: version 2
    return (*rgb, 255)


def sky(x, y):
    """Equirectangular sky, 128x64: +Y up, azimuth from u. Sun at 40 deg azimuth, 30 deg elevation."""
    azimuth = 360.0 * (x + 0.5) / 128
    elevation = 90.0 - 180.0 * (y + 0.5) / 64
    if elevation >= 0:
        t = (elevation / 90.0) ** 0.6
        rgb = [h + (z - h) * t for h, z in zip((235, 220, 195), (55, 105, 200))]
    else:
        t = min(1.0, -elevation / 60.0)
        rgb = [g + (n - g) * t for g, n in zip((110, 95, 80), (45, 40, 35))]
    if abs(elevation) <= 6:
        for center, color in ((0, (210, 40, 40)), (90, (40, 180, 60)), (180, (40, 70, 220)), (270, (230, 200, 40))):
            if abs((azimuth - center + 180) % 360 - 180) <= 5:
                rgb = color
    da = math.radians(azimuth - 40.0)
    sun = (math.cos(math.radians(elevation)) * math.cos(math.radians(30)) * math.cos(da)
           + math.sin(math.radians(elevation)) * math.sin(math.radians(30)))
    if sun > math.cos(math.radians(5)):
        rgb = (255, 250, 235)
    return (*rgb, 255)


def dim(x, y):
    return (40, 45, 60, 255)


TEXTURES = {
    "checker.png": (64, 64, checker), "mr.png": (64, 64, metallic_roughness), "normal.png": (64, 64, normal),
    "occlusion.png": (64, 64, occlusion), "emissive.png": (64, 64, emissive), "mask.png": (64, 64, mask),
    "arrow.png": (32, 32, arrow), "stripes.png": (128, 128, stripes), "panel-v1.png": (32, 32, panel_v1),
    "panel-v2.png": (32, 32, panel_v2), "sky.png": (128, 64, sky), "sky-dim.png": (32, 16, dim),
}


# ------------------------------------------------------------------ meshes

def sphere(rings=32, sectors=64):
    positions, normals, uvs = [], [], []
    for r in range(rings + 1):
        theta = math.pi * r / rings
        for s in range(sectors + 1):
            phi = 2 * math.pi * s / sectors
            p = (math.sin(theta) * math.cos(phi), math.cos(theta), math.sin(theta) * math.sin(phi))
            positions.append(p)
            normals.append(p)
            uvs.append((s / sectors, r / rings))
    indices = []
    for r in range(rings):
        for s in range(sectors):
            a, b, c, d = (r * (sectors + 1) + s, (r + 1) * (sectors + 1) + s,
                          (r + 1) * (sectors + 1) + s + 1, r * (sectors + 1) + s + 1)
            for tri in ((a, b, c), (a, c, d)):
                p0, p1, p2 = (positions[i] for i in tri)
                e1 = [p1[i] - p0[i] for i in range(3)]
                e2 = [p2[i] - p0[i] for i in range(3)]
                cross = (e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2],
                         e1[0] * e2[1] - e1[1] * e2[0])
                if sum(v * v for v in cross) < 1e-14:
                    continue  # degenerate pole triangle
                centre = [sum(positions[i][k] for i in tri) for k in range(3)]
                if sum(cross[k] * centre[k] for k in range(3)) < 0:
                    tri = (tri[0], tri[2], tri[1])  # outward counter-clockwise
                indices.extend(tri)
    return positions, normals, uvs, indices


def quad():
    # 2 x 2 m in XY facing +Z; glTF UV origin at the image's top-left.
    positions = [(-1, 1, 0), (1, 1, 0), (1, -1, 0), (-1, -1, 0)]
    return positions, [(0, 0, 1)] * 4, [(0, 0), (1, 0), (1, 1), (0, 1)], [3, 2, 1, 3, 1, 0]


def floor(tiles=8):
    # 1 x 1 m in XZ facing +Y, UV tiled; scaled per entity.
    positions = [(-0.5, 0, 0.5), (0.5, 0, 0.5), (0.5, 0, -0.5), (-0.5, 0, -0.5)]
    uvs = [(0, tiles), (tiles, tiles), (tiles, 0), (0, 0)]
    return positions, [(0, 1, 0)] * 4, uvs, [0, 1, 2, 0, 2, 3]


def cube():
    positions, normals, uvs, indices = [], [], [], []
    for axis in range(3):
        for sign in (1, -1):
            n = [0, 0, 0]
            n[axis] = sign
            u_axis, v_axis = [(1, 2), (2, 0), (0, 1)][axis]
            base = len(positions)
            for du, dv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                p = [0.0, 0.0, 0.0]
                p[axis] = 0.5 * sign
                p[u_axis] = 0.5 * du
                p[v_axis] = 0.5 * dv
                positions.append(tuple(p))
                normals.append(tuple(n))
                uvs.append(((du + 1) / 2, (1 - dv) / 2))
            for tri in ((0, 1, 2), (0, 2, 3)):
                t = [base + i for i in tri]
                p0, p1, p2 = (positions[i] for i in t)
                e1 = [p1[i] - p0[i] for i in range(3)]
                e2 = [p2[i] - p0[i] for i in range(3)]
                cross = (e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2],
                         e1[0] * e2[1] - e1[1] * e2[0])
                if sum(cross[k] * n[k] for k in range(3)) < 0:
                    t = [t[0], t[2], t[1]]
                indices.extend(t)
    return positions, normals, uvs, indices


MESHES = {"sphere": sphere, "quad": quad, "floor": floor, "cube": cube}


def write_mesh(directory, name, mesh):
    positions, normals, uvs, indices = mesh
    data = b"".join(struct.pack("<3f", *p) for p in positions)
    data += b"".join(struct.pack("<3f", *n) for n in normals)
    data += b"".join(struct.pack("<2f", *t) for t in uvs)
    data += b"".join(struct.pack("<I", i) for i in indices)
    (directory / f"{name}.bin").write_bytes(data)
    count = len(positions)
    views = [{"buffer": 0, "byteOffset": 0, "byteLength": 12 * count},
             {"buffer": 0, "byteOffset": 12 * count, "byteLength": 12 * count},
             {"buffer": 0, "byteOffset": 24 * count, "byteLength": 8 * count},
             {"buffer": 0, "byteOffset": 32 * count, "byteLength": 4 * len(indices)}]
    accessors = [{"bufferView": 0, "componentType": 5126, "count": count, "type": "VEC3",
                  "min": [min(p[i] for p in positions) for i in range(3)],
                  "max": [max(p[i] for p in positions) for i in range(3)]},
                 {"bufferView": 1, "componentType": 5126, "count": count, "type": "VEC3"},
                 {"bufferView": 2, "componentType": 5126, "count": count, "type": "VEC2"},
                 {"bufferView": 3, "componentType": 5125, "count": len(indices), "type": "SCALAR"}]
    return {"uri": f"{name}.bin", "byteLength": len(data)}, views, accessors


# ------------------------------------------------------------------ materials

def sampler(mag=LINEAR, minify=TRILINEAR, wrap=REPEAT):
    return {"magFilter": mag, "minFilter": minify, "wrapS": wrap, "wrapT": wrap}


def pbr(base=(1, 1, 1, 1), metallic=0.0, roughness=0.5, **more):
    value = {"baseColorFactor": list(base), "metallicFactor": metallic, "roughnessFactor": roughness}
    value.update(more)
    return value


# model -> (mesh, material, images [(uri, sampler)])
MODELS = {
    "sphere-checker": ("sphere", {"pbrMetallicRoughness": pbr(baseColorTexture={"index": 0})},
                       [("checker.png", sampler())]),
    "sphere-mr": ("sphere", {"pbrMetallicRoughness": pbr((0.95, 0.75, 0.35, 1), 1.0, 1.0,
                                                         metallicRoughnessTexture={"index": 0})},
                  [("mr.png", sampler())]),
    "sphere-chrome": ("sphere", {"pbrMetallicRoughness": pbr((0.97, 0.96, 0.94, 1), 1.0, 0.08)}, []),
    "sphere-red": ("sphere", {"pbrMetallicRoughness": pbr((0.75, 0.06, 0.05, 1), 0.0, 0.3)}, []),
    "sphere-glass": ("sphere", {"pbrMetallicRoughness": pbr((0.35, 0.6, 0.95, 0.4), 0.0, 0.05),
                                "alphaMode": "BLEND"}, []),
    "sphere-white": ("sphere", {"pbrMetallicRoughness": pbr((0.82, 0.82, 0.82, 1), 0.0, 0.55)}, []),
    "quad-normal": ("quad", {"pbrMetallicRoughness": pbr((0.7, 0.7, 0.72, 1), 0.0, 0.35),
                             "normalTexture": {"index": 0, "scale": 1.0}}, [("normal.png", sampler())]),
    "quad-occlusion": ("quad", {"pbrMetallicRoughness": pbr((0.85, 0.85, 0.85, 1), 0.0, 0.6),
                                "occlusionTexture": {"index": 0, "strength": 1.0}}, [("occlusion.png", sampler())]),
    "quad-emissive": ("quad", {"pbrMetallicRoughness": pbr((0.05, 0.05, 0.05, 1), 0.0, 0.5),
                               "emissiveTexture": {"index": 0}, "emissiveFactor": [1, 1, 1]},
                      [("emissive.png", sampler())]),
    "quad-mask": ("quad", {"pbrMetallicRoughness": pbr(baseColorTexture={"index": 0}), "alphaMode": "MASK",
                           "alphaCutoff": 0.5, "doubleSided": True}, [("mask.png", sampler())]),
    "quad-arrow-single": ("quad", {"pbrMetallicRoughness": pbr(baseColorTexture={"index": 0})},
                          [("arrow.png", sampler(NEAREST, NEAREST, CLAMP))]),
    "quad-arrow-double": ("quad", {"pbrMetallicRoughness": pbr(baseColorTexture={"index": 0}),
                                   "doubleSided": True}, [("arrow.png", sampler(NEAREST, NEAREST, CLAMP))]),
    "quad-panel": ("quad", {"pbrMetallicRoughness": pbr(roughness=0.45, baseColorTexture={"index": 0})},
                   [("panel.png", sampler(NEAREST, TRILINEAR, CLAMP))]),
    "floor-mip": ("floor", {"pbrMetallicRoughness": pbr(roughness=0.8, baseColorTexture={"index": 0})},
                  [("stripes.png", sampler(LINEAR, TRILINEAR, REPEAT))]),
    "floor-nomip": ("floor", {"pbrMetallicRoughness": pbr(roughness=0.8, baseColorTexture={"index": 0})},
                    [("stripes.png", sampler(LINEAR, LINEAR, REPEAT))]),
    "floor-gray": ("floor", {"pbrMetallicRoughness": pbr((0.5, 0.5, 0.5, 1), 0.0, 0.7)}, []),
    "cube-white": ("cube", {"pbrMetallicRoughness": pbr((0.8, 0.8, 0.8, 1), 0.0, 0.6)}, []),
    "cube-checker": ("cube", {"pbrMetallicRoughness": pbr(roughness=0.4, baseColorTexture={"index": 0})},
                     [("checker.png", sampler(NEAREST, NEAREST, CLAMP))]),
}


def gltf(mesh_files, name):
    mesh, material, images = MODELS[name]
    buffer, views, accessors = mesh_files[mesh]
    document = {"asset": {"version": "2.0", "generator": "incant handoff 0064 review fixture"},
                "buffers": [buffer], "bufferViews": views, "accessors": accessors,
                "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2},
                                            "indices": 3, "material": 0}]}],
                "materials": [material], "nodes": [{"mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0}
    if images:
        document["images"] = [{"uri": uri} for uri, _ in images]
        document["samplers"] = [s for _, s in images]
        document["textures"] = [{"source": i, "sampler": i} for i in range(len(images))]
    return document


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    out = Path(sys.argv[1])
    out.mkdir(parents=False, exist_ok=False)
    manifest = {"textures": {}, "meshes": {}, "models": {}}
    for name, (width, height, texel) in TEXTURES.items():
        data = pngio.write(out / name, *image(width, height, texel))
        manifest["textures"][name] = {"width": width, "height": height, "bytes": len(data)}
    mesh_files = {}
    for name, make in MESHES.items():
        mesh = make()
        mesh_files[name] = write_mesh(out, name, mesh)
        manifest["meshes"][name] = {"vertices": len(mesh[0]), "triangles": len(mesh[3]) // 3,
                                    "bin_bytes": mesh_files[name][0]["byteLength"]}
    for name in MODELS:
        text = json.dumps(gltf(mesh_files, name), indent=1) + "\n"
        (out / f"{name}.gltf").write_text(text)
        manifest["models"][name] = {"mesh": MODELS[name][0], "images": [u for u, _ in MODELS[name][2]],
                                    "material": MODELS[name][1]}
    # panel.png starts as version 1; the reimport step replaces its bytes with panel-v2.png.
    (out / "panel.png").write_bytes((out / "panel-v1.png").read_bytes())
    manifest["files"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir())}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(json.dumps({"files": len(manifest["files"])}))


if __name__ == "__main__":
    main()
