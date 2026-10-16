"""Data/source admission for the fixed local proof; no native tool execution."""
import json
import os
from pathlib import Path
import subprocess

from .verify import GUEST_SHA256, SOURCE_SHA256, digest, require


def git(root, *args):
    return subprocess.run(['/usr/bin/git', '-C', str(root), *args], check=True, capture_output=True,
                          timeout=30).stdout


def record(path):
    path = Path(path)
    if path.is_symlink():
        return {'link': os.readlink(path)}
    require(path.is_file(), f'Missing regular input: {path}')
    return {'bytes': path.stat().st_size, 'sha256': digest(path),
            'executable': bool(path.stat().st_mode & 0o111)}


def source_inventory(root):
    require(not git(root, 'status', '--porcelain', '--untracked-files=all'), 'Source must be clean')
    files = git(root, 'ls-files', '-z').decode().split('\0')[:-1]
    return {'head': git(root, 'rev-parse', 'HEAD').decode().strip(),
            'tree': git(root, 'rev-parse', 'HEAD^{tree}').decode().strip(),
            'files': {name: record(root / name) for name in files}}


def verify_packet(root, packet):
    manifest = json.loads((packet / 'packet.json').read_text())
    require({'plan.json', 'source.json', 'tools.json', 'predecessors.json', 'run.py'} <= manifest['files'].keys(),
            'Incomplete frozen packet')
    for name, expected in manifest['files'].items():
        path = packet / name
        require(path.parent == packet and record(path) == expected, f'Packet identity: {name}')
    source = json.loads((packet / 'source.json').read_text())
    require(source_inventory(root) == source, 'Frozen source changed')
    tools = json.loads((packet / 'tools.json').read_text())
    for name, expected in tools.items():
        require(record(Path(name)) == expected, f'Tool/input identity: {name}')
        if 'link' in expected:
            resolved = str(Path(name).resolve(strict=True))
            require(resolved in tools and 'sha256' in tools[resolved], f'Unbound tool link: {name}')
    plan = json.loads((packet / 'plan.json').read_text())
    require(plan['source_root'] == str(root) and plan['source_head'] == source['head'], 'Plan source')
    # Executable plans are trusted review inputs, never authored project content.
    require(plan['format'] == 'incant-authored-wasm-restart-plan-v1', 'Plan format')
    require(plan['target'] == '/private/tmp/incant-agent-font-target', 'Original FONT target')
    require(plan['target_ceiling_bytes'] == 65948704768, 'Preserve original proposed FONT ceiling')
    require(plan['target_baseline_bytes'] == 63801221120, 'Preserve charged FONT baseline')
    require(sum(p.stat().st_size for p in packet.iterdir() if p.is_file()) <= 32 * 1024**2,
            'Frozen packet retained-byte allowance')
    for name in plan['absent_configuration']:
        require(not Path(name).exists() and not Path(name).is_symlink(), f'New ambient configuration: {name}')
    for name, expected in json.loads((packet / 'predecessors.json').read_text()).items():
        require(record(Path(name)) == expected, f'Preserved predecessor: {name}')
    for name in (plan['python'], plan['cargo'], plan['guest'], plan['sdk_receipt'],
                 str(Path(plan['wabt']) / 'bin/wasm2c'), str(Path(plan['wabt']) / 'bin/wat2wasm')):
        require(name in tools, f'Missing required tool binding: {name}')
    sdk = json.loads(Path(plan['sdk_receipt']).read_text())
    require(len(sdk['source_sha256']) == 36, 'Complete qualified SDK source inventory')
    for name, expected in sdk['source_sha256'].items():
        require(digest(root / name) == expected, f'Qualified guest source changed: {name}')
    require(digest(root / 'sdk/rust/samples/session_restart.rs') == SOURCE_SHA256, 'Guest source')
    require(digest(Path(plan['guest'])) == GUEST_SHA256, 'Exact qualified request-once guest')
    return plan
