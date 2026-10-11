"""Small FBX derivatives of the real upstream Maya fixtures for handoff 0040.

Every derivative starts from the unmodified MIT ufbx v0.23.1 Maya file and applies
a few named text edits, following the legal Video/Texture records shown by
crates/incant_assets/tests/fbx.rs::textured_source. Texture records also carry the
Maya UV set name "map1", as Maya writes it. Edits are listed per output in the
manifest so each change is reviewable. Images are original art from art.py.
"""
import base64

FIXTURES = "crates/incant_assets/tests/fixtures/fbx"
CUBE = "maya_cube_7500_ascii.fbx"
PARENTED = "maya_parented_cubes_7500_ascii.fbx"
GEOMETRY, MODEL, MATERIAL = 1908558533920, 1907292239120, 1907674217712
NORMAL_MATERIAL = 1907674217800


def _replace(text, old, new, edits, label):
    if text.count(old) != 1:
        raise ValueError(f"anchor for {label!r} found {text.count(old)} times")
    edits.append(label)
    return text.replace(old, new)


def _video_texture(vid, tid, name, relative, content):
    content_line = f'\t\tContent: ,\n\t\t"{base64.b64encode(content).decode()}"\n' if content is not None else ""
    return (f'\tVideo: {vid}, "Video::{name}", "Clip" {{\n\t\tType: "Clip"\n'
            f'\t\tFilename: "C:/never/opened/{name}.png"\n\t\tRelativeFilename: "{relative}"\n{content_line}\t}}\n'
            f'\tTexture: {tid}, "Texture::{name}", "" {{\n\t\tType: "TextureVideoClip"\n\t\tVersion: 202\n'
            f'\t\tTextureName: "Texture::{name}"\n\t\tProperties70:  {{\n\t\t\tP: "UVSet", "KString", "", "", "map1"\n\t\t}}\n'
            f'\t\tMedia: "Video::{name}"\n\t\tFileName: "C:/never/opened/{name}.png"\n'
            f'\t\tRelativeFilename: "{relative}"\n\t\tModelUVTranslation: 0,0\n\t\tModelUVScaling: 1,1\n'
            f'\t\tTexture_Alpha_Source: "None"\n\t\tCropping: 0,0,0,0\n\t}}\n')


def _insert_objects(text, block, edits, label):
    return _replace(text, "\n\tAnimationStack:", "\n" + block.rstrip("\n") + "\n\tAnimationStack:", edits, label)


def _connect(text, lines, edits, label):
    body = "".join(f"\n\t{line}" for line in lines)
    return _replace(text, "Connections:  {", "Connections:  {" + body, edits, label)


def _lambert(text, edits, color, factor):
    text = _replace(text, 'P: "DiffuseColor", "Color", "", "A",0.5,0.5,0.5',
                    f'P: "DiffuseColor", "Color", "", "A",{color},{color},{color}', edits, f"lambert1 DiffuseColor {color}")
    return _replace(text, 'P: "DiffuseFactor", "Number", "", "A",0.800000011920929',
                    f'P: "DiffuseFactor", "Number", "", "A",{factor}', edits, f"lambert1 DiffuseFactor {factor}")


def textured(source, image, *, relative="face_art.png", embedded=True, channel="DiffuseColor", extra_channel=None,
             texture_props=None):
    """Maya cube with one file texture on lambert1. Returns (text, edits)."""
    edits = []
    text = source
    if channel == "DiffuseColor":
        # The upstream Maya textured cube exports white DiffuseColor with factor 0.8 for a
        # connected colour map; use the same values so the map is not darkened by 0.5 grey.
        text = _lambert(text, edits, 1, 0.800000011920929)
    else:
        text = _lambert(text, edits, 0.8, 1)
    block = _video_texture(90001, 90002, "Image", relative, image if embedded else None)
    if texture_props:
        block = block.replace('\t\t\tP: "UVSet", "KString", "", "", "map1"\n',
                              '\t\t\tP: "UVSet", "KString", "", "", "map1"\n' + "".join(f"\t\t\t{p}\n" for p in texture_props))
        edits.append("texture properties: " + "; ".join(texture_props))
    text = _insert_objects(text, block, edits, f"Video/Texture 'Image' ({'embedded' if embedded else 'project-relative ' + relative})")
    lines = ['C: "OO",90001,90002', f'C: "OP",90002,{MATERIAL}, "{channel}"']
    if extra_channel:
        lines.append(f'C: "OP",90002,{MATERIAL}, "{extra_channel}"')
    return _connect(text, lines, edits, "connections to lambert1 " + "+".join(filter(None, [channel, extra_channel]))), edits


