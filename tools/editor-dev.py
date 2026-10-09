#!/usr/bin/env python3
"""Build the local editor; on macOS create an unsigned development app bundle."""
import argparse
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--no-build', action='store_true', help='Package an existing binary without rebuilding')
    args = parser.parse_args()
    if not args.no_build:
        for command in (
            ['npm', 'run', 'build', '--workspace', 'editor/ui'],
            ['node', 'tools/build_bridge.mjs'],
            ['tools/cargo', 'build', '-p', 'incant_editor', '--features', 'custom-protocol', '--locked']
            + (['--release'] if args.release else []),
        ):
            subprocess.run(command, cwd=ROOT, check=True)
    executable = ROOT / 'target' / ('release' if args.release else 'debug') / (
        'incant_editor.exe' if sys.platform == 'win32' else 'incant_editor'
    )
    if not executable.is_file():
        parser.error(f'Missing compiled editor: {executable}')
    if sys.platform != 'darwin':
        print(executable)
        return
    config = json.loads((ROOT / 'editor/app/tauri.conf.json').read_text())
    bundle = ROOT / 'artifacts/Incant.app/Contents'
    (bundle / 'MacOS').mkdir(parents=True, exist_ok=True)
    # Copy to a temporary file before replacement; never overwrite a running inode.
    temporary = bundle / 'MacOS/incant_editor.next'
    shutil.copy2(executable, temporary)
    temporary.replace(bundle / 'MacOS/incant_editor')
    (bundle / 'Resources').mkdir(exist_ok=True)
    shutil.copy2(ROOT / 'editor/ui/brand/Incant.icns', bundle / 'Resources/Incant.icns')
    with (bundle / 'Info.plist').open('wb') as stream:
        plistlib.dump({
            'CFBundleExecutable': 'incant_editor',
            'CFBundleIconFile': 'Incant.icns',
            'CFBundleIdentifier': config['identifier'],
            'CFBundleName': config['productName'],
            'CFBundleDisplayName': config['productName'],
            'CFBundlePackageType': 'APPL',
            'CFBundleShortVersionString': config['version'],
            'CFBundleVersion': '1',
            'LSMinimumSystemVersion': '12.1',
            'NSHighResolutionCapable': True,
        }, stream)
    print(bundle.parent)


if __name__ == '__main__':
    main()
