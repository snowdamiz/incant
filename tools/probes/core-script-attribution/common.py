"""Private, exact-input diagnostic admission. Never imported by the exporter."""
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools'))
from native_package import read, require, sha256

TARGET = 'aarch64-apple-darwin'
DIAGNOSTIC = 'diagnostic-script-phases'
EXTRA = {'incant_game_player', 'incant_script', 'incant_runtime', 'incant_profile'}
FEATURES = ['project', 'render-input', 'profile', DIAGNOSTIC]
NAVIGATION = 'diagnostic-navigation'
NAV_FEATURES = ['project', 'render-input', 'profile', NAVIGATION]
BUDGET = 'diagnostic-script-budget'
BUDGET_FEATURES = ['project', 'render-input', 'profile', BUDGET]

def additions(features):
    require(features in (FEATURES, NAV_FEATURES, BUDGET_FEATURES), 'Unknown diagnostic selection')
    result = {name: {DIAGNOSTIC} for name in EXTRA}
    if features == NAV_FEATURES:
        for flags in result.values(): flags.add(NAVIGATION)
        result['incant_nav'] = {NAVIGATION}
    if features == BUDGET_FEATURES:
        for name in ('incant_game_player', 'incant_script', 'incant_profile'):
            result[name].add(BUDGET)
    return result

TICKS = 2156
SOURCE_ROOTS = ('crates/', 'third_party/', 'sdk/', 'tools/', '.cargo/')
SOURCE_FILES = ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml')


def load(path, limit=16 * 1024**2):
    return json.loads(read(Path(path).absolute(), limit))


def checked_file(row, limit):
    path = Path(row['path']).absolute()
    raw = read(path, limit)
    require(sha256(raw) == row['sha256'], 'Frozen input changed: ' + path.name)
    return path, raw


def plan(path):
    raw = read(Path(path).absolute(), 4 * 1024**2)
    data = json.loads(raw)
    require(data['format'] == 'incant-script-attribution-private-plan' and data['version'] == 1,
            'Unsupported private diagnostic plan')
    require(data['target'] == TARGET and data['physics_mode'] == 'fast'
            and data['features'] in (FEATURES, NAV_FEATURES, BUDGET_FEATURES) and data['ticks'] == TICKS
            and data['child_seconds'] == 45 and data['retry_count'] == 0
            and data['eligible_for_performance_gate'] is False, 'Diagnostic contract changed')
    return data, sha256(raw)


def source_inventory(files, tracked):
    """Bind every tracked compiler/helper input, not a caller-selected subset."""
    require(isinstance(files, dict) and 1 <= len(files) <= 10000, 'Invalid source inventory')
    expected = {name for name in tracked if name.startswith(SOURCE_ROOTS) or name in SOURCE_FILES}
    require(set(files) == expected, 'Incomplete tracked compiler/source inventory')


def source_check(data):
    require(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True, timeout=10).strip()
            == data['source_commit'], 'Diagnostic source commit changed')
    require(not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, timeout=10).strip(),
            'Diagnostic source is dirty')
    _, raw = checked_file(data['source_manifest'], 4 * 1024**2)
    manifest = json.loads(raw)
    require(manifest['source_commit'] == data['source_commit'], 'Inventory commit differs')
    files = manifest['files']
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT, timeout=10).decode().split('\0')[:-1]
    source_inventory(files, tracked)
    for name, digest in files.items():
        p = Path(name)
        require(not p.is_absolute() and '..' not in p.parts, 'Invalid source name')
        require(sha256(read(ROOT / p, 64 * 1024**2)) == digest, 'Source changed: ' + name)
    return {'commit': data['source_commit'], 'files': len(files),
            'manifest_sha256': data['source_manifest']['sha256']}


def authored(data):
    root = Path(data['authoring_root']).absolute()
    _, raw = checked_file(data['authoring_manifest'], 4 * 1024**2)
    files = json.loads(raw)['authoring_files']
    require(len(files) == 74, 'Expected original 74 authored inputs')
    actual = {p.relative_to(root).as_posix(): sha256(read(p, 256 * 1024**2))
              for p in root.rglob('*') if p.is_file() or p.is_symlink()}
    require(actual == files, 'Authored inputs differ from fixed Core workload')
    return root / 'core.json'


