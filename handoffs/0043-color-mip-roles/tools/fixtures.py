"""CPU-only fixture art, glTF sources and scene specs for handoff 0043.

No engine calls. Original 0041 art and helpers come from the byte-identical Claude
0041 reference (references/0041-fixtures.py); its SHA-256 is checked and exactly
those verified bytes are executed. Nothing in it is edited or copied. 0043 adds
only the variants this delta needs:

* materials: the 0041 64x64 card (F glyph, alpha-0 teal triangle, alpha
  0.2/0.4/0.6/0.8 stripes) with its own alpha ("mixed"), the same RGB with
  all-zero alpha, with all-one alpha (control) and as a 3-channel RGB PNG
  (ordinary input). shared.gltf binds the one mixed image to OPAQUE, MASK and
  BLEND materials (the 0041 card material). controls.gltf holds the alpha controls,
  the 0041 dome normal map (Normal control) and a metallic-roughness map carrying
  the card's alpha (Linear control). emissive.gltf binds the cards as emissive maps
  on OPAQUE and BLEND (alpha 0.7) materials. Every cell is an 80 px quad in the
  ortho board whose UVs repeat k = 1, 2 or 6 times (REPEAT sampler), so its level
  of detail is -0.32, 0.68 or 2.26 at a fixed, pixel-aligned footprint.
* environment: one 1024x512 Color equirectangular sky (alpha 0, 0.5 and 1 zones
  over opaque ground) used by the EnvironmentLight and by a SpriteRenderer, the
  0041 sprite atlas as an atlas coverage control and unlit stripe sprites behind.
* envcontrol: the same scene with an all-alpha-one copy of the sky.
"""

import hashlib
import json
import math
import struct
import sys
import types
import zlib
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
PACKET = TOOLS.parent
ROOT = TOOLS.parents[2]
REFERENCE = (PACKET / "references/0041-fixtures.py",
             "46d9237deb0b427aec2d74f57935962bec175e546e3933bfebaba2a318ca9a7f")
PNGIO = (ROOT / "handoffs/0032-sprite-rendering/tools/pngio.py",
         "5b73bef8fefbb5c3b89515d677d8fa71a2a44fc1cb9980cb4fdb9d10ddde9e86")


def _verified(name, source):
    path, digest = source
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise SystemExit(f"{path.relative_to(ROOT)} does not match its recorded SHA-256")
    module = types.ModuleType(name)
    module.__file__ = str(path)
    return module, compile(data, str(path), "exec")


def _load_reference():
    """Run the verified 0041 reference with 0032's pngio bound only while it imports.

    The 0040 harness imports a different module also named `pngio`; restoring
    sys.modules and sys.path keeps the two from shadowing each other.
    """
    png, code = _verified("pngio_0032", PNGIO)
    exec(code, png.__dict__)
    saved_module, saved_path = sys.modules.get("pngio"), list(sys.path)
    sys.modules["pngio"] = png
    try:
        reference, code = _verified("fixtures_0041", REFERENCE)
        exec(code, reference.__dict__)
    finally:
        sys.path[:] = saved_path
        if saved_module is None:
            sys.modules.pop("pngio", None)
        else:
            sys.modules["pngio"] = saved_module
    return reference, png


REF, PNG = _load_reference()

# ------------------------------------------------------------------ art --

