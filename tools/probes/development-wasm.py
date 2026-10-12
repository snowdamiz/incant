#!/usr/bin/env python3
"""Compile only the trusted Rust fixture; run a real source-deleted Wasm module.

Development JIT conformance, not an untrusted compiler service, wasm2c export,
shipping benchmark or device gate. No project build scripts/proc macros run.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import rust_guest
from shipping import cargo_output


def run(command, **kwargs):
    return subprocess.run([str(x) for x in command], check=True, capture_output=True,
                          text=True, encoding='utf-8', **kwargs).stdout


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--rustup', default='rustup')
    parser.add_argument('--profile', choices=('dev', 'ci'), default='dev', help='Local conformance profile')
    parser.add_argument('--static-conformance', action='store_true', help='Reuse a prepared combined development/static test graph')
    args = parser.parse_args()
    feature = 'static-conformance' if args.static_conformance else 'wasmtime-host'
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run([args.rustup, 'which', '--toolchain', '1.99.0', 'rustc']).strip())
    env = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
    cargo = ROOT / 'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe')
    # Audit shipping roots without selecting the optional development compiler.
    # The real development example below explicitly enables its host feature.
    metadata = json.loads(run([cargo, 'metadata', '--locked', '--format-version=1'],
                              cwd=ROOT, env=env))
    names = {p['id']: p['name'] for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    shipping = ['incant_runtime', 'incant_script', 'incant_game_runtime',
                'incant_script_player', 'incant_game_player']
    for root in shipping:
        pending = [ident for ident, name in names.items() if name == root]
        if len(pending) != 1:
            raise SystemExit(f'Missing/ambiguous shipping root {root}')
        seen = set()
        while pending:
            ident = pending.pop()
            if ident in seen:
                continue
            seen.add(ident)
            if names[ident].startswith(('wasmtime', 'cranelift')):
                raise SystemExit(f'Development compiler leaked into {root}: {names[ident]}')
            pending.extend(dep['pkg'] for dep in nodes[ident]['deps']
                           if any(kind['kind'] is None for kind in dep['dep_kinds']))
    built = cargo_output([str(cargo), 'build', '--profile', args.profile, '-j2', '-p', 'incant_wasm_dev', '--features',
                          feature, '--example', 'development_module', '--locked',
                          '--message-format=json'], cwd=ROOT, env=env)
    if built.returncode:
        raise SystemExit(built.stderr)
    binary = next(Path(record['executable']) for record in map(json.loads, built.stdout.splitlines())
                  if record.get('reason') == 'compiler-artifact' and record.get('executable')
                  and record['target']['name'] == 'development_module')
    runner = out / binary.name; shutil.copy2(binary, runner)
    source = out / 'trusted_fixture.rs'
    shutil.copy2(ROOT / 'crates/incant_wasm_dev/fixtures/bulk.rs', source)
    source_hash = sha(source); module = out / 'bulk.wasm'
    rust_guest.compile(rustc,source,module,env=env)
    source.unlink()
    expected = '1 0.25\n2 0.125\n3 0.25\n4 0.125\n'
    outputs = [run([runner, module], cwd=out, env=env) for _ in range(3)]
    if outputs != [expected] * 3:
        raise SystemExit('Native module state differs from exact dyadic expectation')
    report = {'kind': 'development-jit-conformance', 'rustc': run([rustc, '-vV']),
              'shipping_roots_without_wasmtime_or_cranelift': shipping,
              'rust_sdk': rust_guest.provenance(), 'fixture_sha256': source_hash, 'fixture_deleted': not source.exists(),
              'wasm_sha256': sha(module), 'runner_sha256': sha(runner), 'processes': 3,
              'stdout': outputs, 'limitations': ['host build storage retained',
              'development JIT, not shipping wasm2c', 'trusted fixture compile, no untrusted build service',
              'typed transfer ABI copies columns', 'no device/performance gate']}
    (out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
