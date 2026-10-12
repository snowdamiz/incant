#!/usr/bin/env python3
"""Build a ci-profile project player for one authored project and package its
cooked data with the engine's own tools/native_package.data_package.

    python3 -B package.py <project.json> <new-output-dir> --camera <ULID>

This is a review package, not the audited shipping export (tools/export_native.py):
the player uses the ci profile in the packet's leased target. The guard lease is
checked before the build and before the player's startup check.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
WORKTREE = HERE.parents[2]
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(WORKTREE / 'tools'))
import guard  # noqa: E402
from native_package import data_package, select_camera  # noqa: E402


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--camera', required=True)
    args = parser.parse_args()
    project = args.project.resolve(strict=True)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    guard.check()
    project_sha = digest(project)
    os.environ['INCANT_NATIVE_PROJECT'] = str(project)
    code = guard.cargo(['build', '-p', 'incant_game_player', '--bin', 'incant_game_player',
                        '--features', 'project,native-window'])
    if code:
        raise SystemExit(f'player build failed with exit {code}')
    producers = []
    for receipt_path in (guard.TARGET / 'ci/build').glob('incant_game_player-*/out/project-export.json'):
        receipt = json.loads(receipt_path.read_text())
        if receipt.get('project_sha256') == project_sha:
            producers.append((receipt_path.stat().st_mtime, receipt_path.parent, receipt))
    if not producers:
        raise SystemExit('no project cook output matches this project')
    _, producer, receipt = max(producers, key=lambda item: item[0])
    player_dir = out / 'player'
    player_dir.mkdir()
    binary = guard.TARGET / 'ci/incant_game_player'
    shutil.copy2(binary, player_dir / 'incant_game_player')
    camera = select_camera(receipt, args.camera, True)
    data = data_package(project.parent, producer.resolve(), receipt, player_dir / 'data', camera)
    guard.check()
    startup = subprocess.run([str(player_dir / 'incant_game_player'), '--package', str(player_dir / 'data'),
                              '--ticks', '1', '--output', str(out / 'startup.pcm')],
                             cwd=player_dir, capture_output=True, text=True, timeout=120)
    guard.record({'event': 'player-startup', 'exit': startup.returncode,
                  'player_sha256': digest(player_dir / 'incant_game_player')})
    if startup.returncode:
        raise SystemExit(startup.stderr[-4000:])
    (out / 'startup.pcm').unlink()
    report = {'format': 'incant-0061-review-package', 'profile': 'ci', 'features': 'project,native-window',
              'project': str(project), 'project_sha256': project_sha, 'camera_hex': camera,
              'player_sha256': digest(player_dir / 'incant_game_player'), 'producer': str(producer),
              'receipt': receipt, 'data': data, 'startup': json.loads(startup.stdout)}
    (out / 'package.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'output': str(out), 'camera_hex': camera, 'player_sha256': report['player_sha256']}))


if __name__ == '__main__':
    main()
