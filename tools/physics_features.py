#!/usr/bin/env python3
"""Audit physics mode forwarding and selected/compiled dependency features.

Graph input must come from a scoped Cargo invocation with --prefix none,
--format '{p}|{f}', -e normal, --target TRIPLE and -p PACKAGE. It is a
selected normal graph, never workspace-wide cargo metadata. Compiler evidence
must use explicit --target, including native builds, to separate host units.
"""
import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import re
import sys
import tomllib

MODES = ('deterministic', 'fast', 'fast-simd8')
SOLVERS = ('rapier2d', 'rapier3d', 'parry2d', 'parry3d', 'simba')
VERSIONS = {'rapier2d': '0.36.0', 'rapier3d': '0.36.0',
            'parry2d': '0.31.1', 'parry3d': '0.31.1', 'simba': '0.10.2'}
LEAVES = {'incant_physics_runtime', 'incant_physics_2d_runtime'}
DEFAULTS = {'incant_script': {'compiler'}, 'incant_cook': {'scene', 'script'}}
SOLVER_MODES = {name: {'enhanced-determinism', 'simd8'} for name in SOLVERS if name != 'simba'}
SOLVER_MODES['simba'] = {'libm_force'}


@dataclass(frozen=True)
class Edge:
    name: str
    package: str
    kind: str
    optional: bool
    defaults: bool
    features: tuple


def _load(root):
    root = Path(root)
    workspace = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))['workspace']
    inherited = workspace.get('dependencies', {})
    excluded = {p.resolve() for pattern in workspace.get('exclude', []) for p in root.glob(pattern)}
    paths = {p / 'Cargo.toml' for pattern in workspace['members']
             for p in root.glob(pattern) if p.resolve() not in excluded}
    packages = {}
    for path in sorted(paths):
        data = tomllib.loads(path.read_text(encoding='utf-8'))
        edges = []
        sections = [('', data), *data.get('target', {}).items()]
        for target, section in sections:
            for kind in ('dependencies', 'build-dependencies', 'dev-dependencies'):
                for name, value in section.get(kind, {}).items():
                    spec = {'version': value} if isinstance(value, str) else value
                    base = inherited.get(name, {}) if spec.get('workspace') else {}
                    base = {'version': base} if isinstance(base, str) else base
                    combined = {**base, **spec}
                    # A member cannot cancel default features enabled by workspace inheritance.
                    defaults = (base.get('default-features', True) or spec.get('default-features', False)
                                if spec.get('workspace') else spec.get('default-features', True))
                    features = tuple(sorted(set(base.get('features', []) + spec.get('features', []))))
                    edges.append(Edge(name, combined.get('package', name), f'{kind} {target}'.strip(),
                                      bool(combined.get('optional')), bool(defaults), features))
        packages[data['package']['name']] = {'path': str(path.relative_to(root)),
                                             'features': data.get('features', {}), 'edges': edges}
    return packages


def _activates_mode(manifests, affected, package, feature):
    """Follow possible feature activation, including aliases and weak edges.

    A weak dependency feature can still leak a mode when another option enables
    that dependency. This is the all-path manifest policy, not Cargo resolution.
    Cycles are visited once; ordinary dependency activation with defaults off
    remains valid when mode selection is forwarded separately.
    """
    pending, seen = [(package, feature)], set()
    while pending:
        package, feature = pending.pop()
        if (package, feature) in seen:
            continue
        seen.add((package, feature))
        modes = SOLVER_MODES.get(package, set())
        if package in affected:
            modes = {'deterministic', 'fast-simd8'} if package in LEAVES else {
                'physics-deterministic', 'physics-fast-simd8'}
        if feature in modes:
            return True
        data = manifests.get(package)
        if data is None:
            continue
        # Explicit local features take precedence over optional implicit ones.
        entries = data['features'].get(feature)
        if entries is None:
            entries = [f'dep:{feature}']
        for entry in entries:
            if '/' in entry:
                alias, selected = entry.split('/', 1)
                pending.extend((edge.package, selected) for edge in data['edges']
                               if edge.name == alias.removesuffix('?'))
            elif entry.startswith('dep:'):
                for edge in data['edges']:
                    if edge.name == entry[4:]:
                        pending.extend((edge.package, selected) for selected in edge.features)
                        if edge.defaults:
                            pending.append((edge.package, 'default'))
            else:
                pending.append((package, entry))
    return False


