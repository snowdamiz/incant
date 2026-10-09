#!/usr/bin/env python3
"""Validate Incant's texture fixtures with the pinned Khronos CLI, without installing it."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
VERSION = '4.4.2'
PACKAGES = {
    'arm64': '500bd8f9d63358c3f3a0d83b724c8574436a72c37dc0e4bad90ec1ca38032c3c',
    'x86_64': 'efecc685ab891a6e119a9fdc8cbe038e135f9a367eb2f5d8a059553f947f1fea',
}

def prepare(directory):
    arch = platform.machine()
    if platform.system() != 'Darwin' or arch not in PACKAGES:
        raise RuntimeError('Automatic validator setup requires macOS; supply --ktx on other hosts')
    name = f'KTX-Software-{VERSION}-Darwin-{arch}'
    package = directory / 'ktx.pkg'
    url = f'https://github.com/KhronosGroup/KTX-Software/releases/download/v{VERSION}/{name}.pkg'
    with urllib.request.urlopen(url, timeout=60) as response:
        with package.open('wb') as output:
            shutil.copyfileobj(response, output)
    if hashlib.sha256(package.read_bytes()).hexdigest() != PACKAGES[arch]:
        raise RuntimeError('Khronos package checksum mismatch')
    expanded = directory / 'expanded'
    subprocess.run(['pkgutil', '--expand-full', str(package), str(expanded)], check=True)
    tool = directory / 'ktx'
    shutil.copy2(expanded / f'{name}-tools.pkg/Payload/usr/local/bin/ktx', tool)
    shutil.copy2(expanded / f'{name}-library.pkg/Payload/usr/local/lib/libktx.{VERSION}.dylib', directory / 'libktx.4.dylib')
    return tool

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ktx', type=Path, help='Use an existing Khronos validator')
    parser.add_argument('--output', type=Path, default=ROOT / 'artifacts/texture-format-probe')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(ROOT / 'tools/cargo'), 'run', '-p', 'incant_assets', '--release', '--locked', '--example', 'texture_probe', '--', str(output)], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix='incant-ktx-validator-') as temporary:
        tool = args.ktx.resolve() if args.ktx else prepare(Path(temporary))
        version = subprocess.check_output([str(tool), '--version'], text=True).strip()
        records = []
        for name in ('color', 'linear', 'hdr'):
            path = output / f'{name}.ktx2'
            result = subprocess.run([str(tool), 'validate', str(path)], capture_output=True, text=True)
            records.append({'fixture': name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        report = {'validator': version, 'binary_sha256': hashlib.sha256(tool.read_bytes()).hexdigest(), 'checks': records}
        (output / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
        if any(r['exit_code'] or r['stderr'] or r['stdout'] for r in records):
            raise RuntimeError('Texture validation produced diagnostics; see validation.json')
        print(f'{version}: all three texture formats passed without diagnostics')

if __name__ == '__main__':
    main()
