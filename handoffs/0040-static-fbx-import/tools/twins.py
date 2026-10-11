"""Independent glTF "twins" of static FBX fixtures for handoff 0040.

A twin re-expresses what the FBX file states, using the published FBX transform
semantics rather than the engine importer: per model T * R(rotation order) * S,
the geometric transform Tg * Rg(XYZ) * Sg on a helper child that is not inherited,
the declared axis system and UnitScaleFactor on one root node, per-polygon
material slots as separate primitives, and the file's own per-corner normals.
Two encodings are taken from documented sources, not from engine code:
 - quads split on the shorter diagonal unless that folds the quad (the parser's
   documented rule, so planar/non-planar quads triangulate the same way);
 - Blender's FBX exporter encodes roughness as Shininess = (10 * (1 - r))^2 and
   metallic as ReflectionFactor (also ufbx's documented Blender mapping).
With textures=True, embedded colour/normal maps are carried over, UV0 V is flipped to
glTF's top-left origin, and TANGENT comes from the file's own exported tangent and
binormal layers (w = sign of cross(normal, tangent) . binormal), i.e. Maya's frame.
Pivots, offsets, pre/post rotations and non-RSrs inheritance are refused, not guessed.
"""
import base64
import math

import fbx_probe
import gltfio
from scene import normalize, quat_from_basis, quat_mul, quat_axis_angle

ORDERS = ["XYZ", "XZY", "YZX", "YXZ", "ZXY", "ZYX"]
AXES = {"X": (1, 0, 0), "Y": (0, 1, 0), "Z": (0, 0, 1)}


def euler_quat(degrees, order=0):
    """FBX Euler: the first letter of the order is applied first."""
    letters = ORDERS[order]
    q = [0.0, 0.0, 0.0, 1.0]
    for letter in letters:
        angle = degrees["XYZ".index(letter)]
        q = quat_mul(quat_axis_angle(AXES[letter], angle), q)
    return q


def axis_conversion(settings):
    """Rotation taking the file's (right, up, front) axes to right-handed Y-up +Z-front."""
    def axis(index, sign):
        v = [0.0, 0.0, 0.0]
        v[int(index)] = float(sign)
        return v
    right = axis(settings["CoordAxis"], settings["CoordAxisSign"])
    up = axis(settings["UpAxis"], settings["UpAxisSign"])
    front = axis(settings["FrontAxis"], settings["FrontAxisSign"])
    # M * right = +X, M * up = +Y, M * front = +Z, so M's rows are right, up, front.
    m_cols = [[right[0], up[0], front[0]], [right[1], up[1], front[1]], [right[2], up[2], front[2]]]
    det = (right[0] * (up[1] * front[2] - up[2] * front[1]) - right[1] * (up[0] * front[2] - up[2] * front[0])
           + right[2] * (up[0] * front[1] - up[1] * front[0]))
    if det <= 0:
        raise NotImplementedError("left-handed source axes")
    return quat_from_basis(*m_cols)


def _props(node, template):
    merged = dict(template)
    merged.update(fbx_probe._props70(node))
    return merged


def _vec(props, name, default):
    value = props.get(name)
    return [float(c) for c in value[:3]] if value else list(default)


def _geometry(obj):
    verts = fbx_probe._array(obj.find("Vertices"))
    positions = [[float(c) for c in verts[i:i + 3]] for i in range(0, len(verts), 3)]
    polys, corners, poly = [], [], []
    for n, idx in enumerate(fbx_probe._array(obj.find("PolygonVertexIndex"))):
        poly.append((n, idx if idx >= 0 else ~idx))
        if idx < 0:
            polys.append(poly)
            poly = []
    layer = obj.find("LayerElementNormal")
    normals = None
    if layer is not None:
        mapping = layer.find("MappingInformationType").props[0]
        reference = layer.find("ReferenceInformationType").props[0]
        data = fbx_probe._array(layer.find("Normals"))
        data = [[float(c) for c in data[i:i + 3]] for i in range(0, len(data), 3)]
        if reference == "IndexToDirect":
            index = fbx_probe._array(layer.find("NormalsIndex"))
            data = [data[i] for i in index]
        if mapping == "ByPolygonVertex":
            normals = lambda corner, vertex: data[corner]  # noqa: E731
        elif mapping in ("ByVertice", "ByVertex"):
            normals = lambda corner, vertex: data[vertex]  # noqa: E731
        else:
            raise NotImplementedError(f"normal mapping {mapping}")
    layers = {name: _corner_layer(obj, element, data, size)
              for name, element, data, size in [("uv", "LayerElementUV", "UV", 2),
                                                  ("tangent", "LayerElementTangent", "Tangents", 3),
                                                  ("binormal", "LayerElementBinormal", "Binormals", 3)]}
    mat_layer = obj.find("LayerElementMaterial")
    slots = [0] * len(polys)
    if mat_layer is not None:
        mapping = mat_layer.find("MappingInformationType").props[0]
        values = fbx_probe._array(mat_layer.find("Materials"))
        slots = list(values) if mapping == "ByPolygon" else [values[0]] * len(polys)
    return positions, polys, normals, slots, layers


