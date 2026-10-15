"""Trusted native-window cook selection and actual fresh target-output audit.

Generated audit files are evidence, not runtime inputs. No project path or bytes
can choose a native shader. No timing, cache-hit or whole-renderer claim follows.
"""
import json
from pathlib import Path
import re

from native_package import read, require, sha256

ROOT = Path(__file__).resolve().parents[1]
FEATURE = 'cooked-native-shaders'
COOKED = {FEATURE, 'cooked-output', 'cooked-geometry', 'cooked-materials',
          'cooked-sprites', 'cooked-ui', 'cooked-text', 'cooked-clusters', 'cooked-bloom', 'cooked-ssao'}
# Exact existing runtime ABI contracts, independently admitted by each selector.
CONTRACTS = {
    'output': (
        'incant-output-v1;naga=29.0.4;wgpu=29.0.4;hal=e99f5305ded96ff7006f0714d043a7f735bd45c2;msl=2.4;'
        'spv=1.0,no-y-flip;hlsl=6.0;bounds=restrict;loops=bounded;layout=texture0,uniform80'
    ),
    'diagnostic': (
        'incant-diagnostic-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;'
        'msl=2.4,pull15,stride28,sizes0;spv=1.0,no-y-flip;hlsl=6.0,special-b0;bounds=restrict;'
        'loops=bounded'
    ),
    'shadow': (
        'incant-shadow-v1;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,sizes1;'
        'spv=1.0,no-y-flip;hlsl=6.0,camera-b0,material-b1,texture-t0,sampler-indices-t5,special-b2;'
        'bounds=restrict;loops=bounded'
    ),
    'material': (
        'incant-material-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;'
        'msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7;spv=1.0,no-y-flip;'
        'hlsl=6.0,special-b5;bounds=restrict;loops=bounded'
    ),
    'sprite': (
        'incant-sprite-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;'
        'msl=2.4,pull15v14v,stride12-52,sizes1;spv=1.0,no-y-flip;'
        'hlsl=6.0,camera-b0,texture-t0,sampler-indices-t1,special-b1;bounds=restrict;loops=bounded'
    ),
    'ui': (
        'incant-ui-v1;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15i,stride64,sizes1,frame0;spv=1.0,no-y-flip;'
        'hlsl=6.0,frame-b0,texture-t0,sampler-indices-t1,special-b1;bounds=restrict;loops=bounded'
    ),
    'text': (
        'incant-text-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;'
        'msl=2.4,pull15i,stride32,sizes2,frame0,node1;spv=1.0,no-y-flip;'
        'hlsl=6.0,frame-b0,node-b1,texture-t0,sampler-indices-t1,special-b2;bounds=restrict;'
        'loops=bounded'
    ),
    'cluster': (
        'incant-cluster-v1;naga=29.0.4;wgpu=29.0.4;group64x1x1;shared=atomic4+128x4;'
        'msl=2.4,static-threadgroup,sizes4;spv=1.0,implicit-workgroup-layout;'
        'hlsl=6.0,grid-b0,light-t0,counts-u0,masks-u1,sizes-b1,special-b2;bounds=restrict;loops=bounded'
    ),
    'bloom-prefilter': 'incant-bloom-prefilter-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'bloom-downsample': 'incant-bloom-downsample-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'bloom-upsample': 'incant-bloom-upsample-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'bloom-display': 'incant-bloom-display-v1;naga=29.0.4;wgpu=29.0.4;textures0-2,uniform80@1,uniform16@3;msl=2.4,t0-t1,b0-b1;hlsl=6.0,t0-t1,b0-b1,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'ssao-horizontal': 'incant-ssao-horizontal-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2,accessibility@3;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'ssao-vertical': 'incant-ssao-vertical-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2,accessibility@3;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'ssao-prepass': 'incant-ssao-prepass-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1;hlsl=6.0,frame-b0,material-b1,base-t0,sampler-indices-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'ssao-forward': 'incant-ssao-forward-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7,ao-t0,material-t1;hlsl=6.0,ao-t0,material-t1,sampler-indices-t6-t10-t15,special-b5;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
    'ssao-sample': 'incant-ssao-sample-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2;msl=2.4,b0,t0-t1;hlsl=6.0,b0,t0-t1,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded',
}

