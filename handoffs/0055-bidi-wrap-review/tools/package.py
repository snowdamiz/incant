#!/usr/bin/env python3
"""Fresh ci-profile window player plus ordinary native packages for the 0055 fixture.

Usage: python3 -I -B package.py PROJECT OUT --camera NAME=ULID [--camera NAME=ULID]

This follows tools/export_native.py's build step for `--mode window` (same
package, binary, features `project,native-window`, `--locked`, `-j2`, project
cooked by the player's build script through INCANT_NATIVE_PROJECT) with two
granted differences: the `ci` profile (not `shipping`) and the packet's
exclusive CARGO_TARGET_DIR. Each camera gets one ordinary package tree,
NAME/player/{incant_game_player,data}, assembled by the exporter's own
`native_package.data_package`, and the exporter's CPU startup check
(`--package data --ticks 1`) under an empty environment. Not a shipping export:
no LTO, no symbol packaging, no signing and no performance claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process  # noqa: E402
from native_package import data_package, read, require, select_camera, sha256, write_json  # noqa: E402

TARGET_DIR = Path('/tmp/incant-agent-native-target')
# /tmp is a symlink on macOS; native_package.read refuses linked paths.
SCRATCH = Path('/tmp/incant0055').resolve()
NAME = 'incant_game_player'


def granted():
    work = json.loads((HERE.parent / 'work.json').read_text(encoding='utf-8'))
    require(work.get('native_gpu_granted') is True and work.get('exclusive_to') == '0055-bidi-wrap-review'
            and work.get('cargo_target_dir') == str(TARGET_DIR), 'work.json lacks the 0055 build grant')
    require(not (SCRATCH / 'released').exists(), '0055 lease was released; no build')


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--camera', action='append', required=True, help='NAME=ULID')
    args = parser.parse_args()
    granted()
    project = args.project.resolve(strict=True)
    output = args.output.absolute().resolve()
    require(not output.exists(), 'Output already exists; choose a new directory')
    cameras = dict(item.split('=', 1) for item in args.camera)
    output.mkdir(parents=True)
    host = 'aarch64-apple-darwin'
    project_hash = sha256(read(project, 64 * 1024**2))
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(('INCANT_NATIVE_SCRIPT_', 'INCANT_NATIVE_PROJECT', 'CARGO_PROFILE_'))
           and k not in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')}
    env.update(INCANT_NATIVE_PROJECT=str(project), CARGO_TARGET_DIR=str(TARGET_DIR))
    command = [str(ROOT / 'tools/cargo'), 'build', '--locked', '--offline', '-j2', '-p', NAME, '--bin', NAME,
               '--features', 'project,native-window', '--profile', 'ci', '--message-format=json']
    print('Compiling fresh ci-profile window player...', flush=True)
    with (output / 'build.log').open('w', encoding='utf-8') as stderr, \
            (output / 'cargo.jsonl').open('w', encoding='utf-8') as stdout:
        result = native_process.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, timeout=3600)
    require(result.returncode == 0, 'Player build failed:\n' + (output / 'build.log').read_text()[-6000:])
    messages = [json.loads(line) for line in (output / 'cargo.jsonl').read_text(encoding='utf-8').splitlines()
                if line.startswith('{')]
    artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact'
                 and m['target']['name'] == NAME and m.get('executable')]
    require(len(artifacts) == 1, 'Missing unique player artifact')
    artifact = artifacts[0]
    producers = [Path(m['out_dir']).resolve() for m in messages if m.get('reason') == 'build-script-executed'
                 and m['package_id'] == artifact['package_id'] and (Path(m['out_dir']) / 'project-export.json').is_file()]
    require(len(producers) == 1, 'Missing unique project cook output')
    producer = producers[0]
    receipt = json.loads(read(producer / 'project-export.json', 4 * 1024**2))
    require(receipt['project_sha256'] == project_hash == sha256(read(project, 64 * 1024**2)),
            'Project changed during build')
    require(receipt['target'] == host and receipt['startup_source_parsing'] is False, 'Producer target/mode mismatch')
    # Freeze the executable at once: later builds of this target reuse its path.
    frozen = output / 'bin' / NAME
    frozen.parent.mkdir()
    shutil.copy2(artifact['executable'], frozen)
    player_hash = hashlib.sha256(frozen.read_bytes()).hexdigest()
    packages = {}
    for name, camera in cameras.items():
        tree = output / name / 'player'
        tree.mkdir(parents=True)
        shutil.copy2(frozen, tree / NAME)
        chosen = select_camera(receipt, camera, True)
        data = data_package(project.parent, producer, receipt, tree / 'data', chosen)
        with tempfile.TemporaryDirectory(prefix='incant0055-startup-', dir=SCRATCH) as scratch:
            home = Path(scratch) / 'absent-home'
            check = native_process.run([str(tree / NAME), '--package', str(tree / 'data'), '--ticks', '1',
                                        '--output', str(Path(scratch) / 'startup.pcm')],
                                       cwd=tree, env={'PATH': str(tree), 'HOME': str(home), 'TMPDIR': scratch},
                                       text=True, encoding='utf-8', timeout=120)
        require(check.returncode == 0, f'{name} startup check failed: {check.stderr[-4000:]}')
        require(hashlib.sha256((tree / NAME).read_bytes()).hexdigest() == player_hash, 'Player copy mismatch')
        packages[name] = {'camera_ulid': camera, 'camera': chosen, 'player': str(tree / NAME),
                          'data': data, 'startup': json.loads(check.stdout),
                          'startup_backend': 'CPU render input; no window or GPU device'}
    report = {'format': 'incant-0055-ci-window-packages', 'source_head': git('rev-parse', 'HEAD'),
              'source_tree': git('rev-parse', 'HEAD^{tree}'), 'worktree_clean_outside_handoff':
              git('status', '--porcelain', '--', '.', ':(exclude)handoffs/0055-bidi-wrap-review') == '',
              'cargo_lock_sha256': sha256((ROOT / 'Cargo.lock').read_bytes()),
              'command': command, 'profile': 'ci (inherits release, opt-level 1)',
              'features': 'project,native-window', 'cargo_target_dir': str(TARGET_DIR),
              'project_sha256': project_hash, 'scene_id': receipt['scene_id'], 'producer_receipt': receipt,
              'player_sha256': player_hash, 'player_bytes': frozen.stat().st_size,
              'built_executable': artifact['executable'], 'packages': packages,
              'limits': ['ci profile, not shipping', 'unsigned host==target desktop',
                         'no phone, performance or phase gate']}
    write_json(output / 'package.json', report)
    print(json.dumps({'output': str(output), 'player_sha256': player_hash,
                      'packages': {k: v['player'] for k, v in packages.items()}}))


if __name__ == '__main__':
    main()
