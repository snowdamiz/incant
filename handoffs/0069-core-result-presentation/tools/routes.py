#!/usr/bin/env python3
"""Write the accepted numerical routes used for 0069 runs (CPU only).

Usage: python3 -B routes.py OUT [--accepted DIR]

Recordings come only from samples/core/binding_oracle.py (cases, restart_cases,
restarted); nothing is planned or edited here. They are written in the exact
format of verify_bindings.py, and, when --accepted names Astra's retained
shipping proof, each file must equal that proof's `<name>-input.json` byte for byte.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'samples/core'))
import binding_oracle as oracle  # noqa: E402


def text(value):
    # verify_bindings.write: indent 2, UTF-8, trailing newline.
    return json.dumps(value, indent=2, ensure_ascii=False) + '\n'


def routes():
    recordings, restarts = oracle.restart_cases()
    out = {name: recordings[name] for name in ('idle', 'lose-short', 'locked', 'win')}
    for name, first, second, pad, held in restarts:
        out[name] = oracle.restarted(recordings[first], recordings[second], pad=pad, held=held)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--accepted', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    summary = {}
    for name, value in routes().items():
        data = text(value).encode('utf-8')
        (args.output / f'{name}.json').write_bytes(data)
        row = {'ticks': value['ticks'], 'sha256': hashlib.sha256(data).hexdigest()}
        if args.accepted:
            accepted = args.accepted / f'{name}-input.json'
            row['accepted'] = str(accepted)
            row['equals_accepted'] = accepted.is_file() and accepted.read_bytes() == data
        summary[name] = row
    (args.output / 'routes.json').write_text(json.dumps(summary, indent=1) + '\n', encoding='utf-8')
    print(json.dumps(summary, indent=1))
    if args.accepted and not all(r['equals_accepted'] for r in summary.values()):
        sys.exit(1)


if __name__ == '__main__':
    main()
