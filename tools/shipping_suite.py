#!/usr/bin/env python3
"""Run shipping proofs in one fresh macOS session with a global deletion barrier.

This is an opt-in local experiment until matched coverage and timing are reviewed.
Standalone shipping.py and cooked-script.py commands remain supported.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import shutil
import sys
import time

import shipping
from shipping_session import FreshSession
from shipping_symbols import run

ROOT = Path(__file__).resolve().parents[1]


def load_probe(name):
    directory = ROOT / 'tools/probes'
    search_path = sys.path[:]
    try:
        # Match direct probe execution, including trusted sibling helpers.
        sys.path.insert(0, str(directory))
        spec = importlib.util.spec_from_file_location(name.replace('-', '_'), directory / f'{name}.py')
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        return probe
    finally:
        sys.path[:] = search_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--headless-binary', type=Path, default=ROOT / 'target/release/incant_headless')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / 'target/release/examples/cook_scene')
    parser.add_argument('--four-proofs', action='store_true',
                        help='Also retain the complete scene-audio and native-game GPU proofs')
    parser.add_argument('--audio-binary', type=Path, default=ROOT / 'target/release/examples/native_scene_audio')
    parser.add_argument('--fixture-binary', type=Path, default=ROOT / 'target/release/examples/no_renderer_fixtures')
    parser.add_argument('--flipbook', action='store_true',
                        help='Retain native flipbook numerical controls; requires --four-proofs')
    parser.add_argument('--flipbook-sdk', action='store_true',
                        help='Retain QuickJS and trusted wasm2c controls; requires --flipbook')
    args = parser.parse_args()
    if args.flipbook and not args.four_proofs:
        parser.error('--flipbook requires --four-proofs')
    if args.flipbook_sdk and not args.flipbook:
        parser.error('--flipbook-sdk requires --flipbook')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV'])
    target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    session = FreshSession(ROOT, out / 'fresh-build', rustc, target, 'packed')
    script = load_probe('cooked-script')
    common = {'headless_binary': args.headless_binary, 'cook_binary': args.cook_binary}
    phases = [shipping.phases(argparse.Namespace(output=out / 'runtime', target=target, **common), session),
              script.phases(argparse.Namespace(output=out / 'script', physics=True, **common), session)]
    probes = ['shipping.py', 'cooked-script.py --physics']
    if args.four_proofs:
        audio = load_probe('native-scene-audio')
        game = load_probe('native-game')
        # Match the ordinary CI order; only dependency artifacts created in this
        # invocation may be reused. Every player and its symbol package is distinct.
        phases[1:1] = [
            audio.phases(argparse.Namespace(output=out / 'audio', **common), session),
            game.phases(argparse.Namespace(output=out / 'game', gpu=True, render_input=True, flipbook=args.flipbook,
                flipbook_sdk=args.flipbook_sdk,
                audio_binary=args.audio_binary, fixture_binary=args.fixture_binary, **common), session),
        ]
        probes = ['shipping.py', 'native-scene-audio.py',
                  'native-game.py --gpu' + (' --flipbook' if args.flipbook else '')
                  + (' --flipbook-sdk' if args.flipbook_sdk else ''),
                  'cooked-script.py --physics']
    begin = time.monotonic()
    try:
        for phase in phases:
            next(phase)
        session.seal()
        for phase in phases:
            for _ in phase:
                raise ValueError('Unexpected additional shipping preparation phase')
        result = session.report()
        result.update(passed=True, elapsed_seconds=time.monotonic() - begin,
                      probes=probes)
        (out / 'session.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps({'passed': True, 'seconds': result['elapsed_seconds'], 'evidence': str(out / 'session.json')}))
    finally:
        for phase in phases:
            phase.close()
        if session.path.exists():
            shutil.rmtree(session.path)


if __name__ == '__main__':
    main()
