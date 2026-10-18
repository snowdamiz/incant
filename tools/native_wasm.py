"""Resolve authored Wasm data into trusted private build-generated descriptors."""
import json
import os
from pathlib import Path
from native_package import read, require, key
from wasm2c import prepare


def selection(project, scene):
    """Bounded selector admission shared by trusted native exporters."""
    document = json.loads(read(project, 64 * 1024**2))
    require(isinstance(document, dict) and isinstance(document.get('scenes'), dict), 'Invalid project scene table')
    scenes = document['scenes']
    if scene is None:
        require(len(scenes) == 1, 'Choose a scene when project does not contain exactly one')
        scene = next(iter(scenes))
    require(scene in scenes, 'Selected scene is missing')
    require(isinstance(scenes[scene], dict), 'Invalid selected scene')
    program = scenes[scene].get('program')
    if program is None:
        return None
    require(isinstance(program, dict) and {'kind', 'module'} <= set(program) <= {'kind', 'module', 'instances'} and program['kind'] == 'wasm',
            'Unsupported scene program selector')
    entities = scenes[scene].get('entities')
    require(isinstance(entities, dict) and all(isinstance(entity, dict) and isinstance(entity.get('components'), dict) for entity in entities.values()), 'Invalid scene entity table')
    require(not any('Script' in entity['components'] for entity in entities.values()),
            'Wasm scene programs cannot mix with per-entity TypeScript scripts')
    instances = program.get('instances', {})
    require(isinstance(instances, dict) and len(instances) <= 32768
            and all(owner in entities and isinstance(props, dict) for owner, props in instances.items()),
            'Invalid Wasm instance owner catalog')
    # Complete finite/depth/node/byte validation belongs to the typed Rust cook.
    assets = document.get('assets')
    require(isinstance(assets, dict) and isinstance(program['module'], str), 'Invalid module asset reference')
    asset = assets.get(program['module'])
    require(isinstance(asset, dict) and isinstance(asset.get('sha256'), str), 'Missing module asset or fingerprint')
    require(asset.get('kind') == 'wasm', 'Scene module must identify a wasm asset')
    return {'module': program['module'], 'fingerprint': key(asset['sha256'])}


def capture(project, scene, storage, cargo, env, execute, wabt, *, audit_translation=False):
    selected = selection(project, scene)
    if selected is None:
        return None
    fingerprint = selected['fingerprint']
    require(wabt is not None, 'Wasm export requires --wabt pointing to trusted WABT 1.0.42 tools')
    execute([cargo, 'build', '--locked', '-j2', '-p', 'incant_wasm_cook', '--target-dir', storage / 'cook-target'], env=env, timeout=600)
    cooker = storage / 'cook-target/debug' / ('incant_wasm_cook.exe' if os.name == 'nt' else 'incant_wasm_cook')
    captured = storage / 'captured'
    identity = json.loads(execute([cooker, '--capture', project.parent, fingerprint, captured], env=env).stdout)
    bundle = storage / 'wasm-bundle'
    runner = (lambda args: execute(args, env=env).stdout) if audit_translation else None
    report = prepare(bundle, [('scene-program', captured / 'original.wasm')], Path(wabt), cooker,
                     captured / 'prepared.wasm', execute=runner)
    require(report['modules'][0]['input_sha256'] == identity['input_sha256'] and
            report['modules'][0]['prepared_sha256'] == identity['prepared_sha256'], 'Captured translation identity mismatch')
    env.update(INCANT_WASM_BUNDLE=str(bundle), INCANT_NATIVE_WASM_FINGERPRINT=identity['fingerprint'],
               INCANT_NATIVE_WASM_INPUT=identity['input_sha256'], INCANT_NATIVE_WASM_PREPARED=identity['prepared_sha256'],
               INCANT_NATIVE_WASM_REGISTRY=identity['registry_sha256'])
    return identity
