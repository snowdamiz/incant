#!/usr/bin/env python3
"""Cook one authored Core variant into a package plus the 0065 capture adapter.

Usage: python3 -B build_variant.py VARIANT PROJECT OUT [--clippy]

One guarded `./tools/cargo build --locked -j2 --profile ci --offline` of
incant_game_player's bin and the packet adapter example (features project,render-gpu)
with INCANT_NATIVE_PROJECT=PROJECT: the player's build script cooks the project once
for both executables. The adapter source is copied from tools/live_hud.rs into the
crate's examples/ only for this build; its hash must equal the frozen value in
evidence/freeze/adapter.json, and `git status -- crates` must be clean afterwards.
OUT/player/{incant_game_player,data} is assembled by the exporter's own
`native_package.data_package`; OUT/adapter holds the adapter. A CPU startup check
(`--package data --ticks 1`) runs once. Not a shipping export (ci profile, no window
feature, no signing). Refuses through lease.py after release or without the grant.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / 'tools'))
import lease  # noqa: E402
from native_package import data_package, read, require, select_camera, sha256  # noqa: E402

EXAMPLE = 'zz_0065_live_hud'
COPY = ROOT / 'crates/incant_game_player/examples' / (EXAMPLE + '.rs')
SOURCE = HERE / 'live_hud.rs'
FREEZE = lease.EVIDENCE / 'freeze' / 'adapter.json'
FEATURES = 'project,render-gpu'


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def crates_clean():
    return subprocess.run(['git', 'status', '--porcelain', '--', 'crates'], cwd=ROOT, capture_output=True,
                          text=True, check=True).stdout == ''


def frozen_source():
    """The adapter source identity is frozen once; every later build must match it."""
    actual = digest(SOURCE)
    if not FREEZE.exists():
        FREEZE.parent.mkdir(exist_ok=True)
        with FREEZE.open('x', encoding='utf-8') as stream:
            json.dump({'source': str(SOURCE.relative_to(ROOT)), 'sha256': actual, 'bytes': SOURCE.stat().st_size,
                       'fence_mirror_of': 'crates/incant_game_player/src/restart/input.rs',
                       'fence_mirror_sha256': digest(ROOT / 'crates/incant_game_player/src/restart/input.rs'),
                       'frozen_at': lease.now()}, stream, indent=1)
    expected = json.loads(FREEZE.read_text(encoding='utf-8'))['sha256']
    require(actual == expected, f'adapter source changed after freeze: {actual} != {expected}')
    return expected


def build(args):
    lease.guard()  # refuse before touching crates/ or the output
    project = args.project.resolve(strict=True)
    out = args.output.absolute().resolve()
    require(not out.exists(), 'output exists; choose a new directory')
    require((ROOT / 'node_modules/typescript/package.json').is_file(), 'worktree-root node_modules link is missing')
    require(crates_clean() and not COPY.exists(), 'crates/ must be clean before the adapter copy')
    source_sha = frozen_source()
    project_hash = sha256(read(project, 64 * 1024**2))
    shutil.copyfile(SOURCE, COPY)
    try:
        require(digest(COPY) == source_sha, 'adapter copy differs from frozen source')
        cargo_args = ['--offline', '--profile', 'ci', '-p', 'incant_game_player', '--bin', 'incant_game_player',
                      '--example', EXAMPLE, '--features', FEATURES, '--message-format=json']
        status, stdout, stderr = lease.cargo('build', cargo_args, label=f'cargo-build-{args.variant}',
                                             extra_env={'INCANT_NATIVE_PROJECT': str(project)})
        clippy = None
        if args.clippy and status == 0:
            clippy = lease.cargo('clippy', ['--offline', '--profile', 'ci', '-p', 'incant_game_player', '--example',
                                            EXAMPLE, '--features', FEATURES, '--', '-D', 'warnings'],
                                 label=f'cargo-clippy-{args.variant}',
                                 extra_env={'INCANT_NATIVE_PROJECT': str(project)})
        require(digest(COPY) == source_sha, 'adapter copy changed during the build')
    finally:
        COPY.unlink(missing_ok=True)
    require(crates_clean(), 'crates/ is not clean after removing the adapter copy')
    require(status == 0, 'build failed:\n' + stderr.read_text(encoding='utf-8', errors='replace')[-6000:])
    if clippy is not None:
        require(clippy[0] == 0, 'clippy failed:\n' + clippy[2].read_text(encoding='utf-8', errors='replace')[-6000:])
    messages = [json.loads(line) for line in stdout.read_text(encoding='utf-8').splitlines() if line.startswith('{')]
    artifacts = {m['target']['name']: m for m in messages if m.get('reason') == 'compiler-artifact'
                 and m['target']['name'] in ('incant_game_player', EXAMPLE) and m.get('executable')}
    require(set(artifacts) == {'incant_game_player', EXAMPLE}, f'missing executables: {sorted(artifacts)}')
    package_id = artifacts['incant_game_player']['package_id']
    producers = [Path(m['out_dir']).resolve() for m in messages if m.get('reason') == 'build-script-executed'
                 and m['package_id'] == package_id and (Path(m['out_dir']) / 'project-export.json').is_file()]
    require(len(producers) == 1, 'missing unique project cook output')
    producer = producers[0]
    receipt = json.loads(read(producer / 'project-export.json', 4 * 1024**2))
    require(receipt['project_sha256'] == project_hash == sha256(read(project, 64 * 1024**2)), 'project changed')
    require(receipt['target'] == 'aarch64-apple-darwin' and receipt['startup_source_parsing'] is False,
            'producer target/mode mismatch')
    out.mkdir(parents=True)
    tree = out / 'player'
    tree.mkdir()
    shutil.copy2(artifacts['incant_game_player']['executable'], tree / 'incant_game_player')
    (out / 'adapter').mkdir()
    shutil.copy2(artifacts[EXAMPLE]['executable'], out / 'adapter' / EXAMPLE)
    camera = select_camera(receipt, None, True)
    data = data_package(project.parent, producer, receipt, tree / 'data', camera)
    (out / 'producer').mkdir()
    for name in ('behavior.rs', 'script-symbols.json', 'project-export.json', 'input-actions.json'):
        shutil.copy2(producer / name, out / 'producer' / name)
    scratch = out / 'startup'
    scratch.mkdir()
    check, check_out, check_err = lease.run(
        [tree / 'incant_game_player', '--package', tree / 'data', '--ticks', '1', '--output', scratch / 'startup.pcm',
         '--ui-viewport', '640x480'], label=f'startup-{args.variant}', timeout=120, cwd=tree,
        env={'PATH': str(tree), 'HOME': str(scratch / 'absent-home'), 'TMPDIR': str(scratch)})
    require(check == 0, 'startup check failed: ' + check_err.read_text(encoding='utf-8')[-4000:])
    report = {'format': 'incant-0065-ci-package', 'variant': args.variant,
              'source_head': subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True,
                                            text=True, check=True).stdout.strip(),
              'cargo_lock_sha256': digest(ROOT / 'Cargo.lock'), 'profile': 'ci', 'features': FEATURES,
              'cargo_target_dir': str(lease.TARGET), 'project': str(project), 'project_sha256': project_hash,
              'scene_id': receipt['scene_id'], 'producer_receipt': receipt, 'producer_dir': str(producer),
              'behavior_sha256': digest(out / 'producer/behavior.rs'),
              'player_sha256': digest(tree / 'incant_game_player'),
              'player_bytes': (tree / 'incant_game_player').stat().st_size,
              'adapter_sha256': digest(out / 'adapter' / EXAMPLE), 'adapter_source_sha256': source_sha,
              'built_executables': {k: v['executable'] for k, v in artifacts.items()},
              'camera': camera, 'data': data, 'startup': json.loads(check_out.read_text(encoding='utf-8')),
              'clippy': None if clippy is None else {'status': clippy[0], 'stderr': str(clippy[2])},
              'limits': ['ci profile with project,render-gpu; not the shipping window export',
                         'unsigned host==target desktop', 'no phone, performance or phase gate']}
    (out / 'package.json').write_text(json.dumps(report, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'output': str(out), 'player_sha256': report['player_sha256'],
                      'adapter_sha256': report['adapter_sha256'], 'behavior_sha256': report['behavior_sha256']}))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('variant')
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--clippy', action='store_true')
    args = parser.parse_args()
    try:
        build(args)
    except lease.Refused as error:
        print(f'refused: {error}', file=sys.stderr)
        sys.exit(3)


if __name__ == '__main__':
    main()
