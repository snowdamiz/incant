#!/usr/bin/env python3
"""Bounded authored SDK Wasm restart proof through the ordinary shipping exporter.

Run only a reviewed frozen packet on an exclusively leased FONT target. No
fixture-only player features, native window, GPU or performance claim.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from probes.native_wasm_restart.inputs import record, verify_packet
from probes.native_wasm_restart.owner import GIB, Owner, process_rows
from probes.native_wasm_restart.route import run
from probes.native_wasm_restart.verify import digest, require
from native_package import write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packet', type=Path, required=True)
    args = parser.parse_args()
    packet = args.packet.resolve(strict=True)
    plan = verify_packet(ROOT, packet)
    grant = json.loads((packet / 'root-grant.json').read_text())
    require(grant['packet_sha256'] == digest(packet / 'packet.json'), 'Exact root packet grant')
    require(sys.platform == 'darwin', 'Reviewed macOS owner only')
    require(Path(sys.executable).resolve() == Path(plan['python']).resolve(), 'Pinned Python')
    require(shutil.disk_usage(packet).free >= 20 * GIB, '20 GiB launch admission')
    # Reject dangerous ambient compile switches, matching the ordinary exporter.
    for name, value in os.environ.items():
        if name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER',
                    'RUSTC_WORKSPACE_WRAPPER') or name.startswith('CARGO_PROFILE_') or name.endswith('_RUSTFLAGS'):
            require(not value, f'Unset {name}')
    old_environment = dict(os.environ)
    environment = {k: v for k, v in os.environ.items() if k in ('HOME', 'USER', 'LOGNAME')}
    environment.update(PATH=plan['path'], RUSTUP_TOOLCHAIN='1.99.0-aarch64-apple-darwin',
                       CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', RAYON_NUM_THREADS='2',
                       RUSTUP_IO_THREADS='2', CARGO_INCREMENTAL='0', PYTHONDONTWRITEBYTECODE='1',
                       DEVELOPER_DIR=plan['developer_dir'], LANG='en_US.UTF-8',
                       TMPDIR=str(Path(plan['output']) / 'private-builds'))
    # export_native deliberately obtains its trusted build environment from here.
    os.environ.clear()
    os.environ.update(environment)
    try:
        with Owner(Path(plan['output']), Path(plan['target']), plan['target_ceiling_bytes']) as owner:
            owner.check(force=True)
            command = [plan['cargo'], 'build', '--locked', '--offline', '--profile', 'ci', '-j2',
                       '--target-dir', plan['target'], '-p', 'incant_headless',
                       '--features', 'wasmtime-host', '--message-format=json']
            result = owner.run(command, cwd=ROOT, env=environment, timeout=600)
            require(result.returncode == 0, 'Current headless build failed; see retained diagnostics')
            messages = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
            artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact'
                         and m['target']['name'] == 'incant_headless' and m.get('executable')]
            require(len(artifacts) == 1, 'Unique actual headless artifact')
            actual = artifacts[0]
            require(set(actual['features']) == {'default', 'physics-deterministic', 'wasmtime-host'},
                    'Exact headless feature selection')
            source = Path(actual['executable']).resolve(strict=True)
            require(source.is_relative_to(Path(plan['target']).resolve()), 'Actual headless target')
            binary = owner.output / 'incant_headless'
            shutil.copy2(source, binary)
            write_json(owner.output / 'headless.json', {'cargo_artifact': actual,
                'copied': record(binary), 'source_head': plan['source_head']})
            owner.deadline = min(owner.deadline, time.monotonic() + 3600)
            value = run(owner, ROOT, plan, binary, environment)
            verify_packet(ROOT, packet)
            owner.check(force=True)
            rows = process_rows()
            require(all(g['cleanup_confirmed'] and not any(g['pid'] in (p, group) for p, group, _ in rows)
                        for g in owner.groups), 'Positive final owned PID/group absence')
            value['groups_absent'] = [g['pid'] for g in owner.groups]
            value['packet_sha256'] = digest(packet / 'packet.json')
            write_json(owner.output / 'result.json', value)
            owner.passed = True
            print(json.dumps({'passed': True, 'output': str(owner.output),
                              'player_sha256': value['player_sha256']}))
    finally:
        os.environ.clear()
        os.environ.update(old_environment)


if __name__ == '__main__':
    main()
