"""Trusted offline-export profiling admission; never project-authored settings."""
from pathlib import Path
import re

from native_package import require

MODES = (None, 'summary', 'tracy')
HOSTS = ('incant_game_player', 'incant_game_runtime', 'incant_runtime', 'incant_script')
VERSIONS = {'tracy-client': '0.18.5', 'tracy-client-sys': '0.29.0'}
TRACY = {'enable', 'manual-lifetime', 'ondemand', 'only-localhost'}


def features(mode, route):
    require(mode in MODES, 'Unknown native profiling mode')
    require(mode is None or route == 'headless', 'Profiling export requires headless mode')
    return [] if mode is None else ['profile-tracy' if mode == 'tracy' else 'profile']


def environment(mode, values):
    if mode is not None:
        # The pinned sys crate accepts replacement libraries and extra C++ flags.
        # Those are not another supported configuration of this audited exporter.
        for name, value in values.items():
            override = (name.startswith(('TRACY_', 'CXXFLAGS', 'CPPFLAGS'))
                        or name.endswith(('_CXXFLAGS', '_CPPFLAGS')))
            compiler = (name == 'CXX' or name.startswith('CXX_') or name.endswith('_CXX'))
            require(not (name.startswith('CARGO_FEATURE_') or compiler or (override and value)),
                    f'Unset {name} for the audited profiling exporter')


def expected(mode):
    require(mode in MODES, 'Unknown native profiling mode')
    result = {name: ({'profile'} if mode else set()) for name in HOSTS}
    result['incant_profile'] = {'summary'} if mode else set()
    if mode == 'tracy':
        result['incant_game_player'].add('profile-tracy')
        result['incant_profile'].add('tracy')
        result.update({'tracy-client': TRACY, 'tracy-client-sys': TRACY | {'delayed-init'}})
    return result


def validate(found, mode):
    want = expected(mode)
    require(set(found) == set(want), 'Missing or unexpected profiling package')
    for name, flags in found.items():
        actual = flags & {'profile', 'profile-tracy'} if name in HOSTS else flags
        require(actual == want[name], f'Unexpected profiling features for {name}: {sorted(actual)}')
    return {name: sorted(flags) for name, flags in sorted(found.items())}


def graph(text, mode):
    found = {}
    for line in text.splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v([^ |]+)(?: \([^|]+\))?\|([^|]*)',
                             line.removesuffix(' (*)'))
        require(match is not None, 'Malformed selected profiling graph')
        name, version, flags = match.groups()
        if name not in (*HOSTS, 'incant_profile', *VERSIONS):
            continue
        require(name not in VERSIONS or version == VERSIONS[name], 'Profiling package version changed')
        flags = set(flags.split(',')) if flags else set()
        require(name not in found or found[name] == flags, 'Conflicting profiling graph units')
        found[name] = flags
    return validate(found, mode)


def artifacts(messages, mode, *, target_dir, target, profile):
    destination = Path(target_dir).resolve() / target / profile
    host = Path(target_dir).resolve() / profile
    found, ignored, player_kinds = {}, set(), set()
    names = {name.replace('-', '_'): name for name in (*HOSTS, 'incant_profile', *VERSIONS)}
    finished = [r.get('success') for r in messages if r.get('reason') == 'build-finished']
    require(finished == [True], 'Profiling audit needs one successful Cargo build')
    for row in messages:
        if row.get('reason') != 'compiler-artifact' or row.get('target', {}).get('name') not in names:
            continue
        name = names[row['target']['name']]
        kind = row['target'].get('kind')
        require(kind == ['lib'] or (name == 'incant_game_player' and kind == ['bin']),
                'Unexpected profiling compiler unit')
        if name in VERSIONS:
            require(str(row.get('package_id', '')).endswith(f'#{name}@{VERSIONS[name]}'),
                    'Profiling compiler package version changed')
        paths = row.get('filenames')
        require(isinstance(paths, list) and paths and all(isinstance(p, str) and Path(p).is_absolute() for p in paths),
                'Missing absolute profiling compiler outputs')
        paths = [Path(p).resolve() for p in paths]
        if all(p.is_relative_to(host) for p in paths):
            ignored.add(name)
            continue
        require(all(p.is_relative_to(destination) for p in paths), 'Profiling output outside target directory')
        if name == 'incant_game_player':
            player_kinds.add(kind[0])
        flags = row.get('features')
        require(isinstance(flags, list) and all(isinstance(f, str) for f in flags), 'Malformed profiling compiler features')
        flags = set(flags)
        require(name not in found or found[name] == flags, 'Conflicting profiling compiler units')
        found[name] = flags
    require(player_kinds == {'lib', 'bin'}, 'Profiling audit needs both player target units')
    return {'mode': mode, 'target': target, 'profile': profile,
            'features': validate(found, mode), 'ignored_build_host': sorted(ignored)}
