#!/usr/bin/env python3
"""Build and run a native development Play process matching the project physics.

Usage: python3 tools/play_native.py PROJECT [--target-dir ENGINE_CACHE]
       [--evidence NEW_DIRECTORY] -- [native-play options]

Only fixed engine Cargo features are selected. Project files never choose build
flags, tools or executables. This developer launcher uses the existing native
Play host and its startup-only cook; it is not an editor worker or shipped player.
"""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

import native_process
import physics_export
from native_package import read, require, sha256, write_json
from native_play import build_lock
from physics_features import compiler_messages, validate_artifacts

ROOT = Path(__file__).resolve().parents[1]
NAME = 'incant_headless'


def executable(messages, target_dir, target):
    candidates = [m for m in messages if m.get('reason') == 'compiler-artifact'
                  and m.get('target', {}).get('name') == NAME
                  and m['target'].get('kind') == ['bin'] and m.get('executable')]
    require(len(candidates) == 1, 'Missing unique native Play executable')
    path = Path(candidates[0]['executable']).resolve(strict=True)
    require(path.is_relative_to(Path(target_dir).resolve() / target / 'ci'),
            'Native Play executable is outside selected target output')
    return path


@contextmanager
def prepare(project, target_dir, evidence=None):
    """Yield one private fixed-mode binary while its trusted caller owns it."""
    project = Path(project).resolve(strict=True)
    payload = read(project, 64 * 1024**2)
    mode = physics_export.project_mode(payload)
    project_hash = sha256(payload)
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(('INCANT_NATIVE_', 'INCANT_MOBILE_', 'INCANT_WASM_', 'INCANT_AUTHORED_'))}
    target_dir = Path(target_dir).resolve()
    require(not target_dir.is_relative_to(project.parent), 'Play build storage must be outside the project')
    # This is engine configuration, never a project-supplied tool path.
    selected = native_process.run(['rustup', 'which', 'rustc'], cwd=ROOT, env=env, timeout=30)
    require(selected.returncode == 0, 'Cannot select the repository compiler')
    rustc = Path(selected.stdout.strip()).resolve(strict=True)
    env['PATH'] = str(rustc.parent) + os.pathsep + env.get('PATH', '')
    cargo = ROOT / 'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe')
    compiler = native_process.run([str(rustc), '-vV'], cwd=ROOT, env=env, timeout=30)
    require(compiler.returncode == 0, 'Cannot identify the native compiler')
    hosts = [line.removeprefix('host: ') for line in compiler.stdout.splitlines() if line.startswith('host: ')]
    require(len(hosts) == 1, 'Missing compiler host triple')
    target = hosts[0]
    features = physics_export.cargo_features(mode, [])
    command = [str(cargo), 'build', '--locked', '--profile', 'ci', '-j2', '-p', NAME,
               '--bin', NAME, '--no-default-features', '--target', target,
               '--target-dir', str(target_dir), '--message-format=json']
    if features:
        command += ['--features', ','.join(features)]
    if evidence is not None:
        evidence = Path(evidence).resolve()
        evidence.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='incant-native-play-') as temporary:
        private = Path(temporary).resolve()
        receipt = {'physics_mode': mode, 'target': target, 'profile': 'ci',
                   'project_sha256': project_hash, 'command': command, 'passed': False}
        try:
            # Cargo's own lock ends before artifact copying. Keep both together so
            # concurrent mode selections cannot replace the executable being copied.
            with build_lock(target_dir.parent / (target_dir.name + '.incant-play.lock')):
                with (private / 'cargo.jsonl').open('w') as stdout, (private / 'cargo.stderr').open('w') as stderr:
                    result = native_process.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr, timeout=3600)
                receipt['build_exit_code'] = result.returncode
                require(result.returncode == 0, 'Native Play build failed: ' + (private / 'cargo.stderr').read_text()[-4000:])
                messages = compiler_messages(private / 'cargo.jsonl')
                receipt['physics'] = validate_artifacts(messages, mode, target_dir=target_dir, target=target, profile='ci')
                built = executable(messages, target_dir, target)
                binary = private / built.name
                shutil.copy2(built, binary)
                receipt['binary_sha256'] = sha256(read(binary, 512 * 1024**2))
            require(sha256(read(project, 64 * 1024**2)) == project_hash, 'Project changed during Play build; retry')
            yield binary, env, receipt, private
        finally:
            # Managed process cleanup happens before temporary inputs disappear.
            if evidence is not None:
                for name in ('cargo.jsonl', 'cargo.stderr', 'play.stdout', 'play.stderr', 'worker.stderr'):
                    if (private / name).exists():
                        shutil.copy2(private / name, evidence / name)
                write_json(evidence / 'selection.json', receipt)


def play(project, target_dir, options, evidence=None):
    project = Path(project).resolve(strict=True)
    with prepare(project, target_dir, evidence) as (binary, env, receipt, private):
        receipt['play_command'] = [str(binary), 'native-play', str(project), *options]
        with (private / 'play.stdout').open('w') as stdout, (private / 'play.stderr').open('w') as stderr:
            result = native_process.run(receipt['play_command'], cwd=ROOT, env=env,
                                        stdout=stdout, stderr=stderr, timeout=3600)
        receipt['play_exit_code'] = result.returncode
        require(result.returncode == 0, 'Native Play failed: ' + (private / 'play.stderr').read_text()[-4000:])
        report = json.loads((private / 'play.stdout').read_text())
        require(report['project_sha256'] == receipt['project_sha256'], 'Play loaded a different project snapshot')
        receipt['passed'] = report['passed']
        return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('--target-dir', type=Path,
                        default=Path.home() / '.cache/incant/native-play' / sha256(str(ROOT).encode())[:16])
    parser.add_argument('--evidence', type=Path)
    args, options = parser.parse_known_args()
    if options[:1] == ['--']:
        options = options[1:]
    try:
        report = play(args.project, args.target_dir, options, args.evidence)
        print(json.dumps(report))
        return 0 if report['passed'] else 1
    except (ValueError, OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f'Native Play selection failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