def phong(text, edits):
    return _replace(text, 'ShadingModel: "lambert"', 'ShadingModel: "phong"', edits, "lambert1 ShadingModel phong")


def second_uv_set(text, edits):
    start = text.index("\t\tLayerElementUV: 0 {")
    end = text.index("\t\tLayerElementMaterial: 0 {")
    layer = text[start:end].replace("LayerElementUV: 0 {", "LayerElementUV: 1 {").replace('Name: "map1"', 'Name: "map2"')
    text = text[:end] + layer + text[end:]
    text = _replace(text, "\t\tLayer: 0 {", "\t\tLayer: 1 {\n\t\t\tVersion: 100\n\t\t\tLayerElement:  {\n\t\t\t\tType: \"LayerElementUV\"\n"
                    "\t\t\t\tTypedIndex: 1\n\t\t\t}\n\t\t}\n\t\tLayer: 0 {", edits, "second UV set map2 on Layer 1")
    return text


def model(mid, name, translation=(0, 0, 0), rotation=(0, 0, 0), scaling=(1, 1, 1)):
    fmt = lambda v: ",".join(repr(float(c)).rstrip("0").rstrip(".") if c != int(c) else str(int(c)) for c in v)  # noqa: E731
    return (f'\tModel: {mid}, "Model::{name}", "Mesh" {{\n\t\tVersion: 232\n\t\tProperties70:  {{\n'
            f'\t\t\tP: "InheritType", "enum", "", "",1\n\t\t\tP: "DefaultAttributeIndex", "int", "Integer", "",0\n'
            f'\t\t\tP: "Lcl Translation", "Lcl Translation", "", "A",{fmt(translation)}\n'
            f'\t\t\tP: "Lcl Rotation", "Lcl Rotation", "", "A",{fmt(rotation)}\n'
            f'\t\t\tP: "Lcl Scaling", "Lcl Scaling", "", "A",{fmt(scaling)}\n'
            f'\t\t\tP: "currentUVSet", "KString", "", "U", "map1"\n\t\t}}\n\t\tShading: T\n\t\tCulling: "CullingOff"\n\t}}\n')


INSTANCES = [  # (id, name, material, translation cm, rotation deg, scaling)
    (91001, "A_mirror_x", "A", (1.6, 0, 0), (0, 0, 0), (-1, 1, 1)),
    (91002, "A_nonuniform", "A", (3.2, 0, 0), (0, 0, 0), (1.4, 0.7, 1)),
    (91003, "A_mirror_y", "A", (4.8, 0, 0), (0, 0, 0), (1, -1, 1)),
    (91011, "B_identity", "B", (0, -2, 0), (0, 0, 0), (1, 1, 1)),
    (91012, "B_mirror_x", "B", (1.6, -2, 0), (0, 0, 0), (-1, 1, 1)),
    (91013, "B_nonuniform", "B", (3.2, -2, 0), (0, 0, 0), (1.4, 0.7, 1)),
    (91014, "B_rot_z90", "B", (4.8, -2, 0), (0, 0, 90), (1, 1, 1)),
]


def instances(source, color_png, normal_png):
    """pCube1 (identity, colour map) plus seven placements sharing one geometry."""
    edits = []
    text = _lambert(source, edits, 1, 0.800000011920929)
    material_b = (f'\tMaterial: {NORMAL_MATERIAL}, "Material::bevel_normal", "" {{\n\t\tVersion: 102\n\t\tShadingModel: "lambert"\n'
                  f'\t\tMultiLayer: 0\n\t\tProperties70:  {{\n\t\t\tP: "AmbientColor", "Color", "", "A",0,0,0\n'
                  f'\t\t\tP: "DiffuseColor", "Color", "", "A",0.8,0.8,0.8\n\t\t\tP: "DiffuseFactor", "Number", "", "A",1\n'
                  f'\t\t\tP: "TransparencyFactor", "Number", "", "A",1\n\t\t}}\n\t}}\n')
    block = (_video_texture(90001, 90002, "FaceArt", "face_art.png", color_png)
             + _video_texture(90003, 90004, "Bevels", "bevels.png", normal_png) + material_b
             + "".join(model(i, n, t, r, s) for i, n, _, t, r, s in INSTANCES))
    text = _insert_objects(text, block, edits, "two embedded textures, material bevel_normal, seven Model instances")
    lines = ['C: "OO",90001,90002', f'C: "OP",90002,{MATERIAL}, "DiffuseColor"',
             'C: "OO",90003,90004', f'C: "OP",90004,{NORMAL_MATERIAL}, "NormalMap"']
    for i, _, mat, _, _, _ in INSTANCES:
        lines += [f'C: "OO",{i},0', f'C: "OO",{GEOMETRY},{i}', f'C: "OO",{MATERIAL if mat == "A" else NORMAL_MATERIAL},{i}']
    text = _connect(text, lines, edits, "instance, geometry and material connections")
    return text, edits


