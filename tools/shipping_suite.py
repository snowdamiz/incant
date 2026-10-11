#!/usr/bin/env python3
"""Run the existing runtime and script/physics proofs in one fresh macOS session.

This is an opt-in local experiment until matched coverage and timing are reviewed.
Standalone shipping.py and cooked-script.py commands remain supported.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import shutil
import time

import shipping
from shipping_session import FreshSession
from shipping_symbols import run

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--headless-binary', type=Path, default=ROOT / 'target/release/incant_headless')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / 'target/release/examples/cook_scene')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV'])
    target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    session = FreshSession(ROOT, out / 'fresh-build', rustc, target, 'packed')
    spec = importlib.util.spec_from_file_location('cooked_script_probe', ROOT / 'tools/probes/cooked-script.py')
    script = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(script)
    common = {'headless_binary': args.headless_binary, 'cook_binary': args.cook_binary}
    phases = [shipping.phases(argparse.Namespace(output=out / 'runtime', target=target, **common), session),
              script.phases(argparse.Namespace(output=out / 'script', physics=True, **common), session)]
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
                      probes=['shipping.py', 'cooked-script.py --physics'])
        (out / 'session.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps({'passed': True, 'seconds': result['elapsed_seconds'], 'evidence': str(out / 'session.json')}))
    finally:
        for phase in phases:
            phase.close()
        if session.path.exists():
            shutil.rmtree(session.path)


if __name__ == '__main__':
    main()