CARD = 64
REPEAT = {"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}


def card(alpha):
    """0041 card RGB with its own alpha ('mixed') or a constant 'zero'/'one' alpha."""
    pixels = REF.card_base_pixels()
    if alpha == "mixed":
        return pixels
    value = {"zero": 0, "one": 255}[alpha]
    return [(r, g, b, value) for r, g, b, _ in pixels]


def encode_rgb(width, height, pixels):
    """3-channel PNG, adapted from 0032 pngio.encode_rgba (color type 2, filter 0)."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for p in pixels[y * width:(y + 1) * width]:
            raw.extend(bytes(p[:3]))
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (PNG.SIGNATURE + PNG._chunk(b"IHDR", header) + PNG._chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + PNG._chunk(b"IEND", b""))


def metallic_roughness():
    """Linear control: metal F glyph and triangle (B=1, G=0.35), dielectric elsewhere
    (G=0.8). Alpha copies the mixed card, so a coverage filter would be visible."""
    alpha = [p[3] for p in REF.card_base_pixels()]
    out = []
    for y in range(CARD):
        for x in range(CARD):
            metal = REF.f_glyph(x, y) or REF.triangle_hole(x, y)
            out.append((255, 90 if metal else 205, 255 if metal else 0, alpha[y * CARD + x]))
    return out


SKY_SIZE = (1024, 512)  # width/4 > 128-px cube face, so environment decode reads mip 1
ZENITH, HORIZON, GROUND, SUN = (40, 90, 200), (170, 200, 235), (120, 96, 70), (255, 240, 205)
SUN_CENTRE, SUN_RADIUS = (576.0, 224.0), 20.0
HAZE = (512, 768)  # sky columns with alpha 0.5; other sky columns have alpha 0
SKY_REGION = [448, 192, 256, 128]  # sprite crop: zone boundary, sun, horizon, ground


def sky(alpha_one=False):
    """Equirectangular sky (+Y at the top row, +X at the centre column)."""
    w, h = SKY_SIZE
    out = []
    for y in range(h):
        t = y / (h // 2 - 1)
        row = tuple(round(z + (o - z) * t) for z, o in zip(ZENITH, HORIZON))
        for x in range(w):
            if y >= h // 2:
                rgb, a = GROUND, 255
            elif math.hypot(x + 0.5 - SUN_CENTRE[0], y + 0.5 - SUN_CENTRE[1]) <= SUN_RADIUS:
                rgb, a = SUN, 255
            else:
                rgb, a = row, 128 if HAZE[0] <= x < HAZE[1] else 0
            out.append((*rgb, 255 if alpha_one else a))
    return out


# --------------------------------------------------------------- layout --

CELL, PITCH, PX_PER_UNIT = 2.0, 2.2, 40  # 80 px cells on an 88 px pitch in the ortho board
TILES = (1, 2, 6)  # LOD = log2(64 k / 80) = -0.32, 0.68, 2.26
BASE = ("O-mixed", "O-zero", "O-one", "O-rgb", "M-mixed", "B-mixed", "N-control", "L-control")
EMISSIVE = ("E-O-mixed", "E-O-zero", "E-O-one", "E-B-mixed", "E-B-zero", "E-B-one")
EMISSIVE_BLEND_ALPHA = 0.7
STRIPE, STRIPE_ORIGIN = 0.55, -12.0  # 22 px stripes; every cell starts on a dark stripe
BACKDROP_Z, BACKDROP_Y = -1.0, (-60.0, 12.0)  # large enough for the oblique view's lower rays
BACKDROP_J = range(-64, 108)  # stripe indices: x from -47.2 to 47.4


def cells():
    """name -> group, column, row, tiling k and world centre for all 42 cells."""
    out = {}
    for group, names, top in (("base", BASE, 2.5), ("emissive", EMISSIVE, -0.5)):
        for column, name in enumerate(names):
            for row, k in enumerate(TILES):
                x = (column - (len(names) - 1) / 2) * PITCH
                out[f"{name} k{k}"] = {"group": group, "material": name, "column": column, "row": row, "k": k,
                                       "centre": [round(x, 9), round((top - row) * PITCH, 9), 0.0]}
    return out


def tiled_quad(centre, k):
    """Adapted from 0041 card_quad: +Z facing, glTF top-left UVs, repeated k times."""
    x, y, _ = centre
    h = CELL / 2
    return {"positions": [(x - h, y - h, 0.0), (x + h, y - h, 0.0), (x + h, y + h, 0.0), (x - h, y + h, 0.0)],
            "normals": [(0.0, 0.0, 1.0)] * 4,
            "uvs": [(0.0, float(k)), (float(k), float(k)), (float(k), 0.0), (0.0, 0.0)],
            "indices": [0, 1, 2, 0, 2, 3]}


def color_material(name, alpha, texture, *, normal=None, mr=None):
    """0041 card material values with explicit texture indices."""
    material = {"name": name, "pbrMetallicRoughness": {
        "baseColorFactor": [1.0, 1.0, 1.0, 1.0], "baseColorTexture": {"index": texture},
        "metallicFactor": 0.0, "roughnessFactor": 0.65}, "alphaMode": alpha, "doubleSided": False}
    if normal is not None:
        material["normalTexture"] = {"index": normal, "scale": 1.0}
    if mr is not None:
        material["pbrMetallicRoughness"].update({"metallicRoughnessTexture": {"index": mr},
                                                 "metallicFactor": 1.0, "roughnessFactor": 1.0})
    return material


def emissive_material(name, alpha, texture):
    a = 1.0 if alpha == "OPAQUE" else EMISSIVE_BLEND_ALPHA
    return {"name": name, "pbrMetallicRoughness": {"baseColorFactor": [0.0, 0.0, 0.0, a],
                                                   "metallicFactor": 0.0, "roughnessFactor": 1.0},
            "emissiveTexture": {"index": texture}, "emissiveFactor": [1.0, 1.0, 1.0],
            "alphaMode": alpha, "doubleSided": False}


def backdrop():
    """Lit stripes behind the board; boundaries at x = -12 + 0.55 j fall on 22 px."""
    dark, light = [], []
    for j in BACKDROP_J:
        x0 = STRIPE_ORIGIN + STRIPE * j
        (dark if j % 2 == 0 else light).append(REF.rect_xy(x0, x0 + STRIPE, BACKDROP_Y[0], BACKDROP_Y[1], BACKDROP_Z))
    return ([REF.with_material(REF.merge(dark), 0), REF.with_material(REF.merge(light), 1)],
            [REF.flat_material("backdrop dark", (0.03, 0.03, 0.04), 0.9),
             REF.flat_material("backdrop light", (0.32, 0.32, 0.36), 0.9)])


def write_png(path, width, height, pixels):
    PNG.write_rgba(path, width, height, pixels)


def materials_sources(directory):
    write_png(directory / "card_mixed.png", CARD, CARD, card("mixed"))
    write_png(directory / "card_zero.png", CARD, CARD, card("zero"))
    write_png(directory / "card_one.png", CARD, CARD, card("one"))
    (directory / "card_rgb.png").write_bytes(encode_rgb(CARD, CARD, card("one")))
    write_png(directory / "card_normal.png", CARD, CARD, REF.card_normal_pixels())
    write_png(directory / "card_mr.png", CARD, CARD, metallic_roughness())
    write_png(directory / "env_uniform.png", 2, 1, REF.uniform_pixels(64))
    layout = cells()

    def quads(name, material):
        return [REF.with_material(tiled_quad(layout[f"{name} k{k}"]["centre"], k), material) for k in TILES]

    shared = [REF.card_material(alpha, False, normal=False, name=f"shared card {alpha.lower()}")
              for alpha in ("OPAQUE", "MASK", "BLEND")]
    REF.write_gltf(directory, "shared", quads("O-mixed", 0) + quads("M-mixed", 1) + quads("B-mixed", 2), shared,
                   ["card_mixed.png"], REPEAT)
    controls = [color_material("opaque all-zero alpha", "OPAQUE", 0),
                color_material("opaque all-one alpha", "OPAQUE", 1),
                color_material("opaque rgb", "OPAQUE", 2),
                color_material("normal control", "OPAQUE", 1, normal=3),
                color_material("linear control", "OPAQUE", 1, mr=4)]
    REF.write_gltf(directory, "controls", quads("O-zero", 0) + quads("O-one", 1) + quads("O-rgb", 2)
                   + quads("N-control", 3) + quads("L-control", 4), controls,
                   ["card_zero.png", "card_one.png", "card_rgb.png", "card_normal.png", "card_mr.png"], REPEAT)
    emissive, prims = [], []
    for index, name in enumerate(EMISSIVE):
        _, alpha, source = name.split("-")
        emissive.append(emissive_material(f"emissive {alpha.lower()} {source}",
                                          "OPAQUE" if alpha == "O" else "BLEND", ("mixed", "zero", "one").index(source)))
        prims += quads(name, index)
    REF.write_gltf(directory, "emissive", prims, emissive, ["card_mixed.png", "card_zero.png", "card_one.png"],
                   REPEAT)
    prims, materials = backdrop()
    REF.write_gltf(directory, "props", prims, materials)


MATERIALS_IMPORTS = [("env_uniform.png", "linear"), ("props.gltf", None), ("shared.gltf", None),
                     ("controls.gltf", None), ("emissive.gltf", None)]


def materials_scene(assets):
    p = REF.Project("materials", "01JA43CMM0")
    p.entity("Key light", {"DirectionalLight": {"color": [1.0, 1.0, 1.0], "intensity": 3.0},
                           "Transform": REF.transform((0.0, 6.0, 6.0), REF.look_rotation(tuple(-c for c in REF.KEY_L)))})
    p.entity("Uniform environment", {"EnvironmentLight": {"texture": assets["env_uniform.png"], "intensity": 1.0,
                                                          "rotation_degrees": 0.0}})
    for name, source in (("Backdrop", "props.gltf"), ("Shared OPAQUE MASK BLEND", "shared.gltf"),
                         ("Alpha and role controls", "controls.gltf"), ("Emissive maps", "emissive.gltf")):
        p.entity(name, {"MeshRenderer": REF.mesh(assets[source]), "Transform": REF.transform((0.0, 0.0, 0.0))})
    p.meta.update(cells())
    p.camera(REF.View("board", (0.0, 0.0, 10.0), [0.0, 0.0, 0.0, 1.0], fov=40.0, far=40.0,
                      ortho_half_height=540 / PX_PER_UNIT / 2))
    eye, target = (0.0, 14.5, 10.5), (0.0, -0.2, 0.0)
    p.camera(REF.View("oblique", eye, REF.look_rotation(REF.norm(REF.sub(target, eye))), fov=48.0, far=60.0))
    return p


# --- environment and its all-alpha-one control --------------------------

SPHERES = {"matte white": ((-1.45, 0.95, 0.0), (0.8, 0.8, 0.8), 0.9, 0.0),
           "polished metal": ((1.45, 0.95, 0.0), (0.95, 0.95, 0.95), 0.12, 1.0)}
SPRITE_Y, SKY_SPRITE_SIZE = -1.45, (4.0, 2.0)
STRIPES = (-4.5, 4.5, -2.65, -0.25, 0.25)  # x0, x1, y0, y1, stripe width (unlit sprites)


def environment_sources(directory, alpha_one=False):
    name = "sky_one.png" if alpha_one else "sky.png"
    write_png(directory / name, *SKY_SIZE, sky(alpha_one))
    write_png(directory / "solid.png", 4, 4, [(255, 255, 255, 255)] * 16)
    write_png(directory / "flag.png", 16, 16, REF.outlined_glyph("R", (60, 200, 230, 255), (16, 30, 44, 255)))
    write_png(directory / "panel_b.png", 24, 24, REF.panel_pixels(
        ((215, 70, 200, 255), (60, 210, 220, 255), (150, 220, 60, 255), (245, 245, 245, 255)),
        (80, 80, 92, 255), (200, 200, 215, 255), (50, 40, 70, 255)))
    (directory / "sprites.atlas.json").write_text(json.dumps(REF.atlas_descriptor(), indent=1), encoding="utf-8")
    prims, materials = [], []
    for index, (label, (centre, rgb, roughness, metallic)) in enumerate(SPHERES.items()):
        prims.append(REF.with_material(REF.sphere(0.95, centre), index))
        materials.append(REF.flat_material(label, rgb, roughness, metallic=metallic))
    REF.write_gltf(directory, "spheres", prims, materials)


def environment_imports(alpha_one=False):
    return [("sky_one.png" if alpha_one else "sky.png", "color"), ("solid.png", "color"),
            ("sprites.atlas.json", "color"), ("spheres.gltf", None)]


def environment_scene(assets, alpha_one=False):
    sky_source = "sky_one.png" if alpha_one else "sky.png"
    p = REF.Project("envcontrol" if alpha_one else "environment", "01JA43CMC0" if alpha_one else "01JA43CME0")
    # An authored zero-intensity light disables the renderer's preview key, so the
    # spheres show environment radiance only.
    p.entity("Zero key", {"DirectionalLight": {"color": [1.0, 1.0, 1.0], "intensity": 0.0},
                          "Transform": REF.transform((0.0, 5.0, 0.0), REF.quat_axis((1, 0, 0), -90))})
    p.entity("Sky environment", {"EnvironmentLight": {"texture": assets[sky_source], "intensity": 1.5,
                                                      "rotation_degrees": 0.0}})
    p.entity("Spheres", {"MeshRenderer": REF.mesh(assets["spheres.gltf"]), "Transform": REF.transform((0.0, 0.0, 0.0))})

    def sprite(name, texture, region, centre, size, *, tint=(1.0, 1.0, 1.0, 1.0), layer=0, sampling="nearest"):
        p.entity(name, {"SpriteRenderer": {"texture": texture, "region": region, "size": list(size),
                                           "pivot": [0.5, 0.5], "tint": list(tint), "flip_x": False,
                                           "flip_y": False, "layer": layer, "order": 0, "sampling": sampling,
                                           "mode": {"kind": "simple"}},
                        "Transform": REF.transform(centre)})
        p.meta[name] = {"centre": list(centre), "size": list(size)}

    x0, x1, y0, y1, width = STRIPES
    for k in range(round((x1 - x0) / width)):
        dark = k % 2 == 0
        sprite(f"Stripe {k}", assets["solid.png"], [0, 0, 4, 4], (x0 + width * (k + 0.5), (y0 + y1) / 2, 0.0),
               (width, y1 - y0), tint=(0.02, 0.02, 0.02, 1.0) if dark else (0.55, 0.55, 0.55, 1.0), layer=-10)
    sprite("Sky sprite", assets[sky_source], SKY_REGION, (0.0, SPRITE_Y, 0.0), SKY_SPRITE_SIZE, sampling="linear")
    sprite("Atlas flag", assets["sprites.atlas.json"], {"frame": REF.FRAME_FLAG}, (-3.35, SPRITE_Y, 0.0), (1.4, 1.4))
    sprite("Atlas panel", assets["sprites.atlas.json"], {"frame": REF.FRAME_PANEL}, (3.35, SPRITE_Y, 0.0), (1.4, 1.4))
    for label, (centre, *_rest) in SPHERES.items():
        p.meta[label] = {"centre": list(centre), "radius": 0.95}
    eye, target = (0.0, 0.1, 8.6), (0.0, -0.15, 0.0)
    p.camera(REF.View("view", eye, REF.look_rotation(REF.norm(REF.sub(target, eye))), fov=38.0, far=40.0))
    return p


PROJECTS = {
    "materials": (materials_sources, MATERIALS_IMPORTS, materials_scene),
    "environment": (environment_sources, environment_imports(), environment_scene),
    "envcontrol": (lambda d: environment_sources(d, alpha_one=True), environment_imports(alpha_one=True),
                   lambda a: environment_scene(a, alpha_one=True)),
}

# ----------------------------------------------------------- framing check --


def framing(project):
    """Analytic pixel bounds of the reviewed content; raises if any of it leaves a frame."""
    report = {}
    if project.name == "materials":
        corners = []
        for cell in project.meta.values():
            x, y, z = cell["centre"]
            corners += [(x + sx * CELL / 2, y + sy * CELL / 2, z) for sx in (-1, 1) for sy in (-1, 1)]
        for view in ("board", "oblique"):
            report[view] = REF.bounds(project.views[view], corners)
        # Ortho board: the visible rectangle is 24 x 13.5 units about the origin.
        x0, x1 = STRIPE_ORIGIN + STRIPE * BACKDROP_J[0], STRIPE_ORIGIN + STRIPE * (BACKDROP_J[-1] + 1)
        report["board backdrop covers frame"] = x0 <= -12 and x1 >= 12
        # REF.covers casts perspective rays onto the plane z = z0 (0041 helper).
        report["oblique backdrop covers frame"] = REF.covers(project.views["oblique"], [
            (x0, BACKDROP_Y[0], BACKDROP_Z), (x1, BACKDROP_Y[1], BACKDROP_Z)])
        report["board pixel rectangles"] = {name: cell_rect(cell) for name, cell in project.meta.items()}
    else:
        view = project.views["view"]
        points = []
        for meta in project.meta.values():
            x, y, z = meta["centre"]
            if "radius" in meta:
                r = meta["radius"]
                points += [(x + sx * r, y + sy * r, z) for sx in (-1, 1) for sy in (-1, 1)]
            else:
                w, h = meta["size"]
                points += [(x + sx * w / 2, y + sy * h / 2, z) for sx in (-1, 1) for sy in (-1, 1)]
        report["view"] = REF.bounds(view, points)
    return report


def cell_rect(cell):
    """Exact ortho board rectangle [left, top, right, bottom) in pixels."""
    x, y, _ = cell["centre"]
    left = 480 + PX_PER_UNIT * (x - CELL / 2)
    top = 270 - PX_PER_UNIT * (y + CELL / 2)
    rect = [left, top, left + PX_PER_UNIT * CELL, top + PX_PER_UNIT * CELL]
    if any(abs(v - round(v)) > 1e-6 for v in rect):
        raise SystemExit(f"cell is not pixel aligned: {rect}")
    return [round(v) for v in rect]


def lod(k):
    return math.log2(CARD * k / (PX_PER_UNIT * CELL))


if __name__ == "__main__":
    out = {}
    for project_name, (_, imports, scene) in PROJECTS.items():
        placeholder = {source: f"01JA43CMX0{index:016d}" for index, (source, _) in enumerate(imports)}
        spec = scene(placeholder)
        report = framing(spec)
        report.pop("board pixel rectangles", None)
        out[project_name] = report
    out["lod"] = {f"k{k}": round(lod(k), 3) for k in TILES}
    print(json.dumps(out, indent=1))
