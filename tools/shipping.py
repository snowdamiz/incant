#!/usr/bin/env python3
"""Build, verify and package the standalone native runtime in the shipping profile.

This currently runs on native macOS, GNU/Linux and MSVC Windows hosts. Other
targets may use Cargo's portable shipping profile, but symbol packaging/device
execution must be implemented before this command can certify their artifacts.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import zipfile

from shipping_symbols import backend, package, require, run

ROOT = Path(__file__).resolve().parents[1]


def codegen(command):
    """Read effective codegen values from Cargo's real rustc invocation."""
    return {key: value.strip('"\'') for key, value in re.findall(r'(?:^|\s)-C\s*([a-z-]+)=([^\s`]+)', command)}


def verify_commands(stderr, target, split):
    result = {}
    for crate in ('incant_runtime', 'run_scene'):
        commands = [line for line in stderr.splitlines()
                    if 'Running `' in line and re.search(r'--crate-name ' + crate + r'\b', line)
                    and f'--target {target}' in line]
        require(len(commands) == 1, f'Expected a fresh target rustc invocation for {crate}')
        flags = codegen(commands[0])
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
        if crate == 'run_scene':
            expected['lto'] = 'fat'
        require(all(flags.get(key) == value for key, value in expected.items()), f'Incorrect shipping codegen for {crate}: {flags}')
        require(flags.get('target-cpu') != 'native', 'Shipping artifacts must not depend on the build host CPU')
        require('--test' not in commands[0], 'Test harnesses do not verify shipping panic behavior')
        result[crate] = {key: flags[key] for key in expected}
    return result


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def files(directory):
    return {str(path.relative_to(directory)).replace(os.sep, '/'): {
        'sha256': digest(path), 'bytes': path.stat().st_size,
    } for path in sorted(directory.rglob('*')) if path.is_file()}


def archive(directory, destination):
    with zipfile.ZipFile(destination, 'w', compression=zipfile.ZIP_DEFLATED) as bundle:
        for path in sorted(directory.rglob('*')):
            if path.is_file():
                bundle.write(path, path.relative_to(directory))


