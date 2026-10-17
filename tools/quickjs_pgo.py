"""Cold protocol for the explicit nonshipping Core C-PGO probe.

This is not an exporter option. Project data cannot select a tool or C flag.
"""
from pathlib import Path
import hashlib
import json
import re
import shlex
import shutil

from native_package import read, require

TARGET = 'aarch64-apple-darwin'
MODES = ('ordinary', 'generate', 'use')
UNITS = ('libregexp.c', 'libunicode.c', 'quickjs.c', 'dtoa.c')
HEADERS = ('builtin-array-fromasync.h', 'builtin-iterator-zip-keyed.h', 'builtin-iterator-zip.h',
           'cutils.h', 'dtoa.h', 'libregexp-opcode.h', 'libregexp.h', 'libunicode-table.h',
           'libunicode.h', 'list.h', 'quickjs-atom.h', 'quickjs-c-atomics.h',
           'quickjs-opcode.h', 'quickjs.h')
FEATURES = {m: 'diagnostic-quickjs-' + m for m in ('generate', 'use')}
RETAINED = ('_JS_NewNumber', '_JS_DupValueRT', '_JS_ThrowOutOfMemory',
            '_JS_GetRuntimeOpaque', '_JS_SetRuntimeOpaque', '_JS_GetGCThreshold')
SELECTOR = 'INCANT_QUICKJS_PGO_DESCRIPTOR'


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def file_identity(path, cap=256 * 1024**2):
    path = Path(path)
    require(path.is_absolute() and str(path.resolve()) == str(path), 'Canonical absolute PGO input required')
    value = read(path, cap)
    return {'path': str(path), 'bytes': len(value), 'sha256': hashlib.sha256(value).hexdigest()}


def create_json(path, value):
    value = json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + '\n'
    with Path(path).open('x', encoding='utf-8') as stream:
        stream.write(value)


def additions(descriptor):
    result = ['-Werror=unknown-warning-option']
    if descriptor['mode'] == 'generate':
        return result + ['-fprofile-generate=' + descriptor['raw_directory']]
    require(descriptor['mode'] == 'use', 'Unknown PGO descriptor mode')
    return result + ['-fprofile-use=' + descriptor['profile']['path'],
                     '-Werror=profile-instr-missing', '-Werror=profile-instr-out-of-date',
                     '-Werror=profile-instr-unprofiled', '-Werror=backend-plugin',
                     '-mllvm', '-pgo-warn-missing-function']


def insert_flags(base, extra):
    require(base and base[-1] == '-D_GNU_SOURCE' and '-w' not in base,
            'Exact GNU-only macro tail and visible diagnostics required')
    require(not any('profile-' in x or x.startswith('@') for x in base), 'Nonordinary C base vector')
    return [*base[:-1], *extra, base[-1]]


def descriptor(mode, tools, base_args, vendor, *, profile=None, raw_directory=None):
    require(mode in ('generate', 'use'), 'No custom measured baseline')
    require(set(tools) == {'compiler', 'archiver', 'profdata_tool', 'profile_runtime',
                          'sdk', 'resource_directory', 'tool_inputs'}, 'Closed tool closure')
    require((mode == 'use') == (profile is not None)
            and (mode == 'generate') == (raw_directory is not None), 'Mode/profile combination')
    insert_flags(base_args, [])
    sources = {}
    for name in (*HEADERS, *UNITS):
        record = file_identity(Path(vendor) / 'quickjs' / name, 4 * 1024**2)
        sources[name] = {key: record[key] for key in ('bytes', 'sha256')}
    value = dict(version=1, mode=mode, target=TARGET, **tools, sources=sources,
                 recipe={'optimization': 3, 'debug': True, 'base_args': base_args},
                 profile=None if profile is None else file_identity(profile, 64 * 1024**2),
                 raw_directory=None if raw_directory is None else str(Path(raw_directory).resolve(strict=True)))
    require(len(json.dumps(value).encode()) <= 64 * 1024, 'Descriptor exceeds Rust admission cap')
    return value