SOURCES = {
    'output': ['output.wgsl', 'tone_map.wgsl'], 'diagnostic': ['fixture.wgsl'],
    'shadow': ['shadows/caster.wgsl'],
    'material': ['shadows/shade.wgsl', 'lighting/shade.wgsl', 'model_material.wgsl'],
    'sprite': ['sprites/sprite.wgsl'], 'ui': ['ui/ui.wgsl'],
    'text': ['text/text.wgsl'], 'cluster': ['lighting/grid.wgsl'],
    'ssao-prepass': ['model_material.wgsl', 'ambient_occlusion/prepass.wgsl'],
    'ssao-forward': ['shadows/shade.wgsl', 'lighting/shade.wgsl', 'model_material.wgsl'],
    'ssao-sample': ['ambient_occlusion/ao.wgsl'],
    'ssao-horizontal': ['ambient_occlusion/filter.wgsl'], 'ssao-vertical': ['ambient_occlusion/filter.wgsl'],
    'bloom-prefilter': ['bloom/filter.wgsl'], 'bloom-downsample': ['bloom/filter.wgsl'],
    'bloom-upsample': ['bloom/filter.wgsl'], 'bloom-display': ['output.wgsl', 'tone_map.wgsl', 'bloom/output.wgsl'],
}
DIRECTORIES = {'output': 'output-shaders', 'diagnostic': 'diagnostic-shaders',
    'shadow': 'shadow-shaders', 'material': 'material-shaders', 'sprite': 'sprite-shaders',
    'ui': 'ui-shaders', 'text': 'text-shaders', 'cluster': 'cluster-shaders',
    'bloom-prefilter': 'bloom-prefilter-shaders', 'bloom-downsample': 'bloom-downsample-shaders',
    'bloom-upsample': 'bloom-upsample-shaders', 'bloom-display': 'bloom-display-shaders',
    **{'ssao-' + name: 'ssao-' + name + '-shaders' for name in ('prepass', 'forward', 'sample', 'horizontal', 'vertical')}}
BACKENDS = {'Metal', 'Vulkan', 'Dx12'}
TARGETS = {'aarch64-apple-darwin', 'x86_64-apple-darwin',
    'x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc',
    'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'}


def features(mode, route):
    require(mode in (None, 'cooked'), 'Unknown native shader mode')
    require(route in ('headless', 'window'), 'Unknown native shader route')
    require(mode is None or route == 'window', 'Cooked shaders require native-window export')
    return [] if mode is None else [FEATURE]


def target_admission(mode, target):
    require(mode in (None, 'cooked'), 'Unknown native shader mode')
    require(mode is None or isinstance(target, str) and target in TARGETS, 'Unsupported cooked shader target')


def _object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'Duplicate shader receipt key')
        result[key] = value
    return result


def _digest(value):
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value)
            and value != '0' * 64, 'Invalid shader digest')
    return value


def source_bytes(family, root=ROOT):
    paths = SOURCES[family]
    values = [read(Path(root) / 'crates/incant_render/src' / name, 1024**2) for name in paths]
    if family == 'ssao-prepass':
        marker = b'@fragment fn fragment'
        require(values[0].count(marker) == 1, 'Fixed SSAO prepass seam changed')
        values[0] = values[0].split(marker, 1)[0]
    if family == 'ssao-forward':
        seam = b'(1.0-reflectance)*(1.0-metallic)*base.rgb*irradiance'
        require(values[2].count(seam) == 1, 'Fixed SSAO diffuse seam changed')
        values[2] = (b'@group(0) @binding(1) var diffuse_accessibility:texture_2d<f32>;\n'
                     + values[2].replace(seam, b'(' + seam + b')*textureLoad(diffuse_accessibility,vec2i(in.position.xy),0).r'))
    if family == 'bloom-display':
        original = b'var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);'
        require(values[0].count(original) == 1, 'Fixed bloom output seam changed')
        values[0] = values[0].replace(original, b'var color = display.backdrop.rgb; if inside { color = bloom_color(radiance,coverage,position.xy); }')
    return b'\n'.join(values)