def _corner_layer(obj, element, data_name, size):
    """Per-polygon-vertex layer (Direct or IndexToDirect), or None when absent."""
    layer = obj.find(element)
    if layer is None:
        return None
    if layer.find("MappingInformationType").props[0] != "ByPolygonVertex":
        raise NotImplementedError(f"{element} mapping")
    data = fbx_probe._array(layer.find(data_name))
    data = [[float(c) for c in data[i:i + size]] for i in range(0, len(data), size)]
    if layer.find("ReferenceInformationType").props[0] == "IndexToDirect":
        index_name = {"UV": "UVIndex", "Tangents": "TangentsIndex", "Binormals": "BinormalsIndex"}[data_name]
        data = [data[i] for i in fbx_probe._array(layer.find(index_name))]
    return data


def _sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def _cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _unit(v):
    length = math.sqrt(_dot(v, v))
    return [c / length for c in v] if length > 0 else [0.0, 0.0, 0.0]


def triangulate(points):
    if len(points) == 3:
        return [(0, 1, 2)]
    if len(points) == 4:
        v0, v1, v2, v3 = points
        a, b = _sub(v2, v0), _sub(v3, v1)
        na1, na3 = _unit(_cross(a, _sub(v1, v0))), _unit(_cross(a, _sub(v0, v3)))
        nb0, nb2 = _unit(_cross(b, _sub(v1, v0))), _unit(_cross(b, _sub(v2, v1)))
        dot_na, dot_nb = _dot(na1, na3), _dot(nb0, nb2)
        split_a = _dot(a, a) <= _dot(b, b)
        if dot_na < 0 or dot_nb < 0:
            split_a = dot_na >= dot_nb
        return [(0, 1, 2), (2, 3, 0)] if split_a else [(1, 2, 3), (3, 0, 1)]
    return [(0, i, i + 1) for i in range(1, len(points) - 1)]


def material_values(props, shading, blender):
    def color(name, default):
        return _vec(props, name, default)

    def scalar(name, default):
        value = props.get(name)
        return float(value[0]) if value else default

    if blender:
        shininess = scalar("ShininessExponent", scalar("Shininess", 20.0))
        roughness = max(0.0, 1.0 - math.sqrt(shininess) * 0.1) if shininess >= 0 else 0.0
        return {"base": color("DiffuseColor", (0.8, 0.8, 0.8)), "metallic": scalar("ReflectionFactor", 0.0),
                "roughness": roughness, "alpha": 1.0 - scalar("TransparencyFactor", 0.0),
                "emissive": [c * scalar("EmissiveFactor", 1.0) for c in color("EmissiveColor", (0, 0, 0))]}
    if str(shading).lower() != "lambert":
        raise NotImplementedError(f"shading model {shading}")
    factor = scalar("DiffuseFactor", 1.0)
    return {"base": [c * factor for c in color("DiffuseColor", (0.8, 0.8, 0.8))], "metallic": 0.0, "roughness": 1.0,
            "alpha": 1.0, "emissive": [c * scalar("EmissiveFactor", 1.0) for c in color("EmissiveColor", (0, 0, 0))]}


ZERO_KEYS = ["RotationOffset", "RotationPivot", "ScalingOffset", "ScalingPivot", "PreRotation", "PostRotation"]


