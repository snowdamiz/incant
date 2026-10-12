#!/usr/bin/env python3
"""Fresh ci-profile binaries and one ordinary package for the 0058 review.

Usage: python3 -B build.py headless OUT
       python3 -B build.py player PROJECT OUT --camera ULID

`headless` builds and freezes the fixture author, incant_headless. `player`
follows tools/export_native.py's build step for `--mode window`: the ordinary
incant_game_player binary with features `project,native-window`, `--locked`,
`-j2`, the project cooked by the player's own build script through
INCANT_NATIVE_PROJECT. Packet differences: the `ci` profile and the leased
CARGO_TARGET_DIR. The package tree, OUT/player/{incant_game_player,data}, is
assembled by the exporter's own `native_package.data_package`, then the
exporter's CPU startup check (`--package data --ticks 1`) runs once under an
empty environment. Not a shipping export: no LTO, symbol packaging, signing or
performance claim. Every build refuses without a valid (unexpired, unreleased)
0058 lease and is logged with its exit status in the 0058 process ledger.
"""
import argparse
import datetime
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

PACKET = '0058-native-text-retirement'
TARGET_DIR = Path.home() / '.codex/incant-native-target'
# native_package.read refuses linked parents; resolve the scratch path.
SCRATCH = (Path.home() / '.codex/incant-evidence/0058').resolve()
PLAYER = 'incant_game_player'


def leased():
    if (SCRATCH / 'released').exists():
        raise SystemExit('0058 lease was released; refusing')
    try:
        lease = json.loads((SCRATCH / 'lease.json').read_text(encoding='utf-8'))
        expiry = datetime.datetime.fromisoformat(lease['expires_at'])
    except (OSError, ValueError, KeyError):
        raise SystemExit('no valid 0058 lease; refusing')
    if lease.get('packet') != PACKET or lease.get('cargo_target_dir') != str(TARGET_DIR) or \
            datetime.datetime.now(datetime.timezone.utc) >= expiry:
        raise SystemExit('no valid 0058 lease (foreign, other target or expired); refusing')


def ledger(line):
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
    with (SCRATCH / 'processes.log').open('a', encoding='utf-8') as log:
        log.write(f'{stamp} {line}\n')


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def version(*command):
    return subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def identity():
    return {'source_head': git('rev-parse', 'HEAD'), 'source_tree': git('rev-parse', 'HEAD^{tree}'),
            'worktree_clean_outside_handoff': git('status', '--porcelain', '--', '.',
                                                  f':(exclude)handoffs/{PACKET}') == '',
            'cargo_lock_sha256': sha256((ROOT / 'Cargo.lock').read_bytes()),
            'rustc': version(version('rustup', 'which', 'rustc'), '--version'),
            'cargo': version(str(ROOT / 'tools/cargo'), '--version')}


def cargo(output, label, extra_env, args):
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(('INCANT_NATIVE_SCRIPT_', 'INCANT_NATIVE_PROJECT', 'CARGO_PROFILE_'))
           and k not in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')}
    env.update(CARGO_TARGET_DIR=str(TARGET_DIR), TMPDIR=str(SCRATCH / 'tmp'), **extra_env)
    command = [str(ROOT / 'tools/cargo'), 'build', '--locked', '--offline', '-j2', *args,
               '--profile', 'ci', '--message-format=json']
    ledger(f'build {label} started: {" ".join(command[1:])}')
    with (output / 'build.log').open('w', encoding='utf-8') as stderr, \
            (output / 'cargo.jsonl').open('w', encoding='utf-8') as stdout:
        result = native_process.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, timeout=3600)
    ledger(f'build {label} ended: exit {result.returncode} (process group reaped by native_process)')
    require(result.returncode == 0, f'{label} build failed:\n' + (output / 'build.log').read_text()[-6000:])
    messages = [json.loads(line) for line in (output / 'cargo.jsonl').read_text(encoding='utf-8').splitlines()
                if line.startswith('{')]
    return command, messages


def artifact(messages, name):
    found = [m for m in messages if m.get('reason') == 'compiler-artifact'
             and m['target']['name'] == name and m.get('executable')]
    require(len(found) == 1, f'Missing unique {name} artifact')
    return found[0]


def headless(output):
    command, messages = cargo(output, 'incant_headless', {}, ['-p', 'incant_headless', '--bin', 'incant_headless'])
    built = artifact(messages, 'incant_headless')
    frozen = output / 'incant_headless'
    shutil.copy2(built['executable'], frozen)
    report = {'format': 'incant-0058-headless', **identity(), 'command': command,
              'profile': 'ci (inherits release, opt-level 1)', 'cargo_target_dir': str(TARGET_DIR),
              'built_executable': built['executable'], 'sha256': sha256(frozen.read_bytes()),
              'bytes': frozen.stat().st_size}
    write_json(output / 'headless.json', report)
    print(json.dumps({'binary': str(frozen), 'sha256': report['sha256']}))


