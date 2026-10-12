#!/usr/bin/env python3
"""NDK build/link check for the real mobile ABI; never bypasses cook admission.

This driver is ready for the existing candidate-only Android job once the target
bytecode contract is approved. A compiler refusal is a failed build, not a pass.
No Android execution or device claim follows from a linked ELF binary.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from native_process import run  # noqa: E402


def digest(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as file:
        for part in iter(lambda: file.read(1024 * 1024), b''):
            value.update(part)
    return value.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ndk', type=Path, required=True)
    parser.add_argument('--target-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        parser.error('This candidate CI adapter is scoped to the pinned Linux x86_64 NDK host')
    ndk = args.ndk.resolve(strict=True)
    clang = ndk / 'toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang'
    if not clang.is_file():
        parser.error('NDK arm64 API-24 clang is missing')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    target = args.target_dir.resolve()
    env = dict(os.environ, ANDROID_NDK_HOME=str(ndk), CARGO_TARGET_DIR=str(target),
               LIBCLANG_PATH=str(ndk / 'toolchains/llvm/prebuilt/linux-x86_64/lib'))
    commands = []
    def command(argv, timeout=3600):
        name = f'command-{len(commands):03d}'
        out, err = output / (name + '.stdout'), output / (name + '.stderr')
        start = time.monotonic()
        with out.open('wb') as stdout, err.open('wb') as stderr:
            result = run([str(v) for v in argv], cwd=ROOT, env=env, timeout=timeout,
                         stdout=stdout, stderr=stderr, text=False)
        commands.append({'argv': [str(v) for v in argv], 'exit_code': result.returncode,
                         'seconds': time.monotonic() - start, 'stdout': str(out), 'stderr': str(err),
                         'stdout_sha256': digest(out), 'stderr_sha256': digest(err)})
        if result.returncode:
            raise RuntimeError(f'Android build/link failed; retained {err}')
    passed = False
    report = {'target': 'aarch64-linux-android', 'profile': 'shipping',
              'features': ['mobile-ffi'], 'ndk': str(ndk), 'clang_sha256': digest(clang),
              'claims': 'archive and real C ABI link only; no runtime or device acceptance'}
    try:
        header = ROOT / 'crates/incant_game_player/include/incant_game.h'
        report['header_sha256'] = digest(header)
        report['lock_sha256'] = digest(ROOT / 'Cargo.lock')
        command([ROOT / 'tools/cargo', 'ndk', '-t', 'arm64-v8a', '--platform', '24',
                 'rustc', '--locked', '--profile', 'shipping', '-j2', '-p', 'incant_game_player',
                 '--lib', '--features', 'mobile-ffi', '--crate-type', 'staticlib'])
        archive = target / 'aarch64-linux-android/shipping/libincant_game_player.a'
        copied = output / archive.name
        shutil.copyfile(archive, copied)
        report['archive'] = {'path': str(copied), 'sha256': digest(copied), 'bytes': copied.stat().st_size}
        caller = output / 'incant-mobile-caller'
        command([clang, '-std=c11', '-Wall', '-Wextra', '-Werror', '-I', header.parent,
                 HERE / 'caller.c', copied, '-ldl', '-llog', '-lm', '-o', caller])
        command([clang.parent / 'llvm-readelf', '--file-header', '--symbols', caller])
        report['caller'] = {'path': str(caller), 'sha256': digest(caller), 'bytes': caller.stat().st_size}
        assert digest(header) == report['header_sha256']
        assert digest(ROOT / 'Cargo.lock') == report['lock_sha256']
        passed = True
    finally:
        report.update(passed=passed, commands=commands)
        (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
