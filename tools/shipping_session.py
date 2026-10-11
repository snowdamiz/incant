"""One ephemeral shipping build ledger; never a persistent Cargo cache.

Only exact target artifacts freshly emitted in this session may be reused. Host
build scripts and proc macros are deliberately outside the target proof ledger.
"""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import time

from shipping_symbols import require


def sha(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def snapshot(root):
    paths = subprocess.check_output(['git', 'ls-files', '-z'], cwd=root).split(b'\0')
    return {os.fsdecode(p): sha(root / os.fsdecode(p)) for p in paths if p}


def option(tokens, name):
    values = [tokens[i + 1] for i, token in enumerate(tokens[:-1]) if token == name]
    require(len(values) == 1, f'Expected exactly one {name}: {values}')
    return values[0]


def target_commands(stderr, target, storage):
    result = []
    pending = ''
    for raw in stderr.splitlines(keepends=True):
        if 'Running `' in raw and not pending:
            pending = raw.split('Running `', 1)[1]
        elif pending:
            pending += raw
        else:
            continue
        if not pending.rstrip().endswith('`'):
            continue
        try:
            tokens = shlex.split(pending.rstrip()[:-1])
        except ValueError:
            continue  # Quoted package descriptions can contain newlines/backticks.
        pending = ''
        if '--crate-name' not in tokens:
            continue
        if '--target' not in tokens:
            continue  # The host cooker legitimately has compiler support.
        # Keep exact compiler arguments, excluding Cargo's multiline environment
        # descriptions. Raw Cargo output is separately retained without rewriting.
        start = tokens.index('--crate-name') - 1
        require(start >= 0 and Path(tokens[start]).name == 'rustc', 'Unexpected compiler command')
        line = f'Running `{shlex.join(tokens[start:])}`'
        require(option(tokens, '--target') == target, 'Unexpected target in shipping session')
        out_dir = Path(option(tokens, '--out-dir')).resolve()
        require(out_dir.is_relative_to(storage / target / 'shipping'), 'Target unit escaped session storage')
        features = sorted(re.fullmatch(r'feature="(.*)"', tokens[i + 1])[1]
                          for i, token in enumerate(tokens[:-1]) if token == '--cfg'
                          and re.fullmatch(r'feature="(.*)"', tokens[i + 1]))
        result.append({'line': line, 'crate': option(tokens, '--crate-name'),
                       'features': features, 'out_dir': str(out_dir),
                       'extra_filename': next((tokens[i + 1].split('=', 1)[1]
                           for i, token in enumerate(tokens[:-1]) if token == '-C'
                           and tokens[i + 1].startswith('extra-filename=')), None)})
    require(not pending, 'Incomplete Cargo command evidence')
    return result


class FreshSession:
    """A bounded native-only session with a deletion barrier before execution."""

    def __init__(self, root, storage, rustc, target, split):
        self.root, self.path, self.rustc = map(lambda p: Path(p).resolve(), (root, storage, rustc))
        require(not self.path.exists(), 'Shipping session storage must not preexist')
        require(target.endswith('-apple-darwin'), 'Shared shipping session currently verifies macOS only')
        require(split == 'packed', 'Unexpected shared-session split debug mode')
        self.target, self.split = target, split
        self.compiler = subprocess.check_output([self.rustc, '-vV'], text=True)
        require(f'host: {target}\n' in self.compiler, 'Shared session requires host == target')
        require(not subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=self.root),
                'Freeze tracked sources before the shipping session')
        self.head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=self.root).strip()
        self.sources = snapshot(self.root)
        self.path.mkdir(parents=True)
        self.units, self.builds, self.inputs, self.players = {}, [], [], []
        self.deleted = False

    def check_frozen(self):
        require(subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=self.root).strip() == self.head,
                'Source HEAD changed during shipping session')
        require(snapshot(self.root) == self.sources, 'Tracked source changed during shipping session')
        require(subprocess.check_output([self.rustc, '-vV'], text=True) == self.compiler,
                'Compiler changed during shipping session')

    @contextmanager
    def storage(self):
        require(not self.deleted, 'Shipping build attempted after deletion barrier')
        yield str(self.path)

    def input_directory(self, path):
        self.inputs.append(Path(path).resolve())

    def packaged_player(self, path):
        path = Path(path).resolve()
        require(path.is_file() and not path.is_relative_to(self.path), 'Missing independent packaged player')
        self.players.append((path, sha(path)))

    def bundle_inputs(self, env):
        if not env.get('INCANT_WASM_BUNDLE'):
            return None
        original = Path(env['INCANT_WASM_BUNDLE'])
        path = original.resolve()
        require(not original.is_symlink() and path.is_dir(), 'Missing or symlinked static bundle')
        require(any(path.is_relative_to(parent) for parent in self.inputs),
                'Static bundle must be inside registered disposable authoring inputs')
        files = {}
        for item in sorted(path.rglob('*')):
            require(not item.is_symlink(), 'Symlinked static bundle input')
            if item.is_file():
                files[item.relative_to(path).as_posix()] = sha(item)
            else:
                require(item.is_dir(), 'Unsupported static bundle input')
        require(files, 'Static bundle is empty')
        return {'path': str(path), 'files_sha256': files}

    def build(self, command, *, cwd, env):
        require(not self.deleted and Path(cwd).resolve() == self.root, 'Invalid shipping build boundary')
        # The static SDK proof needs cc's diagnostic command output. Keep this
        # setting identical across the session: changing it between players
        # invalidates cc build-script fingerprints, including QuickJS's units.
        # Copy the caller's environment; standalone probe behavior is unchanged.
        env = dict(env, CC_ENABLE_DEBUG_OUTPUT='1')
        for name, expected in (('--profile', 'shipping'), ('--target', self.target),
                               ('--target-dir', str(self.path)),
                               ('--config', f'profile.shipping.split-debuginfo="{self.split}"')):
            require(option(command, name) == expected, f'Changed shipping build setting: {name}')
        require('--locked' in command and '--message-format=json' in command and '-vv' in command,
                'Shipping build must retain lock and machine/command evidence')
        for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'CARGO_BUILD_RUSTC',
                    'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTC_WRAPPER',
                    'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER'):
            require(not env.get(key), f'Unexpected compiler override: {key}')
        inputs = {key: {'path': str(Path(env[key]).resolve()), 'sha256': sha(Path(env[key]))}
                  for key in ('INCANT_NATIVE_SCRIPT_SOURCE', 'INCANT_NATIVE_SCRIPT_MAP') if env.get(key)}
        bundle = self.bundle_inputs(env)
        self.check_frozen()
        begin = time.monotonic()
        build = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, encoding='utf-8')
        elapsed = time.monotonic() - begin
        evidence = self.path.parent / f'build-{len(self.builds)}'
        evidence.with_suffix('.stdout.jsonl').write_text(build.stdout, encoding='utf-8')
        evidence.with_suffix('.stderr.log').write_text(build.stderr, encoding='utf-8')
        self.check_frozen()
        require(all(sha(Path(value['path'])) == value['sha256'] for value in inputs.values()),
                'Compile input changed during shipping build')
        require(self.bundle_inputs(env) == bundle, 'Static bundle changed during shipping build')
        require(build.returncode == 0, f'Shared shipping build failed:\n{build.stderr[-16000:]}')
        lines = self.record(build.stdout, build.stderr)
        self.builds.append({'command': list(map(str, command)), 'seconds': elapsed,
                            'diagnostic_environment': {'CC_ENABLE_DEBUG_OUTPUT': '1'},
                            'stdout_sha256': hashlib.sha256(build.stdout.encode()).hexdigest(),
                            'stderr_sha256': hashlib.sha256(build.stderr.encode()).hexdigest(),
                            'compile_inputs': inputs, 'static_bundle': bundle,
                            'script_name': env.get('INCANT_NATIVE_SCRIPT_NAME'),
                            'target_units': self.last_units})
        return build, '\n'.join(lines)

    def record(self, stdout, stderr):
        """Join Cargo artifact identity to fresh rustc evidence, or reject reuse."""
        commands = target_commands(stderr, self.target, self.path)
        audited, records = [], []
        for raw in stdout.splitlines():
            if not raw.startswith('{'):
                continue
            artifact = json.loads(raw)
            if artifact.get('reason') != 'compiler-artifact':
                continue
            filenames = [Path(p).resolve() for p in artifact['filenames']]
            if not filenames or not all(p.is_relative_to(self.path / self.target / 'shipping') for p in filenames):
                continue  # Host build artifacts are not target-runtime evidence.
            require(not artifact['profile']['test'], 'Test artifacts cannot prove shipping codegen')
            identity = {key: artifact[key] for key in ('package_id', 'target', 'features', 'profile', 'filenames')}
            key = json.dumps(identity, sort_keys=True)
            hashes = {str(p): sha(p) for p in filenames if p.is_file()}
            require(hashes, 'Target artifact files are missing')
            crate = artifact['target']['name'].replace('-', '_')
            # A larger GPU graph can contain two versions with the same crate
            # name/features. Join libraries using rustc's exact artifact suffix
            # before accepting a unique command; package_id stays in the ledger.
            matches = [c for c in commands if c['crate'] == crate
                       and c['features'] == sorted(artifact['features'])
                       and (artifact.get('executable') or (c['extra_filename'] and any(
                           p.name.startswith(f'lib{crate}{c["extra_filename"]}.')
                           or p.name.startswith(f'{crate}{c["extra_filename"]}.')
                           for p in filenames)))]
            if artifact['fresh']:
                require(not artifact.get('executable'), 'Every root player must compile freshly')
                require(key in self.units, f'Unproven target artifact reuse: {crate}')
                prior = self.units[key]
                require(prior['hashes'] == hashes, f'Reused artifact changed: {crate}')
                require(not matches, f'Ambiguous fresh/reused target artifact: {crate}')
                line = prior['command']
            else:
                require(len(matches) == 1, f'Missing or ambiguous fresh target command: {crate}; '
                        f'artifact features={artifact["features"]}; '
                        f'command features={[c["features"] for c in commands if c["crate"] == crate]}')
                suffix = matches[0]['extra_filename']
                require(suffix and (artifact.get('executable') or any(
                    p.name.startswith(f'lib{crate}{suffix}.') or p.name.startswith(f'{crate}{suffix}.')
                    for p in filenames)), f'Artifact/command metadata mismatch: {crate}')
                line = matches[0]['line']
                require(key not in self.units, f'Target unit unexpectedly rebuilt in one frozen session: {crate}')
                self.units[key] = {'identity': identity, 'hashes': hashes, 'command': line,
                                   'build': len(self.builds)}
            audited.append(line)
            records.append({'crate': crate, 'features': artifact['features'], 'reused': artifact['fresh'],
                            'identity_sha256': hashlib.sha256(key.encode()).hexdigest(),
                            'fresh_build': self.units[key]['build']})
        require(records, 'No target units recorded')
        require(len(set(audited)) == len(audited), 'Ambiguous artifact-to-command mapping')
        self.last_units = records
        return audited

    def seal(self):
        self.check_frozen()
        require(self.builds and self.players, 'Cannot seal an empty shipping proof')
        require(all(not path.exists() for path in self.inputs), 'Authoring input storage remains')
        require(all(path.is_file() and sha(path) == digest for path, digest in self.players),
                'Packaged player missing or changed')
        shutil.rmtree(self.path)
        self.deleted = True
        self.assert_ready()

    def assert_ready(self):
        require(self.deleted and not self.path.exists(), 'Shared build storage remains before player launch')
        require(all(not path.exists() for path in self.inputs), 'Authoring inputs remain before player launch')
        require(all(path.is_file() and sha(path) == digest for path, digest in self.players),
                'Packaged player changed before execution')
        self.check_frozen()

    def report(self):
        self.assert_ready()
        return {'format_version': 1, 'compiler': self.compiler, 'target': self.target, 'profile': 'shipping',
                'source_head': self.head.decode(), 'source_sha256': self.sources, 'builds': self.builds, 'units': list(self.units.values()),
                'build_storage_removed_before_any_player': True,
                'authoring_inputs_removed_before_any_player': True,
                'scope': 'One fresh ephemeral session; no persistent target cache or device performance gate'}
