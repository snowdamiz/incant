#!/usr/bin/env python3
"""Reject authoring dependencies in named runtime packages' resolved Cargo graph.

Roots are explicit: checking incant_types alone must never report that the
not-yet-migrated shipped runtime passes. Inspect every resolved normal edge,
including target-specific edges and renamed dependencies. Dev/build-only edges
do not enter the shipped dependency graph.
"""
import argparse
from collections import deque
import json
from pathlib import Path
import subprocess
import sys

FORBIDDEN = frozenset({
    'incant_doc', 'incant_cmd', 'incant_agent', 'incant_editor',
    'loro', 'loro-internal', 'automerge',
})


def violations(metadata, roots):
    packages = {p['id']: p['name'] for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    problems = []
    for root in roots:
        matches = [ident for ident, name in packages.items() if name == root]
        if len(matches) != 1 or matches[0] not in nodes:
            raise ValueError(f'{root}: expected one resolved package, found {len(matches)}')
        pending = deque([(matches[0], [root])])
        seen = set()
        while pending:
            ident, path = pending.popleft()
            if ident in seen:
                continue
            seen.add(ident)
            if packages[ident] in FORBIDDEN:
                problems.append(path)
                continue
            for dep in nodes[ident]['deps']:
                if any(kind['kind'] is None for kind in dep['dep_kinds']):
                    pending.append((dep['pkg'], path + [packages[dep['pkg']]]))
    return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('roots', nargs='+', help='Packages whose runtime dependencies must stay lean')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    cargo = root / 'tools' / 'cargo'
    command = [str(cargo)] if sys.platform != 'win32' else ['cargo']
    try:
        result = subprocess.run(
            command + ['metadata', '--locked', '--format-version', '1'],
            cwd=root, check=True, capture_output=True, text=True,
        )
        problems = violations(json.loads(result.stdout), args.roots)
    except subprocess.CalledProcessError as error:
        print('Runtime dependency check could not resolve Cargo metadata.', file=sys.stderr)
        print(error.stderr, file=sys.stderr)
        return 1
    except (KeyError, TypeError, ValueError) as error:
        print(f'Runtime dependency check failed: {error}', file=sys.stderr)
        return 1
    if problems:
        for path in problems:
            print('Forbidden runtime dependency: ' + ' -> '.join(path), file=sys.stderr)
        return 1
    print('Authoring-free resolved dependencies: ' + ', '.join(args.roots))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