def player(project, output, camera):
    # The TypeScript cook resolves typescript from the worktree root only.
    require((ROOT / 'node_modules' / 'typescript' / 'package.json').is_file(),
            'worktree-root node_modules link (TypeScript cook) is missing')
    host = 'aarch64-apple-darwin'
    project_hash = sha256(read(project, 64 * 1024**2))
    command, messages = cargo(output, PLAYER, {'INCANT_NATIVE_PROJECT': str(project)},
                              ['-p', PLAYER, '--bin', PLAYER, '--features', 'project,native-window'])
    built = artifact(messages, PLAYER)
    producers = [Path(m['out_dir']).resolve() for m in messages if m.get('reason') == 'build-script-executed'
                 and m['package_id'] == built['package_id'] and (Path(m['out_dir']) / 'project-export.json').is_file()]
    require(len(producers) == 1, 'Missing unique project cook output')
    producer = producers[0]
    receipt = json.loads(read(producer / 'project-export.json', 4 * 1024**2))
    require(receipt['project_sha256'] == project_hash == sha256(read(project, 64 * 1024**2)),
            'Project changed during build')
    require(receipt['target'] == host and receipt['startup_source_parsing'] is False, 'Producer target/mode mismatch')
    tree = output / 'player'
    tree.mkdir()
    # Freeze the executable at once: later builds of this target reuse its path.
    shutil.copy2(built['executable'], tree / PLAYER)
    player_hash = hashlib.sha256((tree / PLAYER).read_bytes()).hexdigest()
    chosen = select_camera(receipt, camera, True)
    data = data_package(project.parent, producer, receipt, tree / 'data', chosen)
    with tempfile.TemporaryDirectory(prefix='startup-', dir=SCRATCH / 'tmp') as scratch:
        ledger(f'startup check started: {tree / PLAYER} --package data --ticks 1 (CPU, no window)')
        check = native_process.run([str(tree / PLAYER), '--package', str(tree / 'data'), '--ticks', '1',
                                    '--output', str(Path(scratch) / 'startup.pcm'), '--ui-viewport', '320x240'],
                                   cwd=tree, env={'PATH': str(tree), 'HOME': str(Path(scratch) / 'absent-home'),
                                                  'TMPDIR': scratch},
                                   text=True, encoding='utf-8', timeout=120)
        ledger(f'startup check ended: exit {check.returncode}')
    require(check.returncode == 0, f'startup check failed: {check.stderr[-4000:]}')
    require(hashlib.sha256((tree / PLAYER).read_bytes()).hexdigest() == player_hash, 'Player copy changed')
    fonts = sorted((tree / 'data' / 'fonts').rglob('*'))
    report = {'format': 'incant-0058-ci-window-package', **identity(), 'command': command,
              'profile': 'ci (inherits release, opt-level 1)', 'features': 'project,native-window',
              'cargo_target_dir': str(TARGET_DIR), 'project_sha256': project_hash, 'scene_id': receipt['scene_id'],
              'producer_receipt': receipt, 'player_sha256': player_hash, 'player_bytes': (tree / PLAYER).stat().st_size,
              'built_executable': built['executable'], 'camera': chosen, 'data': data,
              'font_cache_sha256': {str(p.relative_to(tree / 'data')): hashlib.sha256(p.read_bytes()).hexdigest()
                                    for p in fonts if p.is_file()},
              'startup': json.loads(check.stdout),
              'startup_route': 'CPU render-input at --ui-viewport 320x240; no window or GPU device',
              'limits': ['ci profile, not shipping', 'unsigned host==target desktop',
                         'no phone, performance or phase gate']}
    write_json(output / 'package.json', report)
    print(json.dumps({'output': str(output), 'player_sha256': player_hash, 'data': str(tree / 'data')}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    first = commands.add_parser('headless')
    first.add_argument('output', type=Path)
    second = commands.add_parser('player')
    second.add_argument('project', type=Path)
    second.add_argument('output', type=Path)
    second.add_argument('--camera', required=True)
    args = parser.parse_args()
    leased()
    output = args.output.absolute().resolve()
    require(not output.exists(), 'Output already exists; choose a new directory')
    output.mkdir(parents=True)
    if args.command == 'headless':
        headless(output)
    else:
        player(args.project.resolve(strict=True), output, args.camera)


if __name__ == '__main__':
    main()
