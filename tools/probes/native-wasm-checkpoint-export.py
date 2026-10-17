#!/usr/bin/env python3
"""Reviewed bounded authored Wasm checkpoint proof; no GPU or window route."""
import argparse
import json
import os
from pathlib import Path
import shutil
import sys
import time
import types

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import rust_guest
import wasm_persistent
from native_package import write_json
from probes.native_wasm_restart.inputs import record
from probes.native_wasm_restart.owner import tree_bytes
from probes.native_wasm_restart.verify import digest, require
from probes.native_wasm_checkpoint.inputs import verify
from probes.native_wasm_checkpoint.owner import CheckpointOwner
from probes.native_wasm_checkpoint.route import run
from probes.native_wasm_checkpoint.replay import replay


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packet', type=Path, required=True)
    args = parser.parse_args()
    packet = args.packet.resolve(strict=True)
    plan = verify(ROOT, packet)
    grant = json.loads((packet / 'root-grant.json').read_text())
    require(grant['packet_sha256'] == digest(packet / 'packet.json'), 'Exact root grant')
    require(sys.platform == 'darwin' and Path(sys.executable).resolve() == Path(plan['python']).resolve(),
            'Pinned macOS Python')
    require(shutil.disk_usage(packet).free >= 28 * 1024**3, '28GiB cold launch admission')
    require(not any((packet / name).exists() or (packet / name).is_symlink()
                    for name in ('wrapper-started.json', 'result.json', 'output')), 'Once-only packet')
    for name, value in os.environ.items():
        if name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER',
                    'RUSTC_WORKSPACE_WRAPPER') or name.startswith('CARGO_PROFILE_') or name.endswith('_RUSTFLAGS'):
            require(not value, 'Unexpected compiler override: ' + name)
    os.umask(0o077)
    target = Path(plan['target'])
    require(not target.exists() and not target.is_symlink(), 'Create-only separate cold target')
    target.mkdir(mode=0o700)
    started = time.monotonic()
    with (packet / 'wrapper-started.json').open('x') as file:
        json.dump({'pid': os.getpid(), 'packet_sha256': digest(packet / 'packet.json'),
                   'target_identity': [target.stat().st_dev, target.stat().st_ino]}, file)
    previous = dict(os.environ)
    environment = {k: v for k, v in previous.items() if k in ('HOME', 'USER', 'LOGNAME')}
    environment.update(PATH=plan['path'], RUSTUP_TOOLCHAIN='1.99.0-aarch64-apple-darwin',
        CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='2', RAYON_NUM_THREADS='2',
        RUSTUP_IO_THREADS='2', CARGO_INCREMENTAL='0', PYTHONDONTWRITEBYTECODE='1',
        DEVELOPER_DIR=plan['developer_dir'], LANG='en_US.UTF-8', TMPDIR=str(packet / 'output/private-builds'))
    os.environ.clear()
    os.environ.update(environment)
    owner = None
    final = {'passed': False, 'wrapper_pid': os.getpid(), 'source_head': plan['source_head'],
             'packet_sha256': digest(packet / 'packet.json'), 'fresh_target': True}
    try:
        owner = CheckpointOwner(Path(plan['output']), target)
        with owner:
            owner.check(force=True)
            guests = owner.output / 'guest-build'
            guests.mkdir(mode=0o700)
            guest = guests / 'game.wasm'
            deadline = min(owner.deadline, time.monotonic() + 180)

            def managed(argv, **kwargs):
                kwargs.pop('capture_output', None)
                kwargs.setdefault('cwd', ROOT)
                kwargs.setdefault('env', environment)
                require(time.monotonic() < deadline, '180s total guest stage')
                value = owner.run(argv, timeout=min(120, deadline - time.monotonic()), **kwargs)
                require(value.returncode == 0, 'Trusted SDK guest compile failed; original logs retained')
                return value

            def check_output(argv, **kwargs):
                wants_text = kwargs.get('text', False)
                kwargs['text'] = True
                text = managed(argv, **kwargs).stdout
                return text if wants_text else text.encode()

            original_subprocess = rust_guest.subprocess
            rust_guest.subprocess = types.SimpleNamespace(run=managed, check_output=check_output)
            try:
                rust_guest.compile(plan['rustc'], ROOT / 'tools/probes/native_wasm_checkpoint/guest.rs', guest,
                                   env=environment)
            finally:
                rust_guest.subprocess = original_subprocess
            require(guest.stat().st_size <= 1024**2 and wasm_persistent.declaration(guest.read_bytes()) == {
                'bytes': 24, 'schema_sha256': (bytes([23]) * 32).hex()}, 'Actual guest persistent declaration')
            write_json(owner.output / 'guest.json', {'sdk': rust_guest.provenance(), 'artifact': record(guest),
                'source': record(ROOT / 'tools/probes/native_wasm_checkpoint/guest.rs')})
            command = [plan['cargo'], 'build', '--locked', '--offline', '--profile', 'ci', '-j2',
                       '--target-dir', target, '-p', 'incant_headless', '--bin', 'incant_headless',
                       '--message-format=json']
            result = owner.run(command, cwd=ROOT, env=environment, timeout=1200)
            require(result.returncode == 0, 'Fresh normal headless build failed; original logs retained')
            messages = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
            artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact'
                         and m['target']['name'] == 'incant_headless' and m.get('executable')]
            require(len(artifacts) == 1, 'Unique actual headless executable')
            actual = artifacts[0]
            require(set(actual['features']) == {'default', 'physics-deterministic'}
                    and actual['profile']['test'] is False and actual['fresh'] is False,
                    'Fresh ordinary default-feature headless')
            original = Path(actual['executable']).resolve(strict=True)
            require(original.is_relative_to(target.resolve()), 'Actual headless from separate target')
            require(0 < original.stat().st_size <= 256 * 1024**2, 'Bounded normal executable copy')
            binary = owner.output / 'incant_headless'
            with binary.open('xb') as file:
                file.write(original.read_bytes())
            binary.chmod(0o700)
            write_json(owner.output / 'headless.json', {'source_head': plan['source_head'],
                'cargo_artifact': actual, 'copied': record(binary)})
            result = run(owner, ROOT, plan, binary, guest, environment)
            write_json(owner.output / 'proof.json', result)
            owner.passed = True
        write_json(owner.output / 'data-replay.json', replay(owner.output))
        final['passed'] = True
    except BaseException as error:
        final['error'] = repr(error)
    finally:
        try:
            verify(ROOT, packet)
            final['source_tools_unchanged'] = True
        except BaseException as error:
            final.update(passed=False, postcheck_error=repr(error))
        if owner is not None:
            try:
                final['children_absence'] = owner.absent_children()
                require(final['children_absence'] and all(r['pid_and_group_absent'] is True
                        for r in final['children_absence']), 'All observed children absent')
                require(all(g['cleanup_confirmed'] is True for g in owner.groups), 'Managed cleanup complete')
                final.update(owner_sha256=digest(owner.output / 'owner.json'),
                             process_inventory_sha256=digest(owner.output / 'process-inventory.jsonl'),
                             peaks=owner.peaks)
            except BaseException as error:
                final.update(passed=False, cleanup_error=repr(error))
        try:
            final.update(seconds=time.monotonic() - started, target_allocated_bytes=tree_bytes(target))
            if final['seconds'] > 5400 or final['target_allocated_bytes'] > 12 * 1024**3:
                final.update(passed=False, final_bound_error='Aggregate time or target storage ceiling')
        except BaseException as error:
            final.update(passed=False, resource_postcheck_error=repr(error))
        try:
            write_json(packet / 'result.json', final)
        finally:
            os.environ.clear()
            os.environ.update(previous)
    print(json.dumps(final))
    return 0 if final['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