def manifest_errors(root, packages=None):
    """Return policy failures across all possible normal/build/dev/optional paths.

    packages restricts roots, not the ancestry policy. Optional paths are included
    conservatively here; selected_graph_errors checks an actual chosen normal graph.
    """
    manifests = _load(root)
    affected = set(LEAVES) & manifests.keys()
    while True:
        more = {name for name, data in manifests.items()
                if any(edge.package in affected or edge.package in ('rapier2d', 'rapier3d')
                       for edge in data['edges'])}
        if more <= affected:
            break
        affected |= more
    selected = set(manifests) if packages is None else set(packages)
    unknown = selected - manifests.keys()
    if unknown:
        return [f'unknown selected package: {name}' for name in sorted(unknown)]
    pending = list(selected)
    while pending:
        for edge in manifests[pending.pop()]['edges']:
            if edge.package in manifests and edge.package not in selected:
                selected.add(edge.package)
                pending.append(edge.package)
    errors = []
    for name in sorted(selected):
        data = manifests[name]
        features = data['features']
        for edge in data['edges']:
            if edge.package in ('rapier2d', 'rapier3d') and set(edge.features) & {'enhanced-determinism', 'simd8'}:
                errors.append(f'{name}: unconditional Rapier mode flag on {edge.kind} {edge.name}')
            elif any(_activates_mode(manifests, affected, edge.package, flag) for flag in edge.features):
                errors.append(f'{name}: unconditional physics feature on {edge.kind} {edge.name}')
        allowed = ({'default', 'deterministic', 'fast-simd8'} if name in LEAVES else
                   {'default', 'physics-deterministic', 'physics-fast-simd8'}) if name in affected else set()
        for feature in features.keys() - allowed:
            if _activates_mode(manifests, affected, name, feature):
                errors.append(f'{name}: unrelated feature {feature} activates physics mode')
        if name not in affected:
            continue
        leaf = name in LEAVES
        det = 'deterministic' if leaf else 'physics-deterministic'
        fast8 = 'fast-simd8' if leaf else 'physics-fast-simd8'
        expected_defaults = DEFAULTS.get(name, set()) | {det}
        if set(features.get('default', [])) != expected_defaults:
            errors.append(f'{name}: defaults must preserve {sorted(expected_defaults)}')
        expected = {det: set(), fast8: set()}
        for edge in data['edges']:
            if edge.package in affected:
                if edge.defaults:
                    errors.append(f'{name}: default-enabled physics edge {edge.kind} {edge.name}')
                suffix = '?' if edge.optional else ''
                for mode, dest in ((det, 'deterministic'), (fast8, 'fast-simd8')):
                    if edge.package not in LEAVES:
                        dest = 'physics-' + dest
                    expected[mode].add(f'{edge.name}{suffix}/{dest}')
            elif edge.package in ('rapier2d', 'rapier3d'):
                expected[det].add(f'{edge.name}/enhanced-determinism')
                expected[fast8].add(f'{edge.name}/simd8')
        for mode, entries in expected.items():
            actual = set(features.get(mode, []))
            for missing in sorted(entries - actual):
                errors.append(f'{name}: {mode} missing forwarding {missing}')
            for extra in sorted(actual - entries):
                errors.append(f'{name}: {mode} unexpected activation/forwarding {extra}')
    return errors


def feature_errors(features, mode, target):
    """Check complete 2D/3D solver feature sets; no cross-mode numeric claim."""
    if mode not in MODES:
        raise ValueError(f'unknown physics mode: {mode}')
    if not re.fullmatch(r'[A-Za-z0-9_-]+', target):
        raise ValueError('explicit target triple required')
    errors = []
    for name in SOLVERS:
        if name not in features:
            errors.append(f'missing target physics dependency: {name}')
            continue
        actual = set(features[name])
        expected = ({'libm_force': mode == 'deterministic'} if name == 'simba' else
                    {'enhanced-determinism': mode == 'deterministic',
                     'simd8': mode == 'fast-simd8', 'parallel': not target.startswith('wasm')})
        if name.startswith('parry'):
            expected['rayon'] = not target.startswith('wasm')
        for flag, enabled in expected.items():
            if (flag in actual) != enabled:
                errors.append(f'{name}: {flag} must be {"enabled" if enabled else "disabled"} for {mode}/{target}')
    return errors


def selected_graph_errors(text, mode, target):
    """Read Cargo tree --prefix none --format '{p}|{f}' -e normal output."""
    features = {}
    errors = []
    for line in text.splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v[^|]+\|([^|]*)', line.removesuffix(' (*)'))
        if not match:
            errors.append(f'invalid selected Cargo graph line: {line[:160]}')
            continue
        name, flags = match.groups()
        if name not in SOLVERS:
            continue
        flags = set(filter(None, flags.split(',')))
        if name in features and features[name] != flags:
            errors.append(f'conflicting selected graph units for {name}')
        features[name] = flags
    return errors + feature_errors(features, mode, target)


