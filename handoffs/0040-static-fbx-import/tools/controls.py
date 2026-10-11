"""Independently authored glTF controls for handoff 0040 (no engine code reused).

- axes: 1 m red +X, green +Y and blue +Z bars from the origin.
- ruler: 1 m bar along +X, ten alternating 10 cm white/black segments.
- relief: real geometry matching the normal map's raised button (35-degree bevels),
  so its bevel shading is the geometric reference for the decoded normal map.
- quad_*: a 1 m square facing +Z whose UVs cover the Maya cube's front island
  (glTF top-left UV origin). quad_normal_explicit carries TANGENT (1,0,0,+1): by the
  glTF 2.0 definition bitangent = cross(normal, tangent.xyz) * w = +Y, the image's
  up direction on this quad. quad_normal_generated omits TANGENT, so the engine's
  own tangent generation is used. quad_face_art carries the colour art.
"""
import math

import art
import gltfio


def _box(x0, y0, z0, x1, y1, z1):
    faces = [((1, 0, 0), [(x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1)]),
             ((-1, 0, 0), [(x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0)]),
             ((0, 1, 0), [(x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0)]),
             ((0, -1, 0), [(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)]),
             ((0, 0, 1), [(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)]),
             ((0, 0, -1), [(x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0)])]
    return _quads(faces)


def _quads(faces, prim=None):
    prim = prim or {"positions": [], "normals": [], "indices": []}
    for normal, corners in faces:
        base = len(prim["positions"])
        prim["positions"] += [list(map(float, c)) for c in corners]
        prim["normals"] += [list(map(float, normal))] * len(corners)
        prim["indices"] += [base + i for i in ([0, 1, 2, 2, 3, 0] if len(corners) == 4 else [0, 1, 2])]
    return prim


def axes():
    glb = gltfio.Glb()
    t = 0.02
    prims = []
    for color, box in [((0.85, 0.08, 0.06, 1), (0, -t, -t, 1, t, t)), ((0.1, 0.7, 0.12, 1), (-t, 0, -t, t, 1, t)),
                       ((0.08, 0.2, 0.9, 1), (-t, -t, 0, t, t, 1)), ((0.9, 0.9, 0.9, 1), (-2 * t, -2 * t, -2 * t, 2 * t, 2 * t, 2 * t))]:
        prim = _box(*box)
        prim["material"] = glb.material("axis", base=color)
        prims.append(prim)
    glb.root(glb.node("axes", mesh=glb.mesh("axes", prims)))
    return glb.bytes()


def ruler():
    glb = gltfio.Glb()
    white, black = glb.material("white", base=(0.9, 0.9, 0.9, 1)), glb.material("black", base=(0.03, 0.03, 0.03, 1))
    prims = {white: None, black: None}
    for i in range(10):
        mat = white if i % 2 == 0 else black
        prims[mat] = _box(i * 0.1, 0, -0.015, (i + 1) * 0.1, 0.03, 0.015) if prims[mat] is None else _merge(prims[mat], _box(i * 0.1, 0, -0.015, (i + 1) * 0.1, 0.03, 0.015))
    out = []
    for mat, prim in prims.items():
        prim["material"] = mat
        out.append(prim)
    glb.root(glb.node("ruler 1 m", mesh=glb.mesh("ruler", out)))
    return glb.bytes()


def _merge(a, b):
    base = len(a["positions"])
    a["positions"] += b["positions"]
    a["normals"] += b["normals"]
    a["indices"] += [base + i for i in b["indices"]]
    return a


