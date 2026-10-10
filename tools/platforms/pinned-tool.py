#!/usr/bin/env python3
"""Reuse one version-checked platform tool, without caching Cargo's bin directory."""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TOOLS = {
    'wasm-bindgen': ('wasm-bindgen-cli', '0.2.126'),
    'cargo-ndk': ('cargo-ndk', '4.1.2'),
}


def cache_key(tool):
    """Separate exact versions, host ABI/architecture and pinned compiler inputs."""
    host = [platform.system(), platform.machine()]
    if platform.system() == 'Linux':
        release = platform.freedesktop_os_release()
        host += [release['ID'], release['VERSION_ID']]
    else:
        raise ValueError('Pinned platform tool caching currently supports Linux runners only')
    compiler = hashlib.sha256((ROOT / 'rust-toolchain.toml').read_bytes()).hexdigest()[:16]
    return '-'.join(['incant-cli-v1', tool, TOOLS[tool][1], *host, compiler])


def correct_version(binary, tool):
    try:
        environment = dict(os.environ)
        if tool == 'cargo-ndk':
            # cargo-ndk refuses direct invocation without Cargo's usual marker.
            # Execute this exact cached file, never an unrelated PATH copy.
            environment['CARGO'] = shutil.which('cargo') or 'cargo'
        result = subprocess.run([str(binary), '--version'], capture_output=True,
                                text=True, timeout=15, check=True, env=environment)
        return result.stdout.strip() == f'{tool} {TOOLS[tool][1]}'
    except (OSError, subprocess.SubprocessError):
        return False


def ensure(tool, directory):
    directory.mkdir(parents=True, exist_ok=True)
    binary = directory / tool
    if correct_version(binary, tool):
        print(f'Reusing {tool} {TOOLS[tool][1]} from {binary}', flush=True)
        return binary
    print(f'Installing pinned {tool}: cache absent, unreadable or version mismatched', flush=True)
    # Only the selected executable is published to the cache. Install metadata,
    # compiler intermediates, companion tools and rustup proxies stay outside it.
    with tempfile.TemporaryDirectory(prefix='incant-tool-install-') as storage:
        command = ['cargo', 'install', TOOLS[tool][0], '--version', '=' + TOOLS[tool][1],
                   '--locked', '--bin', tool, '--root', storage, '--target-dir',
                   str(Path(storage) / 'build'), '-j2']
        subprocess.run(command, check=True)
        installed = Path(storage) / 'bin' / tool
        if not correct_version(installed, tool):
            raise ValueError(f'Installed {tool} did not report the pinned version')
        # Copy to the destination filesystem before atomic publication. Never
        # expose a partial executable or overwrite any Cargo/rustup directory.
        with tempfile.NamedTemporaryFile(dir=directory, delete=False) as staged:
            staged_path = Path(staged.name)
            try:
                staged.write(installed.read_bytes())
                staged.flush()
                staged_path.chmod(0o755)
                os.replace(staged_path, binary)
            finally:
                staged_path.unlink(missing_ok=True)
    return binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tool', choices=TOOLS)
    parser.add_argument('directory', type=Path, help='Isolated directory for this executable')
    parser.add_argument('--prepare', action='store_true', help='Write cache metadata, without executing/installing')
    args = parser.parse_args()
    directory = args.directory.resolve()
    if '\n' in str(directory) or '\r' in str(directory):
        raise ValueError('Tool directory cannot contain newlines')
    if args.prepare:
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            output.write(f'key={cache_key(args.tool)}\npath={directory / args.tool}\n')
        return
    ensure(args.tool, directory)
    with open(os.environ['GITHUB_PATH'], 'a') as output:
        output.write(str(directory) + '\n')


if __name__ == '__main__':
    main()