def _family(directory, family, root):
    payload = read(directory / 'audit.json', 64 * 1024)
    receipt = json.loads(payload, object_pairs_hook=_object)
    require(isinstance(receipt, dict) and set(receipt) == {'format', 'version', 'contract',
        'source_sha256', 'producer_sha256', 'manifest_sha256', 'artifacts'}, 'Unknown shader receipt fields')
    require(receipt['format'] == 'incant-fixed-shader-build' and type(receipt['version']) is int
            and receipt['version'] == 1 and receipt['contract'] == CONTRACTS[family], 'Shader family contract differs')
    source = source_bytes(family, root)
    require(_digest(receipt['source_sha256']) == sha256(source), 'Shader source identity differs')
    cooker = root / 'crates/incant_shader_cook'
    if family == 'output':
        producer_paths = ['src/lib.rs', 'src/contract.rs', 'src/emit.rs', 'src/manifest.rs', 'src/audit.rs', 'Cargo.toml']
    elif family.startswith('ssao-'):
        producer_paths = ['src/ambient_occlusion/' + name for name in ('mod.rs', 'contract.rs', 'emit.rs')]
        producer_paths += ['src/geometry/' + name for name in ('mod.rs', 'contract.rs', 'interpolation.rs',
            'emit.rs', 'sprite.rs', 'ui.rs', 'text.rs', 'material.rs', 'material_emit.rs')]
        producer_paths += ['src/lib.rs', 'src/audit.rs', 'Cargo.toml', '../incant_render/src/ambient_occlusion/source.rs']
    elif family.startswith('bloom-'):
        producer_paths = ['src/bloom/' + name for name in ('mod.rs', 'contract.rs', 'emit.rs')]
        producer_paths += ['src/audit.rs', 'Cargo.toml', '../incant_render/src/bloom/source.rs']
    elif family == 'cluster':
        producer_paths = ['src/compute/' + name for name in ('mod.rs', 'contract.rs', 'emit.rs',
            'static_workgroup.rs', 'spirv_workgroup.rs')]
        producer_paths += ['src/audit.rs', 'Cargo.toml']
    else:
        producer_paths = ['src/geometry/' + name for name in ('mod.rs', 'contract.rs', 'interpolation.rs',
            'emit.rs', 'sprite.rs', 'ui.rs', 'text.rs', 'material.rs', 'material_emit.rs')]
        producer_paths += ['src/audit.rs', 'Cargo.toml']
    producer = b'\n'.join(read(cooker / name, 1024**2) for name in producer_paths)
    require(_digest(receipt['producer_sha256']) == sha256(producer), 'Shader producer identity differs')
    manifest = read(directory / 'manifest.rs', 64 * 1024)
    require(_digest(receipt['manifest_sha256']) == sha256(manifest), 'Embedded shader manifest differs')
    stages = {'Compute'} if family == 'cluster' else {'Vertex', 'Fragment'}
    expected = {(backend, stage) for backend in BACKENDS for stage in stages}
    artifacts = receipt['artifacts']
    require(isinstance(artifacts, list) and len(artifacts) == len(expected), 'Missing shader stages')
    seen = set()
    total = 0
    compact = family == 'cluster'
    generated = ('const MANIFEST_CONTRACT: &str = ' + json.dumps(receipt['contract']) + ';\n'
        + 'const SOURCE_HASH: [u8;' + ('' if compact else ' ') + '32] = '
        + str(list(bytes.fromhex(receipt['source_sha256']))) + ';\n'
        + 'const PRODUCER_HASH: [u8;' + ('' if compact else ' ') + '32] = '
        + str(list(bytes.fromhex(receipt['producer_sha256']))) + ';\n'
        + 'const ARTIFACTS: [Artifact;' + ('' if compact else ' ') + str(len(expected)) + '] = [\n')
    for row in artifacts:
        require(isinstance(row, dict) and set(row) == {'backend', 'stage', 'entry', 'file', 'bytes', 'sha256'},
                'Unknown shader artifact fields')
        require(isinstance(row['backend'], str) and isinstance(row['stage'], str), 'Invalid shader stage identity')
        key = (row['backend'], row['stage'])
        require(key in expected and key not in seen, 'Missing/duplicate shader stage')
        seen.add(key)
        require(row['file'] == '-'.join(key) + '.shader', 'Invalid shader artifact path')
        require(isinstance(row['entry'], str) and re.fullmatch('[A-Za-z_][A-Za-z_0-9]{0,255}', row['entry']),
                'Invalid shader entry point')
        require(type(row['bytes']) is int and 0 < row['bytes'] <= 1024**2, 'Invalid shader artifact size')
        data = read(directory / row['file'], 1024**2)
        require(len(data) == row['bytes'] and sha256(data) == _digest(row['sha256']), 'Shader artifact differs')
        if row['backend'] == 'Vulkan':
            require(len(data) >= 20 and len(data) % 4 == 0 and data[:8] == bytes.fromhex('0302230700000100')
                    and data[16:20] == bytes(4), 'Invalid shader SPIR-V header')
        else:
            data.decode('utf-8')
        generated += ('Artifact { backend: wgpu::Backend::' + row['backend'] + ', stage: Stage::'
            + row['stage'] + ', entry: ' + json.dumps(row['entry']) + ', hash: '
            + str(list(bytes.fromhex(row['sha256']))) + ', bytes: include_bytes!('
            + json.dumps(row['file']) + ') },\n')
        total += len(data)
    require(seen == expected, 'Incomplete shader stages')
    require(manifest == (generated + '];\n').encode(), 'Shader audit differs from embedded manifest')
    return {**receipt, 'audit_sha256': sha256(payload), 'artifact_bytes': total}