def relief():
    """Plate in z=0 (normal +Z) with the raised button; footprint matches one UV island."""
    glb = gltfio.Glb()
    outer = art.OUTER_HALF / art.ISLAND
    inner = art.TOP_HALF / art.ISLAND
    height = (outer - inner) * math.tan(math.radians(art.BEVEL_DEGREES))
    s, c = math.sin(math.radians(art.BEVEL_DEGREES)), math.cos(math.radians(art.BEVEL_DEGREES))
    h = 0.5
    o, i, z = outer, inner, height
    faces = [
        ((0, 0, 1), [(-h, -h, 0), (h, -h, 0), (h, -o, 0), (-h, -o, 0)]),   # plate ring: bottom strip
        ((0, 0, 1), [(-h, o, 0), (h, o, 0), (h, h, 0), (-h, h, 0)]),       # top strip
        ((0, 0, 1), [(-h, -o, 0), (-o, -o, 0), (-o, o, 0), (-h, o, 0)]),   # left strip
        ((0, 0, 1), [(o, -o, 0), (h, -o, 0), (h, o, 0), (o, o, 0)]),       # right strip
        ((0, s, c), [(-o, o, 0), (o, o, 0), (i, i, z), (-i, i, z)][::-1]),  # top bevel faces +Y
        ((0, -s, c), [(-o, -o, 0), (-i, -i, z), (i, -i, z), (o, -o, 0)][::-1]),
        ((s, 0, c), [(o, -o, 0), (i, -i, z), (i, i, z), (o, o, 0)][::-1]),
        ((-s, 0, c), [(-o, -o, 0), (-o, o, 0), (-i, i, z), (-i, -i, z)][::-1]),
        ((0, 0, 1), [(-i, -i, z), (i, -i, z), (i, i, z), (-i, i, z)]),
    ]
    prim = _quads(faces)
    # Orient every bevel counter-clockwise when seen from +Z (front faces).
    prim["material"] = glb.material("relief", base=(0.8, 0.8, 0.8, 1))
    glb.root(glb.node("relief", mesh=glb.mesh("relief", [prim])))
    return glb.bytes(), {"outer_half": outer, "inner_half": inner, "height": height}


def _quad_prim(tangent):
    # Front island: u 0.375..0.625; glTF v = 1 - FBX v, so FBX v 0..0.25 -> 1.0..0.75.
    prim = {"positions": [[-0.5, -0.5, 0], [0.5, -0.5, 0], [0.5, 0.5, 0], [-0.5, 0.5, 0]],
            "normals": [[0, 0, 1]] * 4, "uvs": [[0.375, 1.0], [0.625, 1.0], [0.625, 0.75], [0.375, 0.75]],
            "indices": [0, 1, 2, 2, 3, 0]}
    if tangent:
        prim["tangents"] = [[1.0, 0.0, 0.0, 1.0]] * 4
    return prim


def quad_normal(normal_png, explicit):
    glb = gltfio.Glb()
    prim = _quad_prim(explicit)
    prim["material"] = glb.material("bevel_normal", base=(0.8, 0.8, 0.8, 1), normal_texture=glb.texture(normal_png))
    glb.root(glb.node("quad", mesh=glb.mesh("quad", [prim])))
    return glb.bytes()


def quad_face_art(color_png):
    glb = gltfio.Glb()
    prim = _quad_prim(False)
    prim["material"] = glb.material("face_art", base=(0.8, 0.8, 0.8, 1), base_texture=glb.texture(color_png))
    glb.root(glb.node("quad", mesh=glb.mesh("quad", [prim])))
    return glb.bytes()


def material_quads(color_png):
    """Front-island quads at x = 0, 1.6, 3.2, 4.8 m with the expected glTF values of
    the four Lambert cases (diffuse x factor, emissive map, constant emissive, alpha 0.5)."""
    glb = gltfio.Glb()
    texture = glb.texture(color_png)
    cases = [glb.material("diffuse_x_factor", base=(0.1, 0.25, 0.45, 1)),
             glb.material("emissive_map", base=(0, 0, 0, 1), emissive=(1, 1, 1), emissive_texture=texture),
             glb.material("emissive_constant", base=(0.2, 0.2, 0.2, 1), emissive=(0.9, 0.5, 0.1)),
             glb.material("opacity_half", base=(0.8, 0.8, 0.8, 0.5), alpha_mode="BLEND")]
    nodes = []
    for i, material in enumerate(cases):
        prim = _quad_prim(False)
        prim["material"] = material
        nodes.append(glb.node(f"case {i}", translation=(1.6 * i, 0, 0), mesh=glb.mesh(f"case {i}", [prim])))
    glb.root(glb.node("material quads", children=nodes))
    return glb.bytes()


def backdrop(color=(0.9, 0.35, 0.1, 1), width=1.4, height=3.0):
    """Uniform plane behind the opacity case, so FBX and control blend over the same colour."""
    glb = gltfio.Glb()
    w, h = width / 2, height / 2
    prim = {"positions": [[-w, -h, 0], [w, -h, 0], [w, h, 0], [-w, h, 0]], "normals": [[0, 0, 1]] * 4,
            "indices": [0, 1, 2, 2, 3, 0], "material": glb.material("backdrop", base=color)}
    glb.root(glb.node("backdrop", mesh=glb.mesh("backdrop", [prim])))
    return glb.bytes()