def graph_identity(text):
    result = {}
    for line in text.splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v([^ |]+)(?: \([^|]+\))?\|([^|]*)',
                             line.removesuffix(' (*)'))
        require(match is not None, 'Malformed normal graph')
        name, version, flags = match.groups()
        key = (name, version)
        item = tuple(sorted(filter(None, flags.split(','))))
        result.setdefault(key, set()).add(item)
    return result


def graph_delta(actual, baseline, features=FEATURES):
    extra = additions(features)
    actual, baseline = graph_identity(actual), graph_identity(baseline)
    require(actual.keys() == baseline.keys(), 'Normal dependency closure changed')
    changes = {}
    for key, rows in actual.items():
        name, version = key
        # Cargo may report distinct host/target feature rows for one version.
        # Preserve those rows; unioning them would hide a missing unit.
        wanted = {tuple(sorted(set(flags) | extra.get(name, set())))
                  for flags in baseline[key]}
        require(rows == wanted, 'Additional normal graph difference: ' + name)
        if name in extra:
            require(name not in changes, 'Multiple diagnostic crate versions')
            changes[name] = {'version': version,
                             'before': [list(flags) for flags in sorted(baseline[key])],
                             'after': [list(flags) for flags in sorted(rows)]}
    require(changes.keys() == extra.keys(), 'Missing diagnostic feature recipients')
    return changes


def compiler_features(messages, target_dir, baseline, features=FEATURES):
    """Exact target flags; build-host units cannot stand in for runtime units."""
    extra = additions(features)
    require(set(baseline) >= extra.keys(), "Missing diagnostic compiler recipients")
    target = target_dir / TARGET / 'shipping'
    host = target_dir / 'shipping'
    names = set(baseline)
    found, kinds, ignored = {}, set(), set()
    require([m.get('success') for m in messages if m.get('reason') == 'build-finished'] == [True],
            'Expected one completed compiler build')
    for row in messages:
        if row.get('reason') != 'compiler-artifact' or row['target']['name'] not in names:
            continue
        name = row['target']['name']
        files = [Path(p).resolve() for p in row['filenames']]
        require(files, 'Missing compiler outputs')
        if all(p.is_relative_to(host) for p in files):
            ignored.add(name)
            continue
        require(all(p.is_relative_to(target) for p in files), 'Unknown compiler output location')
        require(row['target']['kind'] == ['lib'] or
                name == 'incant_game_player' and row['target']['kind'] == ['bin'],
                'Unexpected target unit')
        if name == 'incant_game_player':
            kinds.add(row['target']['kind'][0])
        flags = set(row['features'])
        require(name not in found or found[name] == flags, 'Conflicting actual target flags')
        wanted = set(baseline[name]) | extra.get(name, set())
        require(flags == wanted, 'Unexpected target compiler features: ' + name)
        found[name] = flags
    require(found.keys() == names and kinds == {'lib', 'bin'}, 'Missing actual target units')
    require(not ({'compiler', 'native-checkpoints', 'physics-fast-simd8'} & found['incant_script']),
            'Unexpected runtime compiler/checkpoint/eight-lane feature')
    return {'features': {k: sorted(v) for k, v in found.items()},
            'ignored_build_host': sorted(ignored), 'extra_feature': features[-1]}


def package_identity(directory):
    """Read only private diagnostic receipts; normal export admission is untouched."""
    directory = Path(directory).absolute()
    require(not (directory / 'export.json').exists(), 'Diagnostic must not masquerade as export')
    raw = read(directory / 'diagnostic-build.json', 16 * 1024**2)
    receipt = json.loads(raw)
    require(receipt['format'] == 'incant-script-attribution-private-build'
            and receipt['version'] == 1 and receipt['diagnostic_only'] is True
            and receipt['eligible_for_distribution'] is False
            and receipt['eligible_for_performance_gate'] is False,
            'Unsupported diagnostic build')
    for name, digest in receipt['artifacts'].items():
        p = Path(name)
        require(not p.is_absolute() and '..' not in p.parts, 'Invalid artifact name')
        require(sha256(read(directory / p, 64 * 1024**2)) == digest, 'Build artifact changed: ' + name)
    player = directory / 'player/incant_game_player'
    require(sha256(read(player, 256 * 1024**2)) == receipt['player_sha256'], 'Player changed')
    actual = {p.relative_to(player.parent / 'data').as_posix(): sha256(read(p, 256 * 1024**2))
              for p in (player.parent / 'data').rglob('*') if p.is_file() or p.is_symlink()}
    require(actual == receipt['data']['files'], 'Private package data changed')
    return player, receipt, sha256(raw)
