"""Generate every Core Sample source asset into one directory, deterministically.

Usage: python3 -B -m art.world OUTPUT_DIR   (run from samples/core)

Writes the static canyon model (terrain and set dressing merged per material),
the three actor models, the sky dome and environment image, the music loop and
event cues, plus a manifest of SHA-256 digests and mesh/audio statistics.
Re-running produces identical bytes.
"""
import hashlib
import json
import sys
from pathlib import Path

from . import actors, gltf, music, props, raster as R, sky, terrain


def materials():
    lin = R.hex_linear
    return {
        'floor': gltf.Material('Canyon floor', roughness=0.96, base_texture='floor_albedo.png', sampler='clamp'),
        'top': gltf.Material('Caprock', roughness=0.9, base_texture='top_albedo.png', normal_texture='top_normal.png'),
        'strata': gltf.Material('Strata', roughness=0.92, base_texture='cliff_albedo.png',
                                normal_texture='cliff_normal.png'),
        'far': gltf.Material('Far buttes', roughness=0.95, base_texture='far_albedo.png',
                             emissive=(0.05, 0.056, 0.088)),
        'valley': gltf.Material('Valley haze', base=(0.25, 0.25, 0.25, 1.0), roughness=1.0, base_texture='valley.png',
                                emissive=(0.8, 0.8, 0.8), emissive_texture='valley.png', sampler='clamp'),
        'rock': gltf.Material('Sandstone boulder', roughness=0.9, base_texture='rock_albedo.png'),
        'sage': gltf.Material('Sage', base=lin('#7F9170') + (1.0,), roughness=0.85),
        'sage_dark': gltf.Material('Sage shade', base=lin('#5F7055') + (1.0,), roughness=0.85),
        'canvas': gltf.Material('Canvas', base=lin('#CDBF9C') + (1.0,), roughness=0.88, double_sided=True),
        'fly': gltf.Material('Rain fly', base=lin('#5D6F82') + (1.0,), roughness=0.75, double_sided=True),
        'dark': gltf.Material('Rubber and shadow', base=lin('#2B2724') + (1.0,), roughness=0.95),
        'white': gltf.Material('Rover hull', base=lin('#E6E2D8') + (1.0,), roughness=0.45),
        'crate': gltf.Material('Field case', base=lin('#77735F') + (1.0,), roughness=0.62),
        'steel': gltf.Material('Painted steel', base=lin('#40454D') + (1.0,), metallic=0.55, roughness=0.5),
        'yellow': gltf.Material('Safety yellow', base=lin('#E0AE36') + (1.0,), roughness=0.55),
        'pad': gltf.Material('Lift pad', metallic=0.5, roughness=0.55, base_texture='pad_albedo.png', sampler='clamp'),
        'lantern': gltf.Material('Lantern', base=(0.0, 0.0, 0.0, 1.0), roughness=0.4, emissive=(1.0, 0.72, 0.42)),
        'beacon': gltf.Material('Beacon', base=(0.0, 0.0, 0.0, 1.0), roughness=0.4, emissive=(1.0, 0.9, 0.72)),
    }


def canyon(out):
    table = materials()
    merged = {}
    for part in (terrain.parts(), props.parts()):
        for key, mesh in part.items():
            merged.setdefault(key, type(mesh)()).merge(mesh)
    prims = [(merged[k], table[k]) for k in table if k in merged and merged[k].indices]
    writer = gltf.Writer()
    writer.add_root(gltf.Node('Canyon', prims))
    writer.save(out, 'canyon')
    return gltf.stats(prims)


def build(out):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    terrain.textures(out)
    props.textures(out)
    stats = {'canyon': canyon(out), 'surveyor': actors.surveyor(out), 'mite': actors.mite(out),
             'sample': actors.sample_core(out), 'sky': sky.build(out)}
    audio = music.build(out)
    files = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir())
             if p.is_file() and p.name != 'manifest.json'}
    manifest = {'generator': 'samples/core/art', 'files': files, 'meshes': stats, 'audio': audio,
                'bytes': sum((out / name).stat().st_size for name in files)}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n', encoding='utf-8')
    return manifest


if __name__ == '__main__':
    print(json.dumps(build(sys.argv[1]), indent=2, sort_keys=True))
