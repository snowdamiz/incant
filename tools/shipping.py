#!/usr/bin/env python3
"""Build, verify and package the standalone native runtime in the shipping profile.

This currently runs on native macOS, GNU/Linux and MSVC Windows hosts. Other
targets may use Cargo's portable shipping profile, but symbol packaging/device
execution must be implemented before this command can certify their artifacts.
"""
import argparse
from contextlib import redirect_stdout
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import zipfile

from shipping_symbols import backend, package, require, run

ROOT = Path(__file__).resolve().parents[1]


def cargo_output(command, *, cwd, env):
    """Capture Cargo machine output without depending on the host locale."""
    return subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, encoding='utf-8')


def codegen(command):
    """Read effective codegen values from Cargo's real rustc invocation."""
    return {key: value.strip('"\'') for key, value in re.findall(r'(?:^|\s)-C\s*([a-z-]+)=([^\s`]+)', command)}


def player_units(stderr, target, split, features):
    """Audit both runtime library and final executable in desktop probe builds."""
    units = {}
    for line in stderr.splitlines():
        if 'Running `' not in line or not re.search(r'--crate-name incant_game_player\b', line):
            continue
        tokens = shlex.split(line.split('Running `', 1)[1].rstrip().removesuffix('`'))
        if '--target' not in tokens:
            continue
        def one(flag):
            values = [tokens[i + 1] for i, token in enumerate(tokens[:-1]) if token == flag]
            require(len(values) == 1, f'Missing unique player {flag}')
            return values[0]
        require(one('--target') == target, 'Unexpected player target')
        kind = one('--crate-type')
        require(kind in ('lib', 'bin') and kind not in units, 'Missing/duplicate player crate kind')
        actual_features = [tokens[i + 1][9:-1] for i, token in enumerate(tokens[:-1])
                           if token == '--cfg' and re.fullmatch(r'feature="[^"]+"', tokens[i + 1])]
        require(len(actual_features) == len(set(actual_features)) and set(actual_features) == set(features),
                f'Unexpected player {kind} features: {actual_features}')
        require('compiler' not in actual_features and '--test' not in tokens, 'Compiler/test player unit')
        values = codegen(line)
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort',
                    'debuginfo': '2', 'split-debuginfo': split}
        if kind == 'bin':
            expected['lto'] = 'fat'
        require(all(values.get(k) == v for k, v in expected.items())
                and values.get('target-cpu') != 'native', f'Nonshipping player {kind}: {values}')
        units[kind] = expected
    require(set(units) == {'lib', 'bin'}, 'Missing player library or executable target compile')
    return {'incant_game_player_library': units['lib'], 'incant_game_player': units['bin']}


def verify_commands(stderr, target, split):
    result = {}
    for crate in ('incant_runtime', 'run_scene'):
        commands = [line for line in stderr.splitlines()
                    if 'Running `' in line and re.search(r'--crate-name ' + crate + r'\b', line)
                    and f'--target {target}' in line]
        require(len(commands) == 1, f'Expected a fresh target rustc invocation for {crate}')
        flags = codegen(commands[0])
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
        if target.endswith('-windows-msvc'):
            expected['target-feature'] = '+crt-static'
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


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='New directory; contains separate player/symbol archives and evidence')
    parser.add_argument('--target', help='Native desktop host triple (cross-device packaging is not implemented)')
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--headless-binary', type=Path, default=ROOT / f'target/release/incant_headless{suffix}')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / f'target/release/examples/cook_scene{suffix}')
    return parser.parse_args()


def phases(args, session=None):
    suffix = '.exe' if os.name == 'nt' else ''
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
    manager = session.storage() if session else tempfile.TemporaryDirectory(prefix='shipping-build-', dir=ROOT / 'target')
    with manager as storage:
        command = [*cargo, 'build', '-j2', '-p', 'incant_runtime', '--example', 'run_scene',
                   '--profile', 'shipping', '--locked', '--target', target, '--target-dir', storage,
                   '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
        print(f'Building standalone runtime for {target} with shipping profile...', flush=True)
        if session:
            build, audited_commands = session.build(command, cwd=ROOT, env=environment)
        else:
            build = cargo_output(command, cwd=ROOT, env=environment)
            audited_commands = build.stderr
        (out / 'build.log').write_text(build.stderr, encoding='utf-8')
        (out / 'cargo-artifacts.jsonl').write_text(build.stdout, encoding='utf-8')
        if build.returncode:
            print(build.stderr[-16000:], file=sys.stderr)
            raise ValueError(f'Shipping Cargo build failed with exit code {build.returncode}')
        flags = verify_commands(audited_commands, target, split)
        artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
        executables = [artifact for artifact in artifacts if artifact.get('reason') == 'compiler-artifact'
                       and artifact['target']['name'] == 'run_scene' and artifact.get('executable')]
        require(len(executables) == 1 and not executables[0]['profile']['test'], 'Missing standalone runtime build artifact')
        executable = Path(executables[0]['executable'])
        require(executable.is_file(), 'Cargo did not produce the runtime executable')
        player, identity = package(executable, out, kind, llvm_bin)
    spec = importlib.util.spec_from_file_location('cooked_runtime_probe', ROOT / 'tools/probes/cooked-runtime.py')
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    cooked = probe.phases(argparse.Namespace(output=out / 'source-free',
        headless_binary=args.headless_binary, cook_binary=args.cook_binary, runtime_binary=player), session)
    next(cooked)  # Cook and remove authoring inputs before any suite player starts.
    if session:
        session.packaged_player(player)
    yield
    if session:
        session.assert_ready()
    require(not Path(storage).exists(), 'Shipping build storage remains before player launch')
    print('Checking the packaged executable, abort behavior and source-free cooked scene...', flush=True)
    abort = verify_abort(player, out)
    with redirect_stdout(io.StringIO()):
        for _ in cooked:
            raise ValueError('Unexpected extra cooked-runtime preparation phase')
    source_free = json.loads((out / 'source-free/result.json').read_text())
    require(source_free['passed'] and source_free['binaries'][player.name] == digest(player), 'Source-free probe did not verify the packaged player')
    sources = [ROOT / name for name in ('Cargo.toml', 'Cargo.lock', '.cargo/config.toml', 'rust-toolchain.toml',
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


def main():
    for _ in phases(arguments()):
        pass


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f'Shipping verification failed: {error}', file=sys.stderr)
        sys.exit(1)