def _compiler_package_identity(record, name):
    package_id = record.get('package_id')
    if not isinstance(package_id, str):
        return False
    version = VERSIONS[name]
    if not package_id.startswith('path+'):
        return package_id.endswith(f'#{name}@{version}')
    # Cargo omits the package name when it equals the path's final component.
    # Only the repository's pinned Rapier/Parry patches may use a path identity here.
    if name not in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d'):
        return False
    package = (Path(__file__).resolve().parents[1] / 'third_party' / name).resolve()
    expected = 'path+' + package.as_uri()
    manifest = record.get('manifest_path')
    return (package_id in (f'{expected}#{version}', f'{expected}#{name}@{version}')
            and isinstance(manifest, str) and Path(manifest).is_absolute()
            and Path(manifest).resolve() == package / 'Cargo.toml')


def validate_artifacts(messages, mode, *, target_dir, target, profile):
    """Return checked target feature evidence, or raise ValueError atomically.

    No host artifact can satisfy a missing target record. Paths come from actual
    Cargo compiler-artifact filenames and must be inside TARGET_DIR/TRIPLE/PROFILE.
    This validates the receipt, not the contents/authenticity of a supplied binary.
    """
    if not re.fullmatch(r'[A-Za-z0-9_-]+', target) or not re.fullmatch(r'[A-Za-z0-9_-]+', profile):
        raise ValueError('explicit target triple and profile required')
    target_dir = Path(target_dir).resolve()
    profile_dir = 'debug' if profile in ('dev', 'test') else profile
    destination = target_dir / target / profile_dir
    host = target_dir / profile_dir
    found, ignored_host, errors, finished = {}, set(), [], []
    for record in messages:
        if record.get('reason') == 'build-finished':
            finished.append(record.get('success'))
        if record.get('reason') != 'compiler-artifact':
            continue
        name = record.get('target', {}).get('name')
        if name not in SOLVERS:
            continue
        if (record['target'].get('kind') != ['lib']
                or not _compiler_package_identity(record, name)):
            errors.append(f'{name}: unexpected compiler package/version/library identity')
            continue
        paths = record.get('filenames')
        if not isinstance(paths, list) or not paths or any(not isinstance(p, str) or not Path(p).is_absolute() for p in paths):
            errors.append(f'{name}: missing/relative compiler artifact filenames')
            continue
        resolved = [Path(p).resolve() for p in paths]
        if all(p.is_relative_to(host) for p in resolved):
            ignored_host.add(name)
            continue
        if not all(p.is_relative_to(destination) for p in resolved):
            errors.append(f'{name}: artifact outside requested target output path')
            continue
        flags = record.get('features')
        if not isinstance(flags, list) or any(not isinstance(f, str) for f in flags):
            errors.append(f'{name}: malformed compiler feature record')
            continue
        flags = sorted(set(flags))
        if name in found and found[name] != flags:
            errors.append(f'{name}: conflicting target compiler units')
        found[name] = flags
    if finished != [True]:
        errors.append('expected exactly one successful Cargo build-finished record')
    errors.extend(feature_errors(found, mode, target))
    if errors:
        raise ValueError('\n'.join(errors))
    return {'mode': mode, 'target': target, 'profile': profile,
            'target_output': str(destination), 'features': found,
            'ignored_build_host': sorted(ignored_host)}


def compiler_messages(path):
    records = []
    for line in Path(path).read_text(encoding='utf-8').splitlines():
        if line.startswith('{'):
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError('expected Cargo JSON object')
            records.append(value)
    return records


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    manifest = commands.add_parser('manifest')
    manifest.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    manifest.add_argument('--package', action='append')
    for command in ('graph', 'artifacts'):
        part = commands.add_parser(command)
        part.add_argument('input', type=Path)
        part.add_argument('--mode', choices=MODES, required=True)
        part.add_argument('--target', required=True)
        if command == 'artifacts':
            part.add_argument('--target-dir', type=Path, required=True)
            part.add_argument('--profile', required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == 'manifest':
            errors = manifest_errors(args.root, args.package)
        elif args.command == 'graph':
            errors = selected_graph_errors(args.input.read_text(encoding='utf-8'), args.mode, args.target)
        else:
            receipt = validate_artifacts(compiler_messages(args.input), args.mode,
                                         target_dir=args.target_dir, target=args.target, profile=args.profile)
            print(json.dumps(receipt, indent=2))
            return 0
        if errors:
            raise ValueError('\n'.join(errors))
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        return 1
    print('physics feature audit passed')
    return 0


if __name__ == '__main__':
    sys.exit(main())