def audit(messages, mode, *, target_dir, target, profile, physics_mode, root=ROOT):
    if mode is None:
        return None
    features(mode, 'window')
    target_admission(mode, target)
    require(physics_mode in ('fast', 'deterministic'), 'Unknown shader physics mode')
    require(isinstance(messages, list) and 0 < len(messages) <= 65536
            and all(isinstance(m, dict) for m in messages), 'Invalid shader compiler messages')
    require([m.get('success') for m in messages if m.get('reason') == 'build-finished'] == [True],
            'Shader audit needs one completed fresh build')
    destination = Path(target_dir).resolve() / target / profile
    host = Path(target_dir).resolve() / profile
    found, ignored, kinds, renderer_id = {}, set(), set(), None
    for row in messages:
        if row.get('reason') != 'compiler-artifact':
            continue
        require(isinstance(row.get('target'), dict) and isinstance(row['target'].get('name'), str),
                'Invalid shader compiler unit')
        name = row['target']['name']
        if name not in ('incant_render', 'incant_game_player'):
            continue
        require(row.get('filenames') and all(isinstance(p, str) and Path(p).is_absolute() for p in row['filenames']),
                'Missing shader target outputs')
        paths = [Path(p).resolve() for p in row['filenames']]
        if all(p.is_relative_to(host) for p in paths):
            ignored.add(name)
            continue
        require(all(p.is_relative_to(destination) for p in paths), 'Unknown shader output location')
        kind = row['target']['kind']
        require(kind == ['lib'] or name == 'incant_game_player' and kind == ['bin'], 'Unexpected shader target unit')
        flags = row['features']
        require(isinstance(flags, list) and all(isinstance(f, str) for f in flags)
                and len(set(flags)) == len(flags), 'Invalid shader compiler features')
        flags = set(flags)
        wanted = COOKED if name == 'incant_render' else {FEATURE}
        require({flag for flag in flags if flag.startswith('cooked-')} == wanted,
                'Missing/extra actual cooked shader features: ' + name)
        require(name not in found or found[name] == flags, 'Conflicting shader compiler units')
        found[name] = flags
        if name == 'incant_game_player':
            kinds.add(kind[0])
            require('native-window' in flags, 'Cooked player has no native window')
        else:
            expected_physics = set() if physics_mode == 'fast' else {'physics-' + physics_mode}
            require(flags == COOKED | expected_physics, 'Unexpected renderer target features')
            require(isinstance(row.get('package_id'), str) and row['package_id']
                    and (renderer_id is None or renderer_id == row['package_id']), 'Multiple/missing renderer packages')
            renderer_id = row['package_id']
    require(set(found) == {'incant_render', 'incant_game_player'} and kinds == {'lib', 'bin'},
            'Missing actual renderer/player shader units')
    outputs = [m.get('out_dir') for m in messages if m.get('reason') == 'build-script-executed'
               and m.get('package_id') == renderer_id]
    require(len(outputs) == 1 and isinstance(outputs[0], str) and Path(outputs[0]).is_absolute()
            and Path(outputs[0]).resolve().is_relative_to(destination), 'Missing unique target shader producer')
    output = Path(outputs[0])
    # Only the compiler-selected private OUT_DIR is read; no project/cache loader.
    families = {name: _family(output / directory, name, Path(root)) for name, directory in DIRECTORIES.items()}
    require(sum(f['artifact_bytes'] for f in families.values()) <= 99 * 1024**2, 'Shader catalog exceeds bound')
    return {'mode': 'cooked', 'target': target, 'profile': profile,
        'compiler_features': {name: sorted(flags) for name, flags in found.items()},
        'ignored_build_host': sorted(ignored), 'families': families,
        'runtime_wgsl_in_selected_ordinary_catalog': False,
        'limits': ['Seventeen fixed ordinary, bloom and SSAO families only; backend native compiler still runs during startup',
            'Motion/temporal and MetalFX remain separate source routes; partial SSAO cooked selection still refuses',
            'No parser-symbol exclusion, driver cache hit, asynchronous fallback or shader-stall gate claim']}