def normal_cube(source, normal_png):
    edits = []
    text, more = textured(source, normal_png, relative="bevels.png", channel="NormalMap")
    return text, edits + more


def shear_mirror(source):
    """Real Maya hierarchy with a mirrored nonuniform parent and a rotated child."""
    edits = []
    text = _replace(source, 'P: "Lcl Scaling", "Lcl Scaling", "", "A",1.1,1.2,1.3',
                    'P: "Lcl Scaling", "Lcl Scaling", "", "A",-1.1,1.2,1.3', edits, "Parent Lcl Scaling -1.1,1.2,1.3")
    text = _replace(text, 'P: "Lcl Translation", "Lcl Translation", "", "A",0,-1,0',
                    'P: "Lcl Translation", "Lcl Translation", "", "A",0,-1,0\n\t\t\tP: "Lcl Rotation", "Lcl Rotation", "", "A",0,0,40',
                    edits, "Child Lcl Rotation 0,0,40 (shear under the nonuniform parent)")
    return text, edits


MATERIAL_CASES = [  # (model id, model, material id, material, translation cm, Properties70 lines)
    (None, "pCube1", MATERIAL, "lambert1", (0, 0, 0), None),  # lambert1 edited: diffuse colour x factor
    (92001, "emissive_map", 92101, "emissive_map", (1.6, 0, 0),
     ['P: "DiffuseColor", "Color", "", "A",0,0,0', 'P: "EmissiveColor", "Color", "", "A",1,1,1', 'P: "EmissiveFactor", "Number", "", "A",1']),
    (92002, "emissive_constant", 92102, "emissive_constant", (3.2, 0, 0),
     ['P: "DiffuseColor", "Color", "", "A",0.2,0.2,0.2', 'P: "EmissiveColor", "Color", "", "A",0.9,0.5,0.1', 'P: "EmissiveFactor", "Number", "", "A",1']),
    (92003, "opacity_half", 92103, "opacity_half", (4.8, 0, 0),
     ['P: "DiffuseColor", "Color", "", "A",0.8,0.8,0.8', 'P: "TransparentColor", "Color", "", "A",0.5,0.5,0.5', 'P: "TransparencyFactor", "Number", "", "A",1']),
]


def materials(source, emissive_png):
    """pCube1 plus three instances, each with its own Lambert constant/map case."""
    edits = []
    text = _replace(source, 'P: "DiffuseColor", "Color", "", "A",0.5,0.5,0.5', 'P: "DiffuseColor", "Color", "", "A",0.2,0.5,0.9',
                    edits, "lambert1 DiffuseColor 0.2,0.5,0.9")
    text = _replace(text, 'P: "DiffuseFactor", "Number", "", "A",0.800000011920929', 'P: "DiffuseFactor", "Number", "", "A",0.5',
                    edits, "lambert1 DiffuseFactor 0.5")
    block = _video_texture(90001, 90002, "Glow", "glow.png", emissive_png)
    lines = ['C: "OO",90001,90002', 'C: "OP",90002,92101, "EmissiveColor"']
    for mid, name, matid, matname, translation, props in MATERIAL_CASES[1:]:
        block += (f'\tMaterial: {matid}, "Material::{matname}", "" {{\n\t\tVersion: 102\n\t\tShadingModel: "lambert"\n'
                  f'\t\tMultiLayer: 0\n\t\tProperties70:  {{\n\t\t\tP: "AmbientColor", "Color", "", "A",0,0,0\n'
                  + "".join(f"\t\t\t{p}\n" for p in props) + "\t\t}\n\t}\n")
        block += model(mid, name, translation)
        lines += [f'C: "OO",{mid},0', f'C: "OO",{GEOMETRY},{mid}', f'C: "OO",{matid},{mid}']
    text = _insert_objects(text, block, edits, "embedded emissive texture, three Lambert materials and instances")
    return _connect(text, lines, edits, "material case connections"), edits
