#!/usr/bin/env python3
"""Cook one authored Core variant into a package plus the 0069 adapter. DRAFT for root review.

Usage: python3 -B build_variant.py VARIANT PROJECT OUT [--clippy]

One guarded `./tools/cargo build --locked -j2 --profile ci --offline` of
incant_game_player's bin and the packet adapter example (features project,render-gpu)
with INCANT_NATIVE_PROJECT=PROJECT: the player's build script cooks the project once
for both executables.

The build is bound to one freeze generation (lease.freeze): the current source,
tools, lockfile, child tool chain and host must match the latest generation before
compiling and still match it afterwards, and the adapter source copied from
tools/result_hud.rs must be byte-identical to the frozen tool. The copy lives in the
crate's examples/ only for the build, and crates/ must equal the frozen state (the
reviewed renderer source included) before the copy and after its removal.

OUT/player/{incant_game_player,data} is assembled by the exporter's own
`native_package.data_package`; OUT/adapter holds the adapter; OUT/producer keeps the
cook receipts. package.json records the freeze generation and digest and an
inventory of every file under player/, adapter/ and producer/; capture.py refuses a
package from another generation or with any changed file. A CPU startup check
(`--package data --ticks 1`) runs once and must leave the inventory unchanged. Not a
shipping export (ci profile, no window feature, no signing). Refuses through
lease.py after release or without the grant.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / 'tools'))
import lease  # noqa: E402
from native_package import data_package, read, require, select_camera, sha256  # noqa: E402

EXAMPLE = 'zz_0069_result_hud'
COPY = ROOT / 'crates/incant_game_player/examples' / (EXAMPLE + '.rs')
SOURCE = HERE / 'result_hud.rs'
FEATURES = 'project,render-gpu'
PACKAGED = ('player', 'adapter', 'producer')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def crates_as_frozen(frozen):
    """crates/ holds exactly the frozen uncommitted state: nothing added or changed."""
    actual = {p: e for p, e in lease.status_entries(ROOT).items() if p.startswith('crates/')}
    expected = {p: e for p, e in frozen['identity']['source']['uncommitted'].items() if p.startswith('crates/')}
    return actual == expected


def inventory(out):
    """Every packaged file and its digest; capture.py checks it before each run."""
    return {str(p.relative_to(out)): digest(p) for p in sorted(out.rglob('*'))
            if p.is_file() and p.relative_to(out).parts[0] in PACKAGED}


def build(args):
    lease.guard()  # refuse before touching crates/ or the output
    project = args.project.resolve(strict=True)
    out = args.output.absolute().resolve()
    require(not out.exists(), 'output exists; choose a new directory')
    require((ROOT / 'node_modules/typescript/package.json').is_file(), 'worktree-root node_modules link is missing')
    frozen = lease.freeze()  # the current generation; refuses if anything changed since it
    source_sha = frozen['identity']['tools'][SOURCE.name]
    require(digest(SOURCE) == source_sha, 'adapter source differs from the freeze')
    require(not COPY.exists() and crates_as_frozen(frozen), 'crates/ differs from the freeze before the adapter copy')
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
    require(crates_as_frozen(frozen), 'crates/ differs from the freeze after removing the adapter copy')
    after = lease.freeze()
    require((after['generation'], after['sha256']) == (frozen['generation'], frozen['sha256']),
            'the freeze generation changed during the build')
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
    files = inventory(out)
    scratch = out / 'startup'
    scratch.mkdir()
    check, check_out, check_err = lease.run(
        [tree / 'incant_game_player', '--package', tree / 'data', '--ticks', '1', '--output', scratch / 'startup.pcm',
         '--ui-viewport', '640x480'], label=f'startup-{args.variant}', timeout=120, cwd=scratch,
        env={'PATH': str(tree), 'TMPDIR': str(scratch)})
    require(check == 0, 'startup check failed: ' + check_err.read_text(encoding='utf-8')[-4000:])
    require(inventory(out) == files, 'the startup check changed a packaged file')
    report = {'format': 'incant-0069-ci-package', 'variant': args.variant,
              'freeze': {'generation': frozen['generation'], 'sha256': frozen['sha256']},
              'source_head': frozen['identity']['source']['head'],
              'cargo_lock_sha256': frozen['identity']['cargo_lock_sha256'], 'profile': 'ci', 'features': FEATURES,
              'cargo_target_dir': str(lease.TARGET), 'project': str(project), 'project_sha256': project_hash,
              'scene_id': receipt['scene_id'], 'producer_receipt': receipt, 'producer_dir': str(producer),
              'behavior_sha256': digest(out / 'producer/behavior.rs'),
              'player_sha256': digest(tree / 'incant_game_player'),
              'player_bytes': (tree / 'incant_game_player').stat().st_size,
              'adapter_sha256': digest(out / 'adapter' / EXAMPLE), 'adapter_source_sha256': source_sha,
              'inventory': files,
              'built_executables': {k: v['executable'] for k, v in artifacts.items()},
              'camera': camera, 'data': data, 'startup': json.loads(check_out.read_text(encoding='utf-8')),
              'clippy': None if clippy is None else {'status': clippy[0], 'stderr': str(clippy[2])},
              'limits': ['ci profile with project,render-gpu; not the shipping window export',
                         'unsigned host==target desktop', 'no phone, performance or phase gate']}
    (out / 'package.json').write_text(json.dumps(report, indent=1) + '\n', encoding='utf-8')
    print(json.dumps({'output': str(out), 'freeze': report['freeze'], 'player_sha256': report['player_sha256'],
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
