#!/usr/bin/env python3
"""Verify the numerical Planar Sample in an ordinary source-free shipping player.

Consumes a successful verify.py output. This is gameplay/export evidence, not
finished presentation, reference-device performance or full-save acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def inventory(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('development', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    development = args.development.resolve(strict=True)
    reference = json.loads((development / 'result.json').read_text())
    reference['reports'] = {name: json.loads((development / (name + '.json')).read_text()) for name in reference['runs']}
    runpy.run_path(str(ROOT / 'samples/planar/verify.py'))['verify_reports'](reference['reports'])
    assert reference['authoring_undo_redo_reopen'] and reference['authoring_unchanged']
    assert reference['gameplay_sha256'] == digest(ROOT / 'samples/planar/gameplay.ts')
    before = inventory(development)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    authoring = out / 'authoring'
    shutil.copytree(development / 'authoring', authoring)
    assert digest(authoring / 'gameplay.ts') == reference['gameplay_sha256']
    project_sha = digest(authoring / 'planar.json')
    sys.path.insert(0, str(ROOT / 'tools'))
    import export_native
    exported = export_native.export(authoring / 'planar.json', out / 'package', camera='00000000000000000000000007')
    assert exported['project_sha256'] == project_sha
    assert exported['build_storage_removed_before_startup']
    assert exported['startup_source_parsing'] is False
    removed_maps = {}
    for path in (out / 'package/symbols').rglob('*'):
        if path.is_file() and (path.name.endswith('.map') or path.name == 'script-symbols.json'):
            removed_maps[str(path.relative_to(out))] = digest(path)
            path.unlink()
    # Only the new proof-owned copy is removed. Original development evidence stays intact.
    shutil.rmtree(authoring)
    player_root = out / 'package/player'
    player = player_root / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    package_before = inventory(player_root)
    environment = {'PATH': str(player_root), 'TMPDIR': str(out), 'TMP': str(out), 'TEMP': str(out)}
    environment.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR') if key in os.environ})
    reports = {}
    for name, expected in reference['reports'].items():
        recording = out / f'{name}-input.json'
        observe = out / f'{name}-observe.json'
        shutil.copyfile(development / recording.name, recording)
        shutil.copyfile(development / observe.name, observe)
        pcm = out / f'{name}.pcm'
        argv = [str(player), '--package', str(player_root / 'data'), '--ticks', str(expected['tick']),
                '--input-recording', str(recording), '--observe', str(observe), '--output', str(pcm)]
        result = subprocess.run(argv, cwd=player_root, env=environment, text=True,
                                capture_output=True, timeout=180)
        write(out / f'{name}-process.json', {'argv': argv, 'exit': result.returncode,
                                           'stdout': result.stdout, 'stderr': result.stderr})
        assert result.returncode == 0, (name, result.stderr)
        actual = json.loads(result.stdout)
        assert actual['tick'] == expected['tick'] and actual['observations']['passed']
        for field in ('frames', 'pcm_sha256', 'logs', 'observations'):
            assert actual[field] == expected[field], (name, field)
        assert digest(pcm) == actual['pcm_sha256']
        write(out / f'{name}.json', actual)
        reports[name] = {'report_sha256': digest(out / f'{name}.json'),
                         'pcm_sha256': digest(pcm), 'ticks': actual['tick']}
    assert inventory(player_root) == package_before
    assert inventory(development) == before
    assert not authoring.exists()
    write(out / 'result.json', {'passed': True, 'runs': reports, 'export': exported,
        'authoring_deleted': True, 'removed_source_maps': removed_maps,
        'package_files': package_before, 'original_development_unchanged': True,
        'source_sha256': reference['gameplay_sha256'], 'helper_sha256': digest(Path(__file__)),
        'limitations': ['Numerical tilemap/sprite room, no finished level or GPU evidence',
                        'Host desktop only, no reference-device or performance gate',
                        'No physics checkpoint continuation claim']})
    print(json.dumps({'passed': True, 'runs': len(reports), 'result': str(out / 'result.json')}))


if __name__ == '__main__':
    main()