def identity(value):
    fields = ('mode', 'target', 'sources', 'recipe', 'sdk', 'resource_directory')
    result = {key: value[key] for key in fields}
    result['format'] = 'incant-quickjs-c-pgo-v1'
    content = lambda v: {key: v[key] for key in ('bytes', 'sha256')}
    for key in ('compiler', 'archiver', 'profdata_tool', 'profile_runtime'):
        result[key] = content(value[key])
    result['tool_inputs'] = [{'path': v['path'], 'content': content(v)} for v in value['tool_inputs']]
    result['profile'] = None if value['profile'] is None else content(value['profile'])
    return hashlib.sha256(json.dumps(result, sort_keys=True, separators=(',', ':'),
                                   ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def c_commands(log, target, vendor, *, expected=None):
    """Parse cc's actual compile vectors, keeping both Cargo roles separate."""
    target, vendor = Path(target).resolve(), Path(vendor).resolve()
    found = {}
    for line in log.splitlines():
        match = re.fullmatch(r'\[rquickjs-sys 0\.14\.0\] running: (.+)', line)
        if not match:
            continue
        tokens = shlex.split(match[1])
        if '-c' not in tokens:
            continue
        require(tokens.count('-c') == tokens.count('-o') == 1, 'Exact C compile source/output')
        source = Path(tokens[tokens.index('-c') + 1])
        if source.name not in UNITS:
            require(source.name == 'flag_check.c', 'Unexpected QuickJS C translation unit')
            continue
        output = Path(tokens[tokens.index('-o') + 1])
        require(output.is_absolute(), 'Absolute owned C object output')
        if output.is_relative_to(target / TARGET / 'shipping'):
            role = 'runtime'
        else:
            require(output.is_relative_to(target / 'shipping'), 'C object outside host/runtime target')
            role = 'cooker'
        compiler_index = next((i for i, a in enumerate(tokens) if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*=.*', a)), None)
        require(compiler_index is not None, 'Missing actual C compiler')
        compiler = str(Path(shutil.which(tokens[compiler_index]) or tokens[compiler_index]).resolve(strict=True))
        args = []
        skip = {tokens.index('-c'), tokens.index('-c') + 1, tokens.index('-o'), tokens.index('-o') + 1}
        args = [v for i, v in enumerate(tokens) if i > compiler_index and i not in skip]
        key = (role, source.name)
        require(key not in found, 'Duplicate C translation unit/role')
        if expected is not None:
            require(source == Path('quickjs') / source.name, 'Unstable diagnostic static-function source path')
            require(compiler == expected['compiler']['path']
                    and args == insert_flags(expected['recipe']['base_args'], additions(expected)),
                    'Actual C role recipe mismatch')
        else:
            require(source.is_absolute() and source.parent == output.parent,
                    'Ordinary copied-OUT_DIR source path changed')
        actual_source = source if source.is_absolute() else vendor / source
        require(read(actual_source, 4 * 1024**2) == read(vendor / 'quickjs' / source.name, 4 * 1024**2),
                'Actual C translation unit changed')
        found[key] = {'compiler': compiler, 'args': args, 'source': str(source), 'output': str(output)}
    require(set(found) == {(r, n) for r in ('runtime', 'cooker') for n in UNITS},
            'Both actual four-TU QuickJS roles required')
    for role in ('runtime', 'cooker'):
        require(len({json.dumps(found[role, n]['args']) for n in UNITS}) == 1, 'Per-TU C recipe mismatch')
    return [{'role': r, 'unit': n, **v} for (r, n), v in sorted(found.items())]


def covered_names(raw):
    lines = raw.splitlines()
    require(lines and len(lines) == len(set(lines)) and all(s and s == s.strip() for s in lines),
            'Unique exact covered names required')
    values = set(lines)
    require({'quickjs/quickjs.c;JS_CallInternal', 'JS_ReadObject', 'JS_EvalFunction'} <= values,
            'Core must cover call/read/evaluate under exact relative names')
    require(not any(';JS_CallInternal' in s and s != 'quickjs/quickjs.c;JS_CallInternal' for s in values),
            'Foreign or near-match static name')
    return sorted(values)


def artifact_features(messages, mode, target, root):
    """Bind the actual host/target feature closure, not just requested flags."""
    target, root = Path(target).resolve(), Path(root).resolve()
    known = {'path+'+(root / folder / name).as_uri()+'#'+version: name
             for name, folder, version in (('incant_script', 'crates', '0.0.1'),
                ('incant_cook', 'crates', '0.0.1'), ('incant_game_player', 'crates', '0.0.1'),
                ('rquickjs-sys', 'third_party', '0.14.0'))}
    wanted = set() if mode == 'ordinary' else {FEATURES[mode]}
    found = {}
    for row in messages:
        name = known.get(row.get('package_id'))
        if row.get('reason') != 'compiler-artifact' or name is None:
            continue
        kind = row['target']['kind']
        if kind == ['custom-build']:
            continue
        require(row['target']['name'] == name.replace('-', '_') and
                (kind == ['lib'] or (name == 'incant_game_player' and kind == ['bin'])),
                'Unexpected diagnostic artifact kind')
        paths = [Path(p) for p in row['filenames']]
        require(paths and all(p.is_absolute() for p in paths), 'Missing actual diagnostic artifacts')
        if all(p.is_relative_to(target / TARGET / 'shipping') for p in paths):
            role = 'runtime'
        else:
            require(all(p.is_relative_to(target/'shipping') for p in paths), 'Foreign artifact role')
            role = 'cooker'
        flags = row['features']
        require(type(flags) is list and all(type(f) is str for f in flags)
                and len(flags) == len(set(flags)) and set(flags) & set(FEATURES.values()) == wanted,
                'Actual PGO feature mode mismatch')
        key = (name, role, kind[0])
        require(key not in found, 'Duplicate diagnostic artifact role')
        found[key] = {'package': name, 'role': role, 'kind': kind[0], 'features': sorted(flags)}
    expected = {(p, r, 'lib') for p in ('incant_script', 'rquickjs-sys') for r in ('cooker', 'runtime')}
    expected |= {('incant_cook', 'cooker', 'lib'), ('incant_game_player', 'runtime', 'lib'),
                 ('incant_game_player', 'runtime', 'bin')}
    require(found.keys() == expected, 'Complete actual PGO host/runtime feature graph required')
    return [found[k] for k in sorted(found)]


def final_link(raw, mode, runtime, *, compiler, target):
    require(len(raw.splitlines()) == 1, 'One actual final link command')
    tokens = shlex.split(raw)
    first = next((i for i, v in enumerate(tokens) if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*=.*', v)), None)
    require(first is not None and str(Path(shutil.which(tokens[first]) or tokens[first]).resolve(strict=True)) == compiler,
            'Actual final linker differs from admitted native driver')
    require(tokens.count('-o') == 1 and Path(tokens[tokens.index('-o') + 1]).is_absolute()
            and Path(tokens[tokens.index('-o') + 1]).is_relative_to(Path(target)), 'Owned actual final output')
    args = [part for v in tokens for part in (v[4:].split(',') if v.startswith('-Wl,') else [v])]
    require(args.count('-dead_strip') == (0 if mode == 'generate' else 1), 'Training-only dead-code retention')
    require((str(runtime) in args) == (mode == 'generate'), 'Training-only runtime archive')
    require(mode != 'generate' or '-force_load' in args, 'Training profile runtime force load')
    return {'dead_strip': mode != 'generate', 'profile_runtime': mode == 'generate', 'raw_sha256': hashlib.sha256(raw.encode()).hexdigest()}


def instrumentation(mode, symbols, sections):
    present = '__llvm_profile' in symbols and '__llvm_prf' in sections
    require(present if mode == 'generate' else '__llvm_profile' not in symbols and '__llvm_prf' not in sections,
            'Training-only actual profile symbols/sections')
    return {'profile_sections': mode == 'generate', 'profile_runtime_symbols': mode == 'generate',
            'symbols_sha256': hashlib.sha256(symbols.encode()).hexdigest(),
            'sections_sha256': hashlib.sha256(sections.encode()).hexdigest()}


def training_symbols(raw):
    for name in (*RETAINED, '_JS_ReadObject', '_JS_EvalFunction', '___JS_EvalInternal'):
        require(re.search(r'(?m)^\s*[0-9a-fA-F]+\s+[Tt]\s+' + re.escape(name) + r'$', raw),
                'Training retention failed: ' + name)
    return {'training_only': True, 'source_free_claim': False, 'retained_apis': list(RETAINED),
            'symbols_sha256': hashlib.sha256(raw.encode()).hexdigest()}
