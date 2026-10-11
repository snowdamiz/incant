"""Small deterministic glTF 2.0 binary (GLB) writer for handoff 0040 controls."""
import json
import struct


class Glb:
    def __init__(self, generator="incant handoff 0040 review controls (Claude)"):
        self.json = {"asset": {"version": "2.0", "generator": generator}, "buffers": [], "bufferViews": [],
                     "accessors": [], "meshes": [], "materials": [], "nodes": [], "scenes": [{"nodes": []}],
                     "scene": 0}
        self.bin = bytearray()

    def _view(self, data, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        view = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target:
            view["target"] = target
        self.bin += data
        self.json["bufferViews"].append(view)
        return len(self.json["bufferViews"]) - 1

    def _accessor(self, values, kind, width, target, bounds=False):
        if kind == "index":
            data = struct.pack("<%dI" % len(values), *values)
            accessor = {"componentType": 5125, "count": len(values), "type": "SCALAR"}
        else:
            flat = [c for v in values for c in v]
            data = struct.pack("<%df" % len(flat), *flat)
            accessor = {"componentType": 5126, "count": len(values), "type": {2: "VEC2", 3: "VEC3", 4: "VEC4"}[width]}
            if bounds:
                accessor["min"] = [min(v[i] for v in values) for i in range(width)]
                accessor["max"] = [max(v[i] for v in values) for i in range(width)]
        accessor["bufferView"] = self._view(data, target)
        self.json["accessors"].append(accessor)
        return len(self.json["accessors"]) - 1

    def material(self, name, base=(1, 1, 1, 1), metallic=0.0, roughness=1.0, base_texture=None, normal_texture=None,
                 emissive=(0, 0, 0), emissive_texture=None, alpha_mode=None):
        pbr = {"baseColorFactor": list(base), "metallicFactor": metallic, "roughnessFactor": roughness}
        if base_texture is not None:
            pbr["baseColorTexture"] = {"index": base_texture}
        value = {"name": name, "pbrMetallicRoughness": pbr, "emissiveFactor": list(emissive)}
        if normal_texture is not None:
            value["normalTexture"] = {"index": normal_texture}
        if emissive_texture is not None:
            value["emissiveTexture"] = {"index": emissive_texture}
        if alpha_mode is not None:
            value["alphaMode"] = alpha_mode
        self.json["materials"].append(value)
        return len(self.json["materials"]) - 1

    def texture(self, png_bytes):
        self.json.setdefault("images", []).append({"bufferView": self._view(bytes(png_bytes)), "mimeType": "image/png"})
        self.json.setdefault("samplers", [{"wrapS": 10497, "wrapT": 10497}])
        self.json.setdefault("textures", []).append({"source": len(self.json["images"]) - 1, "sampler": 0})
        return len(self.json["textures"]) - 1

    def mesh(self, name, primitives):
        """primitives: dicts with positions, normals, indices, optional uvs/tangents/material."""
        out = []
        for p in primitives:
            attributes = {"POSITION": self._accessor(p["positions"], "f", 3, 34962, bounds=True),
                          "NORMAL": self._accessor(p["normals"], "f", 3, 34962)}
            if p.get("uvs") is not None:
                attributes["TEXCOORD_0"] = self._accessor(p["uvs"], "f", 2, 34962)
            if p.get("tangents") is not None:
                attributes["TANGENT"] = self._accessor(p["tangents"], "f", 4, 34962)
            prim = {"attributes": attributes, "indices": self._accessor(p["indices"], "index", 1, 34963), "mode": 4}
            if p.get("material") is not None:
                prim["material"] = p["material"]
            out.append(prim)
        self.json["meshes"].append({"name": name, "primitives": out})
        return len(self.json["meshes"]) - 1

    def node(self, name, translation=None, rotation=None, scale=None, mesh=None, children=None):
        value = {"name": name}
        if translation is not None:
            value["translation"] = [float(c) for c in translation]
        if rotation is not None:
            value["rotation"] = [float(c) for c in rotation]
        if scale is not None:
            value["scale"] = [float(c) for c in scale]
        if mesh is not None:
            value["mesh"] = mesh
        if children:
            value["children"] = list(children)
        self.json["nodes"].append(value)
        return len(self.json["nodes"]) - 1

    def root(self, node):
        self.json["scenes"][0]["nodes"].append(node)

    def bytes(self):
        while len(self.bin) % 4:
            self.bin.append(0)
        doc = dict(self.json)
        doc["buffers"] = [{"byteLength": len(self.bin)}]
        for key in ("materials", "meshes"):
            if not doc[key]:
                del doc[key]
        text = json.dumps(doc, separators=(",", ":"), sort_keys=True).encode()
        while len(text) % 4:
            text += b" "
        body = struct.pack("<I4s", len(text), b"JSON") + text + struct.pack("<I4s", len(self.bin), b"BIN\x00") + bytes(self.bin)
        return struct.pack("<4sII", b"glTF", 2, 12 + len(body)) + body
