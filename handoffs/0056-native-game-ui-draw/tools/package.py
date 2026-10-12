#!/usr/bin/env python3
"""Fresh ci-profile CPU package player plus one ordinary package for the 0056 fixture.

Usage: python3 -B package.py PROJECT OUT --camera ULID

Follows tools/export_native.py's build step (same package and binary, features
`project,render-input`, `--locked`, `-j2`, the project cooked by the player's
build script through INCANT_NATIVE_PROJECT) with two packet differences: the
`ci` profile and the packet's exclusive CARGO_TARGET_DIR. The package tree,
OUT/player/{incant_game_player,data}, is assembled by the exporter's own
`native_package.data_package`, then started once under an empty environment.
Not a shipping export: no LTO, symbols, signing, window or performance claim.
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
# /tmp is a symlink on macOS; native_package.read refuses linked parents.
SCRATCH = Path('/tmp/incant0056').resolve()
NAME = 'incant_game_player'


def leased():
    lease = json.loads((SCRATCH / 'lease.json').read_text(encoding='utf-8'))
    require(lease.get('packet') == '0056-native-game-ui-draw'
            and lease.get('cargo_target_dir') == str(TARGET_DIR), '0056 lease marker missing')
    require(not (SCRATCH / 'released').exists(), '0056 lease was released; no build')


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--camera', required=True)
    args = parser.parse_args()
    leased()
    project = args.project.resolve(strict=True)
    output = args.output.absolute().resolve()
    require(not output.exists(), 'Output already exists; choose a new directory')
    output.mkdir(parents=True)
    host = 'aarch64-apple-darwin'
    project_hash = sha256(read(project, 64 * 1024**2))
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(('INCANT_NATIVE_SCRIPT_', 'INCANT_NATIVE_PROJECT', 'CARGO_PROFILE_'))
           and k not in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')}
    env.update(INCANT_NATIVE_PROJECT=str(project), CARGO_TARGET_DIR=str(TARGET_DIR))
    command = [str(ROOT / 'tools/cargo'), 'build', '--locked', '--offline', '-j2', '-p', NAME, '--bin', NAME,
               '--features', 'project,render-input', '--profile', 'ci', '--message-format=json']
    print('Compiling fresh ci-profile CPU package player...', flush=True)
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
    tree = output / 'player'
    tree.mkdir()
    # Freeze the executable at once: later builds of this target reuse its path.
    shutil.copy2(artifact['executable'], tree / NAME)
    player_hash = hashlib.sha256((tree / NAME).read_bytes()).hexdigest()
    chosen = select_camera(receipt, args.camera, False)
    data = data_package(project.parent, producer, receipt, tree / 'data', chosen)
    with tempfile.TemporaryDirectory(prefix='incant0056-startup-', dir=SCRATCH) as scratch:
        check = native_process.run([str(tree / NAME), '--package', str(tree / 'data'), '--ticks', '1',
                                    '--output', str(Path(scratch) / 'startup.pcm'), '--ui-viewport', '960x540'],
                                   cwd=tree, env={'PATH': str(tree), 'HOME': str(Path(scratch) / 'absent-home'),
                                                  'TMPDIR': scratch},
                                   text=True, encoding='utf-8', timeout=120)
    require(check.returncode == 0, f'startup check failed: {check.stderr[-4000:]}')
    report = {'format': 'incant-0056-ci-cpu-package', 'source_head': git('rev-parse', 'HEAD'),
              'source_tree': git('rev-parse', 'HEAD^{tree}'),
              'worktree_clean_outside_handoff': git('status', '--porcelain', '--', '.',
                                                    ':(exclude)handoffs/0056-native-game-ui-draw') == '',
              'cargo_lock_sha256': sha256((ROOT / 'Cargo.lock').read_bytes()),
              'command': command, 'profile': 'ci (inherits release, opt-level 1)',
              'features': 'project,render-input', 'cargo_target_dir': str(TARGET_DIR),
              'project_sha256': project_hash, 'scene_id': receipt['scene_id'], 'producer_receipt': receipt,
              'player_sha256': player_hash, 'player_bytes': (tree / NAME).stat().st_size,
              'built_executable': artifact['executable'], 'camera': chosen, 'data': data,
              'startup': json.loads(check.stdout),
              'limits': ['ci profile, not shipping', 'CPU render-input package; no window or GPU device',
                         'no phone, performance or phase gate']}
    write_json(output / 'package.json', report)
    print(json.dumps({'output': str(output), 'player_sha256': player_hash, 'data': str(tree / 'data')}))


if __name__ == '__main__':
    main()
