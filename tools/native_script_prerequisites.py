"""Early checkout diagnostics, not a replacement for the trusted script cook."""
import json

from native_package import read, require


COMPILERS = ('typescript', '@swc/core', '@jridgewell/sourcemap-codec')


def check(project_bytes, scene, engine):
    """Refuse missing compiler packages before spending time on a fresh Cargo build.

    Inspect only the selected scene's declarations. Never import or execute
    project code, install packages, or accept a project-supplied compiler path.
    The actual cook still checks and uses the pinned compiler implementations.
    """
    document = json.loads(project_bytes)
    require(isinstance(document, dict) and isinstance(document.get('scenes'), dict),
            'Invalid project scene table')
    scenes = document['scenes']
    if scene is None:
        require(len(scenes) == 1, 'Choose a scene when project does not contain exactly one')
        scene = next(iter(scenes))
    require(scene in scenes and isinstance(scenes[scene], dict), 'Selected scene is missing or invalid')
    entities = scenes[scene].get('entities', {})
    require(isinstance(entities, dict), 'Invalid scene entity table')
    attached = any(isinstance(entity, dict) and isinstance(entity.get('components'), dict)
                   and 'Script' in entity['components'] for entity in entities.values())
    if not attached:
        return
    message = "Authored TypeScript needs this engine checkout's pinned Node dependencies; run npm ci in the engine root"
    require((engine / 'tools/build_native_script.mjs').is_file(),
            'Missing trusted TypeScript compiler driver in the engine checkout')
    manifest = json.loads(read(engine / 'package.json', 256 * 1024))
    require(isinstance(manifest, dict) and isinstance(manifest.get('devDependencies'), dict),
            'Invalid engine compiler dependency manifest')
    expected = manifest['devDependencies']
    for name in COMPILERS:
        manifest = engine / 'node_modules' / name / 'package.json'
        require(manifest.is_file(), f'{message} (missing {name})')
        installed = json.loads(read(manifest, 256 * 1024))
        require(isinstance(expected.get(name), str) and isinstance(installed, dict)
                and installed.get('version') == expected[name], f'{message} (version mismatch: {name})')