def verify_abort(player, output):
    info = run([player, '--build-info']).strip()
    require(info == 'panic=abort debug_assertions=false', f'Wrong executable configuration: {info}')
    marker = output / 'unwind-marker'
    require(not marker.exists(), 'Panic marker already exists')
    options = {}
    if os.name == 'posix':
        def disable_core():
            import resource
            resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        options['preexec_fn'] = disable_core
    else:
        # Suppress OS crash dialogs in unattended CI. This changes no credentials
        # or distribution signing and is inherited only by this build process.
        import ctypes
        ctypes.windll.kernel32.SetErrorMode(0x0001 | 0x0002)
    probe = subprocess.run([str(player), '--panic-probe', str(marker)], cwd=output,
                           capture_output=True, text=True, timeout=30, **options)
    require(probe.returncode != 0 and 'incant shipping panic probe' in probe.stderr,
            'The deliberate runtime panic did not execute')
    require(not marker.exists(), 'Runtime panic unwound through a Drop guard')
    if os.name == 'posix':
        require(probe.returncode == -signal.SIGABRT, f'Runtime did not terminate with SIGABRT: {probe.returncode}')
    return {'build_info': info, 'panic_exit_code': probe.returncode, 'drop_guard_ran': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='New directory; contains separate player/symbol archives and evidence')
    parser.add_argument('--target', help='Native desktop host triple (cross-device packaging is not implemented)')
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--headless-binary', type=Path, default=ROOT / f'target/release/incant_headless{suffix}')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / f'target/release/examples/cook_scene{suffix}')
    args = parser.parse_args()
    rustc = Path(run(['rustup', 'which', 'rustc']).strip()) if shutil.which('rustup') else Path(shutil.which('rustc') or '')
    require(rustc.is_file(), 'Install the pinned Rust toolchain before building')
    compiler = run([rustc, '-vV'])
    host = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    target = args.target or host
    kind = backend(target)
    require(target == host, 'Shipping verification requires native execution; cross-device packaging is not implemented')
    split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm_bin = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / host / 'bin'
    require((llvm_bin / f'llvm-readobj{suffix}').is_file(), 'Install the pinned llvm-tools component')
    for binary in (args.headless_binary, args.cook_binary):
        require(binary.is_file(), f'Missing authoring-side probe tool: {binary}; build incant_headless and the cook_scene example with --release first')
    require(not os.environ.get('RUSTFLAGS') and not os.environ.get('CARGO_ENCODED_RUSTFLAGS'),
            'Unset RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS; this packager verifies the declared portable shipping profile')
    require(not os.environ.get('RUSTC_WRAPPER') and not os.environ.get('RUSTC_WORKSPACE_WRAPPER'),
            'Unset compiler wrappers for independently verified shipping builds')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    # Fresh build storage makes the recorded rustc flags and symbol files belong
    # to this exact package. It is never included in either distribution archive.
    (ROOT / 'target').mkdir(exist_ok=True)
    environment = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
    cargo = [str(ROOT / 'tools/cargo')] if os.name != 'nt' else [str(rustc.with_name('cargo.exe'))]
    with tempfile.TemporaryDirectory(prefix='shipping-build-', dir=ROOT / 'target') as storage:
        command = [*cargo, 'build', '-j2', '-p', 'incant_runtime', '--example', 'run_scene',
                   '--profile', 'shipping', '--locked', '--target', target, '--target-dir', storage,
                   '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
        print(f'Building standalone runtime for {target} with shipping profile...', flush=True)
        build = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
        if build.returncode:
            print(build.stderr[-16000:], file=sys.stderr)
            raise ValueError(f'Shipping Cargo build failed with exit code {build.returncode}')
        flags = verify_commands(build.stderr, target, split)
        artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
        executables = [artifact for artifact in artifacts if artifact.get('reason') == 'compiler-artifact'
                       and artifact['target']['name'] == 'run_scene' and artifact.get('executable')]
        require(len(executables) == 1 and not executables[0]['profile']['test'], 'Missing standalone runtime build artifact')
        executable = Path(executables[0]['executable'])
        require(executable.is_file(), 'Cargo did not produce the runtime executable')
        player, identity = package(executable, out, kind, llvm_bin)
    print('Checking the packaged executable, abort behavior and source-free cooked scene...', flush=True)
    abort = verify_abort(player, out)
    subprocess.run([sys.executable, str(ROOT / 'tools/probes/cooked-runtime.py'), str(out / 'source-free'),
                    '--headless-binary', str(args.headless_binary.resolve()), '--cook-binary', str(args.cook_binary.resolve()),
                    '--runtime-binary', str(player)], check=True, cwd=ROOT, capture_output=True, text=True)
    source_free = json.loads((out / 'source-free/result.json').read_text())
    require(source_free['passed'] and source_free['binaries'][player.name] == digest(player), 'Source-free probe did not verify the packaged player')
    sources = [ROOT / name for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
               'crates/incant_runtime/Cargo.toml', 'crates/incant_types/Cargo.toml', 'tools/cargo',
               'tools/shipping.py', 'tools/shipping_symbols.py', 'tools/probes/cooked-runtime.py')]
    sources += sorted((ROOT / 'crates/incant_runtime').rglob('*.rs'))
    sources += sorted((ROOT / 'crates/incant_types').rglob('*.rs'))
    manifest = {
        'format_version': 1, 'target': target, 'profile': 'shipping', 'compiler': compiler,
        'host': platform.platform(), 'source_commit': run(['git', '-C', ROOT, 'rev-parse', 'HEAD']).strip(),
        'source_sha256': {str(path.relative_to(ROOT)).replace(os.sep, '/'): digest(path) for path in sources},
        'codegen': flags, 'symbols': identity, 'player_files': files(out / 'player'),
        'symbol_files': files(out / 'symbols'), 'executable_checks': abort,
        'source_free_checks': source_free,
        'scope': 'Native Transform/Velocity runner; no phone, PGO, export-pipeline or phase-gate certification',
    }
    manifest_text = json.dumps(manifest, indent=2) + '\n'
    (out / 'manifest.json').write_text(manifest_text)
    # The identical manifest in each archive ties the independently stored files
    # to their executable identity and hashes without bundling symbols in player.zip.
    for group in ('player', 'symbols'):
        (out / group / 'manifest.json').write_text(manifest_text)
        archive(out / group, out / f'{group}.zip')
    print(json.dumps({'passed': True, 'target': target, 'symbols': identity, 'manifest': str(out / 'manifest.json')}))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f'Shipping verification failed: {error}', file=sys.stderr)
        sys.exit(1)
