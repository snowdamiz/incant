#!/usr/bin/env python3
"""Report credential dependency impact. Never skip a command or execute Cargo.

Only a lock-only change can receive an unaffected report. The graph deliberately
includes every lock edge, including optional/target/build/transitive dev edges;
it is an overestimate of any one host's package-test graph. Unchanged manifests
and identical reachable package records are prerequisites, not a skip policy.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib


class UnknownGraph(ValueError):
    pass


def locked_graph(raw, root='incant_agent'):
    lock = tomllib.loads(raw)
    if set(lock) != {'version', 'package'} or lock['version'] not in (3, 4):
        raise UnknownGraph('unsupported_lock_format')
    packages = lock['package']
    if not isinstance(packages, list):
        raise UnknownGraph('invalid_packages')
    records, references = {}, {}
    for package in packages:
        if not isinstance(package, dict) or set(package) - {
                'name', 'version', 'source', 'checksum', 'dependencies'}:
            raise UnknownGraph('unsupported_package_record')
        if any(not isinstance(package.get(key), str) or not package[key]
               for key in ('name', 'version')):
            raise UnknownGraph('invalid_package_identity')
        if any(not isinstance(package[key], str) or not package[key]
               for key in ('source', 'checksum') if key in package):
            raise UnknownGraph('invalid_package_source')
        if not re.fullmatch(r'[A-Za-z0-9_-]+', package['name']) or not re.fullmatch(
                r'\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)*', package['version']):
            raise UnknownGraph('unsupported_package_identity')
        if 'checksum' in package and not re.fullmatch(r'[0-9a-f]{64}', package['checksum']):
            raise UnknownGraph('invalid_checksum')
        dependencies = package.get('dependencies', [])
        if not isinstance(dependencies, list) or not all(
                isinstance(value, str) for value in dependencies):
            raise UnknownGraph('invalid_dependencies')
        identity = (package['name'], package['version'], package.get('source', ''))
        if identity in records:
            raise UnknownGraph('duplicate_package_identity')
        records[identity] = package
        names = [identity[0], ' '.join(identity[:2])]
        if identity[2]:
            names.append(f'{identity[0]} {identity[1]} ({identity[2]})')
        for name in names:
            references.setdefault(name, []).append(identity)
    adjacency = {}
    for identity, package in records.items():
        edges = []
        for dependency in package.get('dependencies', []):
            matches = references.get(dependency, [])
            if len(matches) != 1:
                raise UnknownGraph('missing_or_ambiguous_dependency')
            edges.append(matches[0])
        adjacency[identity] = sorted(edges)
    roots = [key for key in records if key[0] == root and not key[2]]
    if len(roots) != 1:
        raise UnknownGraph('missing_or_ambiguous_root')
    pending, reachable = roots[:], {}
    while pending:
        identity = pending.pop()
        if identity in reachable:
            continue
        package, edges = records[identity], adjacency[identity]
        reachable[identity] = {
            'identity': identity, 'checksum': package.get('checksum'),
            'dependencies': sorted(edges),
        }
        pending.extend(edges)
    canonical = [reachable[key] for key in sorted(reachable)]
    digest = hashlib.sha256(json.dumps(canonical, sort_keys=True).encode()).hexdigest()
    return {
        'sha256': digest, 'packages': len(canonical),
        'local_packages': [key[0] for key in sorted(reachable) if not key[2]],
    }


def classify(paths, before=None, after=None):
    report = {'report_only': True, 'decision': 'run_all', 'classification': 'full',
              'reason': 'non_lock_inputs', 'changed_paths': sorted(set(paths))}
    if report['changed_paths'] != ['Cargo.lock']:
        return report
    try:
        old, new = locked_graph(before), locked_graph(after)
    except (UnknownGraph, tomllib.TOMLDecodeError, TypeError, KeyError) as error:
        report['reason'] = str(error) if isinstance(error, UnknownGraph) else 'unreadable_lock'
        return report
    report.update({'base_graph': old, 'head_graph': new})
    report['reason'] = 'reachable_lock_graph_changed'
    if old == new:
        report.update(classification='lock_only_unaffected', reason='reachable_lock_graph_identical')
    return report


def git(repo, *arguments):
    return subprocess.check_output(
        ['git', '-C', str(repo), *arguments], stderr=subprocess.DEVNULL)


def report(repo, base, head, event):
    result = classify([])
    result.update(base=base, head=head, event=event)
    if event != 'pull_request':
        result['reason'] = 'non_pull_request_event'
        return result
    if not all(re.fullmatch(r'[0-9a-f]{40}', value or '') for value in (base, head)):
        result['reason'] = 'missing_or_invalid_commit'
        return result
    try:
        for revision in (base, head):
            if git(repo, 'rev-parse', '--verify', revision + '^{commit}').decode().strip() != revision:
                raise UnknownGraph('commit_mismatch')
        git(repo, 'merge-base', '--is-ancestor', base, head)
        # Disabling rename detection reports both deletion and addition. NUL
        # delimiters preserve spaces/newlines; no paginated GitHub file list.
        raw = git(repo, 'diff', '--no-ext-diff', '--no-textconv', '--no-renames',
                  '--name-only', '-z', base, head, '--')
        if raw and not raw.endswith(b'\0'):
            raise UnknownGraph('incomplete_diff')
        paths = [path.decode('utf-8') for path in raw.split(b'\0') if path]
        before = after = None
        if paths == ['Cargo.lock']:
            before = git(repo, 'show', base + ':Cargo.lock').decode('utf-8')
            after = git(repo, 'show', head + ':Cargo.lock').decode('utf-8')
        result.update(classify(paths, before, after))
    except (subprocess.CalledProcessError, OSError, UnicodeError, UnknownGraph):
        result['reason'] = 'commit_or_diff_unavailable'
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', default=os.environ.get('INCANT_IMPACT_BASE', ''))
    parser.add_argument('--head', default=os.environ.get('INCANT_IMPACT_HEAD', ''))
    parser.add_argument('--event', default=os.environ.get('GITHUB_EVENT_NAME', ''))
    parser.add_argument('--repo', type=Path, default=Path.cwd())
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = report(args.repo, args.base, args.head, args.event)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(f"Credential impact (report only): {result['classification']}; "
          f"{result['reason']}. All commands still run.")


if __name__ == '__main__':
    main()
