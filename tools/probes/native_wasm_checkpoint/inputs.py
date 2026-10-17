"""Data/source admission for the reviewed create-only cold shipping proof."""
import json
from pathlib import Path

from ..native_wasm_restart.inputs import record, source_inventory
from ..native_wasm_restart.verify import require


def verify(root, packet):
    manifest = json.loads((packet / 'packet.json').read_text())
    require(manifest['format'] == 'incant-wasm-checkpoint-shipping-packet-v1', 'Packet format')
    required = {'plan.json', 'source.json', 'tools.json', 'run.py'}
    require(required <= manifest['files'].keys(), 'Complete reviewed packet')
    for name, expected in manifest['files'].items():
        require(Path(name).name == name and record(packet / name) == expected, 'Packet file identity')
    source = json.loads((packet / 'source.json').read_text())
    require(source_inventory(root) == source, 'Frozen clean source changed')
    plan = json.loads((packet / 'plan.json').read_text())
    require(plan['source_head'] == source['head'] and plan['source_root'] == str(root), 'Source producer')
    require(plan['format'] == 'incant-authored-wasm-checkpoint-shipping-v1', 'Plan format')
    require(plan['target'] in ('/private/tmp/incant-wasm-persistent-shipping-20261017-target',
                               '/private/tmp/incant-wasm-timers-shipping-20261017-target',
                               '/private/tmp/incant-wasm-coroutines-shipping-20261017-target')
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
    for name in (plan['python'], plan['cargo'], plan['rustc'],
                 str(Path(plan['wabt']) / 'bin/wasm2c'), str(Path(plan['wabt']) / 'bin/wat2wasm')):
        require(name in tools, 'Required trusted tool binding')
    for name in plan['absent_configuration']:
        require(not Path(name).exists() and not Path(name).is_symlink(), 'Ambient configuration appeared')
    return plan
