#!/usr/bin/env python3
"""CPU-only ordinary-load proof with actual authored TS and embedded native code.

CI-profile correctness only. Optional warm artifacts are copied into private build
storage, which is deleted with all copied engine/fixture/compiler inputs before
launching the copied executable. No shipping/codegen, device or whole-game claim.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

NAME = 'incant_native_session_checkpoint_probe'
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT/'tools'))
from native_process import run


def require(value, message):
    if not value:
        raise ValueError(message)


def execute(command, *, cwd=None, env=None, timeout=120):
    result = run(list(map(str, command)), cwd=cwd, env=env, timeout=timeout)
    require(result.returncode == 0, result.stderr[-6000:])
    return result.stdout


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--warm-target', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    spec = importlib.util.spec_from_file_location('embedding', ROOT/'tools/probes/project-script-embedding.py')
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    helper.NAME = NAME
    with tempfile.TemporaryDirectory(prefix='checkpoint-build-', dir=out) as temporary:
        storage = Path(temporary)
        engine = storage/'engine'
        hashes = helper.snapshot(ROOT, engine)
        crate = engine/'crates'/NAME
        shutil.copytree(ROOT/'tools/probes/native-session-checkpoint', crate)
        (engine/'node_modules').symlink_to((ROOT/'node_modules').resolve(), target_is_directory=True)
        target = storage/'target'
        if args.warm_target:
            require(sys.platform == 'darwin', 'APFS warm-target clone is macOS-only')
            execute(['cp', '-cR', args.warm_target.resolve(), target], timeout=300)
        env = dict(os.environ, CARGO_TARGET_DIR=str(target))
        for key in list(env):
            if key.startswith('INCANT_NATIVE_'):
                del env[key]
        cargo = engine/'tools/cargo'
        before = tomllib.loads((engine/'Cargo.lock').read_text())['package']
        metadata = json.loads(execute([cargo, 'metadata', '--offline', '--format-version=1'], cwd=engine, env=env))
        after = tomllib.loads((engine/'Cargo.lock').read_text())['package']
        require([p for p in after if p['name'] != NAME] == before, 'Existing resolutions changed')
        graph = helper.normal_graph(metadata)
        with (out/'cargo.jsonl').open('w') as stdout, (out/'build.log').open('w') as stderr:
            result = run([str(cargo), 'build', '--offline', '--locked', '--profile', 'ci', '-j2', '-p', NAME,
                          '--message-format=json'], cwd=engine, env=env, timeout=1800, stdout=stdout, stderr=stderr)
        require(result.returncode == 0, f'Build failed; see {out / "build.log"}')
        artifacts = [json.loads(line) for line in (out/'cargo.jsonl').read_text().splitlines() if line.startswith('{')]
        scripts = [a for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'incant_script' and 'lib' in a['target']['kind']]
        require(any(not a['features'] for a in scripts), 'No compiler-disabled script runtime artifact')
        players = [Path(a['executable']) for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == NAME and a.get('executable')]
        require(len(players) == 1, 'Expected one actual probe executable')
        player = out/players[0].name
        shutil.copy2(players[0], player)
        generated = list((target/'ci/build').glob(f'{NAME}-*/out/behavior.rs'))
        require(len(generated) == 1, 'Expected one trusted generated behavior')
        generated_hash = digest(generated[0])
        (out/'compiler.json').write_bytes(generated[0].with_name('compiler.json').read_bytes())
        # Every copied source remains identical through the actual build.
        require(all(digest(engine/name) == value for name,value in hashes.items() if name != 'Cargo.lock'), 'Frozen source changed')
    require(not storage.exists(), 'Private source/build storage retained')
    env = {'PATH':str(out), 'HOME':str(out/'absent'), 'TMPDIR':str(out), 'TEMP':str(out), 'TMP':str(out)}
    env.update({k:os.environ[k] for k in ('SystemRoot','SystemDrive','WINDIR') if k in os.environ})
    observations = []
    for mode, source, output in [('baseline','-',out/'baseline.save'), ('save','-',out/'mid.save'),
                                  ('resume',out/'mid.save',out/'restored.save'), ('resume',out/'mid.save',out/'repeated.save')]:
        observations.append(execute([player,mode,source,output], cwd=out, env=env, timeout=30).strip())
    require((out/'baseline.save').read_bytes() == (out/'restored.save').read_bytes() == (out/'repeated.save').read_bytes(), 'Resumed state/timer/world differ from uninterrupted result')
    bad = bytearray((out/'mid.save').read_bytes()); bad[-1] ^= 1
    (out/'corrupt.save').write_bytes(bad)
    refused = run([str(player),'resume',str(out/'corrupt.save'),str(out/'must-not-exist')],cwd=out,env=env,timeout=30)
    require(refused.returncode != 0 and not (out/'must-not-exist').exists(), 'Corruption was published')
    report = {'passed':True,'successful_processes':4,'refusal_processes':1,
              'observations':observations,'world_state_timer_bytes_equal':True,
              'private_source_and_build_storage_deleted':True,'source_parser_feature_disabled':True,
              'normal_dependencies':graph,'source_hashes':hashes,'generated_rust_sha256':generated_hash,
              'player_sha256':digest(player),'final_checkpoint_sha256':digest(out/'baseline.save'),
              'warm_artifacts_cloned':bool(args.warm_target),
              'limitations':['CI profile correctness only; no shipping/performance gate',
                             'original repository and warm cache retained; disposable copied build/source inputs removed',
                             'ordinary neutral-input load, not VM heap or exact physical-input replay',
                             'physics/audio/static-module and GameSession/export load routes remain unsupported']}
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({key:report[key] for key in ('passed','successful_processes','refusal_processes','final_checkpoint_sha256')},indent=2))

if __name__ == '__main__':
    main()
