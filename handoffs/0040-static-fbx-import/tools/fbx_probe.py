"""Independent FBX 6/7 ASCII and binary reader for handoff 0040 expectations.

It does not use ufbx or any engine code. It reads node trees, then summarises the
global axes/units, models and their transform properties, geometry sizes, per-polygon
material partitions and material/texture properties, so rendered images can be
judged against what the source file itself states.
Usage: python3 -I fbx_probe.py FILE.fbx [...]  (prints JSON)
"""
import json
import re
import struct
import sys
import zlib


class Node:
    __slots__ = ("name", "props", "children")

    def __init__(self, name, props, children):
        self.name, self.props, self.children = name, props, children

    def find(self, name):
        return next((c for c in self.children if c.name == name), None)

    def all(self, name):
        return [c for c in self.children if c.name == name]


# ---------------------------------------------------------------- binary
def _binary(data):
    version = struct.unpack_from("<I", data, 23)[0]
    wide = version >= 7500
    head = "<QQQB" if wide else "<IIIB"
    head_size = struct.calcsize(head)
    null_size = 25 if wide else 13

    def prop(pos):
        code = chr(data[pos])
        pos += 1
        scalar = {"Y": "<h", "C": "<?", "I": "<i", "F": "<f", "D": "<d", "L": "<q"}
        if code in scalar:
            value = struct.unpack_from(scalar[code], data, pos)[0]
            return value, pos + struct.calcsize(scalar[code])
        if code in "fdlib":
            count, encoding, length = struct.unpack_from("<III", data, pos)
            pos += 12
            raw = data[pos:pos + length]
            if encoding == 1:
                raw = zlib.decompress(raw)
            fmt = {"f": "f", "d": "d", "l": "q", "i": "i", "b": "?"}[code]
            return list(struct.unpack("<%d%s" % (count, fmt), raw)), pos + length
        if code in "SR":
            (length,) = struct.unpack_from("<I", data, pos)
            raw = bytes(data[pos + 4:pos + 4 + length])
            if code == "S":
                text = raw.decode("utf-8", "replace")
                # Binary object names are "Name\x00\x01Class"; present them as "Class::Name".
                if "\x00\x01" in text:
                    name, cls = text.split("\x00\x01", 1)
                    text = f"{cls}::{name}"
                return text, pos + 4 + length
            return raw, pos + 4 + length
        raise ValueError(f"unknown FBX property type {code!r} at {pos}")

    def node(pos):
        end, count, _, name_len = struct.unpack_from(head, data, pos)
        if end == 0:
            return None, pos + null_size
        pos += head_size
        name = data[pos:pos + name_len].decode("ascii")
        pos += name_len
        props = []
        for _ in range(count):
            value, pos = prop(pos)
            props.append(value)
        children = []
        while pos < end:
            child, pos = node(pos)
            if child is None:
                break
            children.append(child)
        return Node(name, props, children), end

    pos, top = 27, []
    while pos < len(data) - null_size:
        child, pos = node(pos)
        if child is None:
            break
        top.append(child)
    return version, Node("", [], top)


# ---------------------------------------------------------------- ASCII
_TOKEN = re.compile(r'\s*(?:;[^\n]*|"((?:[^"\\]|\\.)*)"|([A-Za-z_][\w|]*)\s*:|([{}]|,)|(\*\d+)|([^\s,{}";]+))', re.S)


def _ascii(text):
    tokens = []
    for m in _TOKEN.finditer(text):
        if m.group(1) is not None:
            tokens.append(("str", m.group(1)))
        elif m.group(2) is not None:
            tokens.append(("key", m.group(2)))
        elif m.group(3) is not None:
            tokens.append(("sym", m.group(3)))
        elif m.group(4) is not None:
            tokens.append(("count", m.group(4)))
        elif m.group(5) is not None:
            tokens.append(("val", m.group(5)))
    pos = 0

    def value(token):
        kind, raw = token
        if kind == "str":
            return raw
        try:
            return int(raw)
        except ValueError:
            try:
                return float(raw)
            except ValueError:
                return raw

    def parse_children(stop):
        nonlocal pos
        out = []
        while pos < len(tokens) and tokens[pos] != ("sym", "}"):
            kind, raw = tokens[pos]
            if kind != "key":
                pos += 1
                continue
            pos += 1
            props, children = [], []
            while pos < len(tokens):
                kind2, raw2 = tokens[pos]
                if kind2 == "key":
                    break
                if (kind2, raw2) == ("sym", "{"):
                    pos += 1
                    children = parse_children("}")
                    pos += 1
                    break
                if (kind2, raw2) == ("sym", "}"):
                    break
                if kind2 != "sym" and kind2 != "count":
                    props.append(value(tokens[pos]))
                pos += 1
            if raw == "a" and not children:
                # Array payload "a: v,v,v" inside a "*N { }" block.
                out.append(Node("a", props, []))
            else:
                out.append(Node(raw, props, children))
        return out

    root = Node("", [], parse_children(None))
    header = root.find("FBXHeaderExtension")
    version = header.find("FBXVersion").props[0] if header and header.find("FBXVersion") else None
    return version, root