def build(path, textures=False):
    """Return (glb_bytes, summary) for one static FBX file.

    With textures=True, embedded DiffuseColor/NormalMap images become glTF base
    colour/normal textures, UV0 V is flipped to glTF's top-left origin, and normal
    mapped primitives carry TANGENT from the file's own tangent/binormal layers.
    """
    _, _, root = fbx_probe.load(path)
    creator = root.find("Creator")
    blender = bool(creator and "Blender" in str(creator.props[0]))
    settings = {k: v[0] for k, v in fbx_probe._props70(root.find("GlobalSettings")).items() if v}
    templates = {}
    for ot in root.find("Definitions").all("ObjectType"):
        tpl = ot.find("PropertyTemplate")
        if tpl is not None:
            templates[ot.props[0]] = fbx_probe._props70(tpl)
    objects = {o.props[0]: o for o in root.find("Objects").children}
    links = [c.props for c in root.find("Connections").all("C")]
    glb = gltfio.Glb()
    material_index, normal_mapped = {}, set()
    for oid, obj in objects.items():
        if obj.name == "Material":
            shading = obj.find("ShadingModel")
            values = material_values(_props(obj, templates.get("Material", {})), shading.props[0] if shading else None, blender)
            maps = {}
            for link in links:
                if link[0] == "OP" and link[2] == oid:
                    if not textures or link[3] not in ("DiffuseColor", "NormalMap"):
                        raise NotImplementedError(f"texture channel {link[3]}")
                    video = next(objects[l[1]] for l in links if l[0] == "OO" and l[2] == link[1] and objects[l[1]].name == "Video")
                    content = video.find("Content")
                    if content is None:
                        raise NotImplementedError("twin needs embedded image content")
                    maps[link[3]] = glb.texture(base64.b64decode(content.props[-1]))
            if "NormalMap" in maps:
                normal_mapped.add(oid)
            material_index[oid] = glb.material(fbx_probe._name(obj.props[1]), base=values["base"] + [values["alpha"]],
                                               metallic=values["metallic"], roughness=values["roughness"],
                                               emissive=values["emissive"], base_texture=maps.get("DiffuseColor"),
                                               normal_texture=maps.get("NormalMap"))
    meshes = {}

    def mesh_for(geometry_id, model_materials):
        key = (geometry_id, tuple(model_materials))
        if key in meshes:
            return meshes[key]
        positions, polys, normals, slots, layers = _geometry(objects[geometry_id])
        parts = {}
        for poly, slot in zip(polys, slots):
            points = [positions[v] for _, v in poly]
            if normals is None:
                flat = _unit(_cross(_sub(points[1], points[0]), _sub(points[2], points[0])))
            part = parts.setdefault(slot if model_materials else None, {"positions": [], "normals": [], "indices": []})
            material = model_materials[slot] if model_materials else None
            base = len(part["positions"])
            for corner, vertex in poly:
                part["positions"].append(positions[vertex])
                n = normalize(normals(corner, vertex)) if normals else flat
                part["normals"].append(n)
                if textures and layers["uv"] is not None:
                    u, v = layers["uv"][corner]
                    part.setdefault("uvs", []).append([u, 1.0 - v])  # FBX bottom-left -> glTF top-left
                if material in normal_mapped:
                    # The exporter's own frame: bitangent sign from Maya's exported binormal.
                    tangent, binormal = _unit(layers["tangent"][corner]), layers["binormal"][corner]
                    w = 1.0 if _dot(_cross(n, tangent), binormal) >= 0 else -1.0
                    part.setdefault("tangents", []).append(tangent + [w])
            for tri in triangulate(points):
                part["indices"] += [base + i for i in tri]
        prims = []
        for slot, part in sorted(parts.items(), key=lambda kv: (kv[0] is None, kv[0])):
            if slot is not None:
                part["material"] = material_index[model_materials[slot]]
            prims.append(part)
        meshes[key] = glb.mesh(f"geometry {geometry_id}", prims)
        return meshes[key]

    model_tpl = templates.get("Model", {})
    children = {}
    for child, parent in [(l[1], l[2]) for l in links if l[0] == "OO"]:
        children.setdefault(parent, []).append(child)
    summary = {"models": []}

    def build_model(oid):
        obj = objects[oid]
        props = _props(obj, model_tpl)
        for key in ZERO_KEYS:
            if any(abs(c) > 1e-12 for c in _vec(props, key, (0, 0, 0))):
                raise NotImplementedError(f"{key} on {obj.props[1]}")
        if int(props.get("InheritType", [1])[0]) != 1:
            raise NotImplementedError("non-RSrs inheritance")
        order = int(props.get("RotationOrder", [0])[0])
        t, r, s = _vec(props, "Lcl Translation", (0, 0, 0)), _vec(props, "Lcl Rotation", (0, 0, 0)), _vec(props, "Lcl Scaling", (1, 1, 1))
        kids = [build_model(c) for c in children.get(oid, []) if objects.get(c) is not None and objects[c].name == "Model"]
        geometry = [c for c in children.get(oid, []) if objects.get(c) is not None and objects[c].name == "Geometry"]
        mats = [c for c in children.get(oid, []) if objects.get(c) is not None and objects[c].name == "Material"]
        if geometry:
            mesh = mesh_for(geometry[0], mats)
            gt, gr, gs = (_vec(props, "GeometricTranslation", (0, 0, 0)), _vec(props, "GeometricRotation", (0, 0, 0)),
                          _vec(props, "GeometricScaling", (1, 1, 1)))
            if gt == [0, 0, 0] and gr == [0, 0, 0] and gs == [1, 1, 1]:
                node_mesh = mesh
            else:
                kids.append(glb.node(f"{fbx_probe._name(obj.props[1])} geometry", gt, euler_quat(gr, 0), gs, mesh=mesh))
                node_mesh = None
        else:
            node_mesh = None
        summary["models"].append({"name": fbx_probe._name(obj.props[1]), "translation": t, "rotation": r, "order": ORDERS[order],
                                  "scale": s, "materials": len(mats)})
        return glb.node(fbx_probe._name(obj.props[1]), t, euler_quat(r, order), s, mesh=node_mesh, children=kids)

    top = [build_model(c) for c in children.get(0, []) if objects.get(c) is not None and objects[c].name == "Model"]
    unit = float(settings.get("UnitScaleFactor", 1.0)) / 100.0
    rotation = axis_conversion(settings)
    glb.root(glb.node("FBX root (axes and units)", None, rotation, [unit] * 3, children=top))
    summary.update({"unit_meters": unit, "axis_rotation": [round(c, 9) for c in rotation], "blender_encoding": blender,
                    "materials": len(material_index), "meshes": len(meshes)})
    return glb.bytes(), summary
