#!/usr/bin/env python3
"""Cook/build a native behavior, then play a project using the copied executable.

Developer coordinator, not an engine-agent shell capability. Only the trusted
engine Cargo package/build script runs. The project is runtime input, never the
Cargo working directory or a source of build scripts. Scripts must use the native
SDK; project-attached legacy Script components still fail explicitly. TypeScript
is checked and bundled only in the authoring process.
"""
import argparse
from contextlib import contextmanager
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def bounded(path, limit):
    with path.open('rb') as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise ValueError(f'Cook input exceeds {limit} bytes: {path}')
    data.decode('utf-8')
    return data


def cook_typescript(root, entry):
    """Run the pinned text-only compiler, never project configs/plugins/scripts."""
    node = shutil.which('node')
    if not node:
        raise RuntimeError('Node.js is required for the native TypeScript cook')
    environment = dict(os.environ)
    for key in ('NODE_OPTIONS', 'NODE_PATH'):
        environment.pop(key, None)
    with tempfile.TemporaryDirectory(prefix='incant-typescript-cook-') as temporary:
        output = Path(temporary)/'output'
        result = subprocess.run([node, '--max-old-space-size=512',
            str(ROOT/'tools/build_native_script.mjs'), str(root), str(entry), str(output)],
            cwd=ROOT, env=environment, capture_output=True, text=True, encoding='utf-8', timeout=60)
        if result.returncode:
            raise RuntimeError(result.stderr.strip() or 'Native TypeScript cook failed')
        return bounded(output/'behavior.cjs', 1_000_000), bounded(output/'behavior.cjs.map', 4*1024*1024)


@contextmanager
def build_lock(path):
    """Serialize build-and-copy, not gameplay; simultaneous runs own separate bins."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+b') as lock:
        if sys.platform == 'win32':
            import msvcrt
            if lock.seek(0, os.SEEK_END) == 0:
                lock.write(b'\0'); lock.flush()
            while True:
                try:
                    lock.seek(0)
                    msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
                    break
                except OSError as error:
                    if error.errno not in (13, 36):
                        raise
                    time.sleep(0.1)
            try:
                yield
            finally:
                lock.seek(0)
                msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
            try:
                yield
            finally:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


@contextmanager
def prepare(source, source_map=None):
    """Yield a private copied executable after its temporary compile inputs are gone."""
    digest = hashlib.sha256(source).hexdigest()
    # This private target directory is separate from editor/CI/default binaries.
    # A filesystem lock protects the exact build-and-copy pair across callers.
    target = ROOT/'target/native-project-play'
    rustup = shutil.which('rustup')
    if not rustup:
        raise RuntimeError('rustup is required to select the repository toolchain')
    rustc = Path(subprocess.check_output([rustup, 'which', 'rustc'], cwd=ROOT, text=True).strip())
    suffix = '.exe' if sys.platform == 'win32' else ''
    cargo = rustc.with_name('cargo' + suffix)
    env = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get("PATH", ""))
    for key in ('INCANT_NATIVE_SCRIPT_SOURCE', 'INCANT_NATIVE_SCRIPT_MAP', 'INCANT_NATIVE_SCRIPT_NAME'):
        env.pop(key, None)
    with tempfile.TemporaryDirectory(prefix='incant-native-play-') as temporary:
        temporary = Path(temporary)
        cooked_source = temporary/'behavior.js'; cooked_source.write_bytes(source)
        env['INCANT_NATIVE_SCRIPT_SOURCE'] = str(cooked_source)
        env['INCANT_NATIVE_SCRIPT_NAME'] = 'native-project-behavior.js'
        if source_map is not None:
            map_path = temporary/'behavior.js.map'; map_path.write_bytes(source_map)
            env['INCANT_NATIVE_SCRIPT_MAP'] = str(map_path)
        player = temporary/('incant_headless' + suffix)
        with build_lock(target.with_suffix('.lock')):
            subprocess.run([cargo, 'build', '--locked', '--release', '-j2',
                '-p', 'incant_headless', '--bin', 'incant_headless', '--target-dir', target],
                cwd=ROOT, env=env, stdout=sys.stderr, check=True)
            shutil.copy2(target/'release'/player.name, player)
        # The runtime cannot reread even the copied input source. Only the trusted
        # build's embedded bytecode and its hashes survive into this process.
        cooked_source.unlink()
        if source_map is not None:
            map_path.unlink()
        yield player, digest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    scripts = parser.add_mutually_exclusive_group()
    scripts.add_argument('--script', type=Path, help='Native SDK CommonJS; omitted uses the engine no-op behavior')
    scripts.add_argument('--typescript', type=Path, help='Native TypeScript entry, relative to --script-root (default: project directory)')
    parser.add_argument('--script-root', type=Path, help='Bounded local module root; requires --typescript')
    parser.add_argument('--source-map', type=Path, help='Version-3 CommonJS source map, retained in build symbols')
    args, playback = parser.parse_known_args(argv)
    if args.source_map and not args.script:
        parser.error('--source-map requires --script')
    if args.script_root and not args.typescript:
        parser.error('--script-root requires --typescript')
    if any(a == '--script-sha256' or a.startswith('--script-sha256=') for a in playback):
        parser.error('--script-sha256 is owned by the coordinator')
    project = args.project.resolve(strict=True)
    if args.typescript:
        source, source_map = cook_typescript(args.script_root or project.parent, args.typescript)
    else:
        source = bounded(args.script or ROOT/'crates/incant_headless/fixtures/native-play.js', 1_000_000)
        source_map = bounded(args.source_map, 4*1024*1024) if args.source_map else None
    with prepare(source, source_map) as (player, digest):
        result = subprocess.run([player, 'native-play', project,
            '--script-sha256', digest, *playback], check=False)
        return result.returncode



if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, UnicodeError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f'Native play failed: {error}', file=sys.stderr)
        raise SystemExit(1)