def load(path):
    with open(path, "rb") as f:
        data = f.read()
    if data.startswith(b"Kaydara FBX Binary"):
        version, root = _binary(data)
        return "binary", version, root
    version, root = _ascii(data.decode("utf-8"))
    return "ascii", version, root


def _array(node):
    if node is None:
        return []
    a = node.find("a")
    if a is not None:
        return a.props
    return node.props[0] if node.props and isinstance(node.props[0], list) else node.props


def _props70(node):
    out = {}
    block = node.find("Properties70") if node else None
    for p in (block.all("P") if block else []):
        out[p.props[0]] = p.props[4:] if len(p.props) > 4 else []
    return out


def _name(prop):
    text = str(prop)
    return text.split("::", 1)[1] if "::" in text else text


def summarize(path):
    kind, version, root = load(path)
    settings = _props70(root.find("GlobalSettings"))
    objects = root.find("Objects")
    templates = {}
    definitions = root.find("Definitions")
    for ot in (definitions.all("ObjectType") if definitions else []):
        tpl = ot.find("PropertyTemplate")
        if tpl:
            templates[ot.props[0]] = _props70(tpl)
    by_id = {}
    for obj in objects.children:
        by_id[obj.props[0]] = obj
    links = []
    for c in root.find("Connections").all("C"):
        links.append({"type": c.props[0], "child": c.props[1], "parent": c.props[2],
                      "property": c.props[3] if len(c.props) > 3 else None})
    counts = {}
    for obj in objects.children:
        counts[obj.name] = counts.get(obj.name, 0) + 1
    model_tpl = templates.get("Model", {})

    def model_props(obj):
        merged = dict(model_tpl)
        merged.update(_props70(obj))
        keep = ["Lcl Translation", "Lcl Rotation", "Lcl Scaling", "GeometricTranslation", "GeometricRotation",
                "GeometricScaling", "RotationOrder", "InheritType", "PreRotation", "PostRotation", "RotationPivot",
                "ScalingPivot", "RotationOffset", "ScalingOffset", "Visibility", "Show"]
        return {k: merged[k] for k in keep if k in merged}

    models = {}
    for oid, obj in by_id.items():
        if obj.name == "Model":
            parents = [l["parent"] for l in links if l["child"] == oid and l["type"] == "OO"]
            models[oid] = {"name": _name(obj.props[1]), "type": obj.props[2] if len(obj.props) > 2 else None,
                           "parent": parents[0] if parents else None, "properties": model_props(obj),
                           "geometry": [l["child"] for l in links if l["parent"] == oid and by_id.get(l["child"]) is not None and by_id[l["child"]].name == "Geometry"],
                           "materials": [l["child"] for l in links if l["parent"] == oid and by_id.get(l["child"]) is not None and by_id[l["child"]].name == "Material"]}
    for m in models.values():
        parent = m["parent"]
        m["parent_name"] = models[parent]["name"] if parent in models else ("RootNode" if parent == 0 else parent)
    mat_tpl = templates.get("Material", {})
    materials = {}
    for oid, obj in by_id.items():
        if obj.name == "Material":
            merged = dict(mat_tpl)
            merged.update(_props70(obj))
            shading = obj.find("ShadingModel")
            materials[oid] = {"name": _name(obj.props[1]), "shading_model": shading.props[0] if shading else None,
                              "properties": {k: v for k, v in merged.items() if v != []},
                              "textures": [{"texture": l["child"], "property": l["property"]} for l in links
                                           if l["parent"] == oid and l["type"] == "OP"]}
    geometries = {}
    for oid, obj in by_id.items():
        if obj.name != "Geometry":
            continue
        verts = _array(obj.find("Vertices"))
        poly = _array(obj.find("PolygonVertexIndex"))
        sizes, current = [], 0
        polygons, corners = [], []
        for idx in poly:
            current += 1
            corners.append(idx if idx >= 0 else ~idx)
            if idx < 0:
                sizes.append(current)
                polygons.append(corners)
                current, corners = 0, []
        layer_mat = obj.find("LayerElementMaterial")
        mapping = layer_mat.find("MappingInformationType").props[0] if layer_mat else None
        mat_idx = _array(layer_mat.find("Materials")) if layer_mat else []
        uv = obj.find("LayerElementUV")
        positions = [verts[i:i + 3] for i in range(0, len(verts), 3)]
        per_material = {}
        for p, corners_ in enumerate(polygons):
            slot = (mat_idx[p] if mapping == "ByPolygon" else (mat_idx[0] if mat_idx else None))
            entry = per_material.setdefault(slot, {"polygons": 0, "sum": [0.0, 0.0, 0.0], "corners": 0,
                                                  "min": [float("inf")] * 3, "max": [float("-inf")] * 3})
            entry["polygons"] += 1
            for c in corners_:
                for a in range(3):
                    v = positions[c][a]
                    entry["sum"][a] += v
                    entry["min"][a] = min(entry["min"][a], v)
                    entry["max"][a] = max(entry["max"][a], v)
                entry["corners"] += 1
        for entry in per_material.values():
            entry["centroid"] = [round(s / entry["corners"], 6) for s in entry.pop("sum")]
            entry.pop("corners")
        lo = [min(p[a] for p in positions) for a in range(3)] if positions else None
        hi = [max(p[a] for p in positions) for a in range(3)] if positions else None
        geometries[oid] = {"vertices": len(positions), "polygons": len(polygons),
                           "polygon_sizes": {str(s): sizes.count(s) for s in sorted(set(sizes))},
                           "bounds": [lo, hi], "material_mapping": mapping,
                           "polygons_per_material_slot": {str(k): v for k, v in sorted(per_material.items(), key=lambda kv: (kv[0] is None, kv[0]))},
                           "uv_sets": len(obj.all("LayerElementUV")),
                           "uv_mapping": uv.find("MappingInformationType").props[0] if uv else None,
                           "layer_elements": sorted({c.name for c in obj.children if c.name.startswith("LayerElement")})}
    textures = {oid: {"name": _name(o.props[1]), "relative": (o.find("RelativeFilename").props[0] if o.find("RelativeFilename") else None)}
                for oid, o in by_id.items() if o.name == "Texture"}
    axis = {k: settings.get(k, [None])[0] for k in ("UpAxis", "UpAxisSign", "FrontAxis", "FrontAxisSign",
                                                     "CoordAxis", "CoordAxisSign", "UnitScaleFactor", "OriginalUnitScaleFactor")}
    creator = root.find("Creator")
    header = root.find("FBXHeaderExtension")
    app = None
    if header and header.find("SceneInfo"):
        info = _props70(header.find("SceneInfo"))
        app = {k: info[k][0] for k in ("Original|ApplicationName", "Original|ApplicationVersion") if k in info and info[k]}
    return {"format": kind, "version": version, "creator": creator.props[0] if creator and creator.props else None,
            "application": app, "global_settings": axis, "object_counts": dict(sorted(counts.items())),
            "models": {str(k): v for k, v in models.items()}, "geometries": {str(k): v for k, v in geometries.items()},
            "materials": {str(k): v for k, v in materials.items()}, "textures": {str(k): v for k, v in textures.items()}}


def _jsonable(value):
    if isinstance(value, bytes):
        return f"<{len(value)} raw bytes>"
    if isinstance(value, dict):
        return {k: _jsonable(v) for k, v in value.items()}
    if isinstance(value, list):
        return [_jsonable(v) for v in value]
    if isinstance(value, float) and value in (float("inf"), float("-inf")):
        return None
    return value


if __name__ == "__main__":
    print(json.dumps({p: _jsonable(summarize(p)) for p in sys.argv[1:]}, indent=1))
