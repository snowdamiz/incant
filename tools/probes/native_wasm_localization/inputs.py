"""Data/source admission for the reviewed create-only cold shipping proof."""
import json
from pathlib import Path

from ..native_wasm_restart.inputs import record, source_inventory
from ..native_wasm_restart.verify import require


def verify(root, packet):
    manifest = json.loads((packet / 'packet.json').read_text())
    require(manifest['format'] == 'incant-wasm-localization-shipping-packet-v1', 'Packet format')
    required = {'plan.json', 'source.json', 'tools.json', 'node-dependencies.json', 'run.py'}
    require(required <= manifest['files'].keys(), 'Complete reviewed packet')
    for name, expected in manifest['files'].items():
        require(Path(name).name == name and record(packet / name) == expected, 'Packet file identity')
    source = json.loads((packet / 'source.json').read_text())
    require(source_inventory(root) == source, 'Frozen clean source changed')
    plan = json.loads((packet / 'plan.json').read_text())
    require(plan['source_head'] == source['head'] and plan['source_root'] == str(root), 'Source producer')
    require(plan['format'] == 'incant-authored-wasm-localization-shipping-v1', 'Plan format')
    require(plan['target'] == '/private/tmp/incant-wasm-localization-shipping-20261017-target'
            and plan['target_ceiling_allocated_bytes'] == 12 * 1024**3
            and plan['aggregate_seconds'] == 5400 and plan['jobs'] == plan['rayon_threads'] == 2,
            'Reviewed fresh allocation')
    require(plan['scratch_bytes'] == 8 * 1024**3 and plan['retained_bytes'] == 2 * 1024**3
            and plan['rss_bytes'] == 8 * 1024**3 and plan['launch_floor_bytes'] == 28 * 1024**3
            and plan['free_floor_bytes'] == 6 * 1024**3, 'Reviewed resource ceilings')
    require(plan['output'] == str(packet / 'output'), 'Packet-owned output')
    require(sum(p.stat().st_size for p in packet.iterdir() if p.is_file()) <= 32 * 1024**2,
            'Packet reserved32MiB bound')
    tools = json.loads((packet / 'tools.json').read_text())
    for name, expected in tools.items():
        path = Path(name)
        require(record(path) == expected, 'Pinned tool identity: ' + name)
        if 'link' in expected:
            require(str(path.resolve(strict=True)) in tools, 'Bind symlink destination')
    require(node_inventory(root) == json.loads((packet / 'node-dependencies.json').read_text()),
            'Exact private pinned TypeScript dependency closure')
    for name in (plan['python'], plan['cargo'], plan['rustc'], plan['node'],
                 str(Path(plan['wabt']) / 'bin/wasm2c'), str(Path(plan['wabt']) / 'bin/wat2wasm')):
        require(name in tools, 'Required trusted tool binding')
    for name in plan['absent_configuration']:
        require(not Path(name).exists() and not Path(name).is_symlink(), 'Ambient configuration appeared')
    return plan


def node_inventory(root):
    directory = root / 'node_modules'
    require(directory.is_dir() and not directory.is_symlink(), 'Private dependency directory')
    paths = sorted(directory.rglob('*'))
    require(len(paths) <= 1024 and all(not path.is_symlink() for path in paths), 'Bounded link-free tool closure')
    files = {str(path.relative_to(directory)): record(path) for path in paths if path.is_file()}
    require(sum(row['bytes'] for row in files.values()) <= 64 * 1024**2, '64MiB preparatory Node dependency bound')
    packages = {'typescript': '5.9.3', '@swc/core': '1.16.13', '@swc/core-darwin-arm64': '1.16.13',
                '@swc/counter': '0.1.3', '@swc/types': '0.1.28', '@jridgewell/sourcemap-codec': '1.6.0'}
    lock = json.loads((root / 'package-lock.json').read_text())['packages']
    for name, version in packages.items():
        require(json.loads((directory / name / 'package.json').read_text())['version'] == version
                and lock['node_modules/' + name]['version'] == version, 'Pinned dependency: ' + name)
    return files


def environment_admission(environment):
    for name, value in environment.items():
        forbidden = name in {
            'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTDOC', 'RUSTDOCFLAGS',
            'CARGO_ENCODED_RUSTDOCFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
            'CARGO_BUILD_RUSTC', 'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER'}
        forbidden |= name.startswith(('CARGO_PROFILE_', 'CARGO_FEATURE_')) or name.endswith('_RUSTFLAGS')
        require(not (forbidden and value), 'Unexpected compiler override: ' + name)
