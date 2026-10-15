#!/usr/bin/env python3
"""One manual Windows DX12 library diagnostic; no window or performance gate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process

GIB = 1024**3
MIB = 1024**2
STAGES = ('seed', 'reuse-stream', 'reuse-classic', 'disabled', 'lifetime', 'corrupt', 'missing', 'empty')
TEST = 'dx12_library::tests::dx12_library_native_stage'
BASE_FEATURES = {'default', 'physics-deterministic', 'dx12-pipeline-library', 'persistent-pipeline-cache'}
COOKED = {'cooked-native-shaders', 'cooked-materials', 'cooked-geometry', 'cooked-output',
          'cooked-sprites', 'cooked-ui', 'cooked-text', 'cooked-clusters', 'cooked-bloom',
          'cooked-ssao', 'cooked-temporal', 'cooked-metalfx-boundary'}
COUNTERS = {'stream_hits', 'classic_hits', 'compute_hits', 'miss_or_mismatch', 'native_load_errors',
            'fallback_attempts', 'stores', 'store_errors', 'strict_refusals', 'serialization_errors'}
NATIVE_MISS = re.compile(r'Native \{ operation: "Load(?:Pipeline|GraphicsPipeline|ComputePipeline)", code: -2147024809 \}')
REQUIRED_TESTS = {
    'native_descriptor_control::tests::' + name for name in (
        'native_render_fields_and_root_identity_change_semantic_name_without_struct_bytes',
        'payload_addresses_do_not_select_name_but_shader_semantics_layout_and_output_do',
        'compute_keys_bind_actual_shader_root_node_flags_and_cached_blob')}
REQUIRED_TESTS |= {'native_backing_control::tests::' + name for name in (
    'prepared_pointer_remains_stable_and_failed_creation_releases_backing',
    'failed_attach_and_unwind_release_unreturned_object_before_backing',
    'successful_com_attachment_keeps_backing_until_last_retained_interface')}
REQUIRED_TESTS |= {'windows_private::tests::' + name for name in (
    'canonical_private_leaf_roundtrips_atomically_and_keeps_ancestors_pinned',
    'hardlinks_unsafe_destination_and_existing_staging_never_replace_prior_bytes',
    'broad_unprotected_and_null_dacl_are_refused_without_repair',
    'held_private_leaf_rechecks_new_file_acl_before_native_read',
    'reparse_leaf_and_file_never_redirect_trusted_native_input')}
REQUIRED_TESTS.add('windows_private::security::tests::exact_owner_check_refuses_another_valid_sid_without_changing_any_acl')
REQUIRED_TESTS |= {'native_key_control::tests::' + name for name in (
    'byte_fields_are_length_delimited_and_domain_separated', 'scalar_bits_and_order_do_not_collapse')}
REQUIRED_TESTS |= {'library_codec::tests::' + name for name in (
    'exact_source_adapter_driver_and_field_boundaries_select_identity',
    'bounded_envelope_refuses_wrong_identity_mutation_truncation_and_declared_overflow')}


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def tree_bytes(root):
    root = Path(root)
    # Cargo retires temporary directories as well as files during compilation.
    # Admit only a vanished descendant; a lost root or unreadable subtree must
    # still stop the observer instead of reporting a deceptively small target.
    root.stat()

    def walk_error(error):
        path = Path(error.filename) if error.filename is not None else None
        if (isinstance(error, FileNotFoundError) and path is not None
                and path != root and path.is_relative_to(root)):
            return
        raise error

    total = 0
    for directory, _, files in os.walk(root, onerror=walk_error):
        for name in files:
            try:
                total += (Path(directory) / name).lstat().st_size
            except FileNotFoundError:
                pass  # Owned compiler temporary retired during observation.
    root.stat()
    return total


def validate_cpu_log(text):
    totals = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', text)
    observed = re.findall(r'^test pipeline_cache::(\S+) \.\.\. ok$', text, re.MULTILINE)
    names = set(observed)
    require(len(totals) == 1 and int(totals[0]) >= len(REQUIRED_TESTS)
            and len(names) == len(observed) == int(totals[0]) and REQUIRED_TESTS <= names,
            'Required native descriptor/COM/ACL controls did not all run')
    return int(totals[0])


def artifact(text, cooked, target):
    rows = [json.loads(line) for line in text.splitlines() if line.startswith('{')]
    rows = [row for row in rows if row.get('reason') == 'compiler-artifact'
            and row.get('target', {}).get('name') == 'incant_render'
            and row.get('profile', {}).get('test') and row.get('executable')]
    require(len(rows) == 1, 'Exactly one linked renderer test artifact required')
    row = rows[0]
    require(set(row['features']) == BASE_FEATURES | (COOKED if cooked else set()), 'Actual target features differ')
    path = Path(row['executable'])
    require(path.is_absolute() and path.resolve().is_relative_to(target.resolve())
            and path.is_file() and path.suffix.lower() == '.exe', 'Expected owned Windows executable')
    return path, row['features']


def validate_counts(counts, stage, final=False):
    require(set(counts) == COUNTERS | {'disabled', 'overflowed', 'last_error'}, 'Native counter schema')
    require(all(type(counts[key]) is int and 0 <= counts[key] <= 2**64 - 1 for key in COUNTERS), 'Exact u64 native counters')
    require(type(counts['disabled']) is bool and type(counts['overflowed']) is bool
            and not counts['overflowed'], 'Counter state')
    require(counts['native_load_errors'] == counts['store_errors'] == counts['serialization_errors'] == 0,
            'Native operation errors')
    if final and stage != 'seed':
        require(counts['strict_refusals'] > 0 and counts['stores'] == counts['fallback_attempts'] == 0,
                'Late refusal must not fall back or store')
        require(counts['disabled'] is (stage == 'disabled'), 'Explicit disable state')
        if stage == 'disabled':
            require(counts['last_error'] is None and counts['miss_or_mismatch'] == 0, 'Disable refuses before native load')
        else:
            require(isinstance(counts['last_error'], str) and NATIVE_MISS.fullmatch(counts['last_error'])
                    and counts['miss_or_mismatch'] > 0, 'Retained native late-miss reason')
        return
    require(not counts['disabled'] and counts['strict_refusals'] == 0,
            'Initial catalog must be healthy')
    if stage == 'seed':
        require(counts['fallback_attempts'] == counts['stores'] == counts['miss_or_mismatch'] > 0
                and counts['stream_hits'] == counts['classic_hits'] == counts['compute_hits'] == 0,
                'Fresh seed stores actual misses, never claims hits')
        require(isinstance(counts['last_error'], str) and NATIVE_MISS.fullmatch(counts['last_error']),
                'Capture retains the expected native load miss even after successful store')
    else:
        require(counts['last_error'] is None, 'Strict initial native error')
        require(counts['compute_hits'] > 0 and counts['miss_or_mismatch'] == counts['fallback_attempts'] == counts['stores'] == 0,
                'Strict reuse requires actual hits without fallback')
        selected, excluded = ('classic_hits', 'stream_hits') if stage == 'reuse-classic' else ('stream_hits', 'classic_hits')
        require(counts[selected] > 0 and counts[excluded] == 0, 'Selected native descriptor path')


def compare_frames(paths):
    require(paths, 'Nonempty raw comparison')
    raw = [Path(path).read_bytes() for path in paths]
    require(all(len(value) == 160 * 96 * 4 for value in raw), 'Exact RGBA dimensions')
    require(all(value == raw[0] for value in raw), 'Zero-tolerance raw parity')
    require(all(value == 255 for value in raw[0][3::4]), 'Opaque exact coverage')
    for channel in range(3):
        require(sum(raw[0][i + channel] > 150 and all(raw[0][i + channel] - raw[0][i + c] > 80
                    for c in range(3) if c != channel) for i in range(0, len(raw[0]), 4)) > 50,
                'Meaningful mixed fixture colors')
    return hashlib.sha256(raw[0]).hexdigest()


class CleanupObserver:
    """Observe the existing native_process owner, never replace its cleanup."""
    def __init__(self, tree, rows, save):
        self.tree, self.rows, self.save = tree, rows, save
        self.original_init, self.original_stop = tree.__init__, tree.stop
        self.by_object = {}

    def install(self):
        observer = self

        def initialize(tree, *args, **kwargs):
            try:
                observer.original_init(tree, *args, **kwargs)
            except BaseException as error:
                process = getattr(tree, 'process', None)
                observer.rows.append({'pid': getattr(process, 'pid', None),
                                      'cleanup_confirmed': False, 'constructor_error': repr(error)})
                observer.save()
                raise
            row = {'pid': tree.process.pid, 'cleanup_confirmed': False}
            observer.rows.append(row)
            observer.by_object[id(tree)] = row
            try:
                observer.save()
            except BaseException:
                # The caller does not yet own a successfully constructed tree.
                # Retire the gated child even when receipt publication fails.
                stop(tree)
                raise

        def stop(tree):
            row = observer.by_object[id(tree)]
            try:
                observer.original_stop(tree)
            except BaseException as error:
                row['cleanup_error'] = repr(error)
                raise
            else:
                row['cleanup_confirmed'] = True
            finally:
                observer.save()

        self.tree.__init__, self.tree.stop = initialize, stop

    def restore(self):
        self.tree.__init__, self.tree.stop = self.original_init, self.original_stop


class Probe:
    def __init__(self, output, target, expected_source):
        require(os.name == 'nt', 'Native Windows only; Mac must not execute this probe')
        require(re.fullmatch('[0-9a-f]{40}', expected_source), 'Exact expected source commit')
        self.output, self.target = output.resolve(), target.resolve()
        require(not output.exists() and not output.is_symlink() and not target.exists()
                and not target.is_symlink(), 'Fresh output and private target required')
        require(not self.output.is_relative_to(self.target) and not self.target.is_relative_to(self.output), 'Separate owned paths')
        require(shutil.disk_usage(self.target.parent).free >= 6 * GIB + 8 * GIB + 640 * MIB, 'Launch headroom')
        self.output.mkdir(parents=True)
        self.target.mkdir()
        self.start = time.monotonic()
        self.deadline = self.start + 3600
        self.cancelled = None
        self.record = {'format': 'incant-dx12-native-probe-v1', 'expected_source': expected_source,
                       'status': 'incomplete', 'calls': [], 'owned_processes': [], 'artifacts': [],
                       'claims': 'native library membership and exact pixels only; no window, appearance or performance gate'}
        self.env = dict(os.environ)
        blocked = ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
                   'RUSTDOC', 'RUSTDOCFLAGS', 'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_BUILD_RUSTC',
                   'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTDOC',
                   'CARGO_BUILD_TARGET', 'CARGO_TARGET_DIR', 'CLIPPY_ARGS', 'CLIPPY_CONF_DIR',
                   'CC', 'CXX', 'AR', 'CFLAGS', 'CXXFLAGS', 'CPPFLAGS', 'LDFLAGS', 'RUSTC_BOOTSTRAP')
        require(not any(value and (key in blocked or key.startswith(('CARGO_PROFILE_', 'CARGO_FEATURE_', 'INCANT_'))
                        or key.endswith(('_RUSTFLAGS', '_LINKER'))) for key, value in self.env.items()), 'Ambient build override')
        self.env.update(RUSTUP_TOOLCHAIN='1.99.0', RUSTUP_IO_THREADS='2', CARGO_BUILD_JOBS='2',
                        RAYON_NUM_THREADS='2', CARGO_INCREMENTAL='0')
        self.observer = CleanupObserver(native_process._WindowsTree, self.record['owned_processes'], self.save)
        self.last_sample = 0

    def save(self):
        (self.output / 'result.json').write_text(json.dumps(self.record, indent=2) + '\n')

    def check(self):
        require(not self.cancelled and time.monotonic() < self.deadline, self.cancelled or 'Cooperative deadline')
        if time.monotonic() - self.last_sample < 1:
            return
        self.last_sample = time.monotonic()
        free = shutil.disk_usage(self.target).free
        size, output = tree_bytes(self.target), tree_bytes(self.output)
        logs = [p.stat().st_size for p in self.output.rglob('*.log')]
        self.record.update(target_bytes=size, output_bytes=output,
                           minimum_free_bytes=min(self.record.get('minimum_free_bytes', free), free))
        require(free >= 6 * GIB and size <= 8 * GIB and output <= 640 * MIB, 'Storage bound')
        require(all(n <= 16 * MIB for n in logs) and sum(logs) <= 128 * MIB, 'Log bound')

    def run(self, name, argv, timeout=120, extra_env=None):
        self.check()
        row = {'name': name, 'argv': [str(x) for x in argv], 'managed_call_returned': False,
               'cleanup_confirmed': False}
        self.record['calls'].append(row)
        first = len(self.record['owned_processes'])
        self.save()
        try:
            with (self.output / (name + '.stdout.log')).open('xb') as stdout, (self.output / (name + '.stderr.log')).open('xb') as stderr:
                result = native_process.run(argv, cwd=ROOT, env=dict(self.env, **(extra_env or {})), stdout=stdout,
                                            stderr=stderr, timeout=timeout, cancel=self.check)
                row.update(managed_call_returned=True, exit=result.returncode)
            require(result.returncode == 0, 'First failed child: ' + name)
        finally:
            groups = self.record['owned_processes'][first:]
            row['cleanup_confirmed'] = bool(groups) and all(group['cleanup_confirmed'] for group in groups)
            row['call_finished'] = True
            self.save()
        require(row['cleanup_confirmed'], 'Unconfirmed owned cleanup')
        self.check()
        return (self.output / (name + '.stdout.log')).read_text(encoding='utf-8')

    def source_check(self):
        require(self.run('source-' + str(len(self.record['calls'])), ['git', 'rev-parse', 'HEAD']).strip()
                == self.record['expected_source'], 'Source HEAD differs')
        require(not self.run('dirty-' + str(len(self.record['calls'])), ['git', 'status', '--porcelain']).strip(), 'Dirty source')

    def execute(self):
        self.observer.install()
        handlers = {s: signal.signal(s, lambda number, _: setattr(self, 'cancelled', 'signal ' + str(number)))
                    for s in (signal.SIGINT, signal.SIGTERM)}
        try:
            self.source_check()
            self.record['Cargo_lock_sha256'] = digest(ROOT / 'Cargo.lock')
            self.run('toolchain', ['rustup', 'toolchain', 'install', '1.99.0', '--profile', 'minimal', '--component', 'clippy'], 300)
            cargo = Path(self.run('cargo-path', ['rustup', 'which', '--toolchain', '1.99.0', 'cargo']).strip())
            rustc = Path(self.run('rustc-path', ['rustup', 'which', '--toolchain', '1.99.0', 'rustc']).strip())
            self.env['RUSTC'] = str(rustc)
            self.env['PATH'] = str(cargo.parent) + os.pathsep + self.env.get('PATH', '')
            require('release: 1.99.0\n' in self.run('rustc-version', [rustc, '-vV']), 'Pinned Rust version')
            self.record['OS'] = str(sys.getwindowsversion())
            self.record['tools'] = [{'path': str(path), 'sha256': digest(path)} for path in (cargo, rustc)]
            binaries = {}
            for mode in ('ordinary', 'cooked'):
                features = 'dx12-pipeline-library' + (',cooked-native-shaders' if mode == 'cooked' else '')
                common = ['--target', 'x86_64-pc-windows-msvc', '--profile', 'ci', '--locked', '-j2', '--target-dir', self.target]
                text = self.run(mode + '-cpu', [cargo, 'test', '-p', 'incant_render', *common, '--lib', '--message-format=json',
                                '--features', features, 'pipeline_cache::', '--', '--test-threads=1'], 1200)
                count = validate_cpu_log(text)
                path, actual_features = artifact(text, mode == 'cooked', self.target)
                size = path.stat().st_size
                require(sum(row['bytes'] for row in self.record['artifacts']) + size <= 256 * MIB
                        and shutil.disk_usage(self.output).free - size >= 6 * GIB, 'Copy reservation')
                copied = self.output / (mode + '.exe')
                shutil.copy2(path, copied)
                require(digest(path) == digest(copied), 'Copied executable differs')
                binaries[mode] = copied
                self.record['artifacts'].append({'mode': mode, 'path': str(copied), 'bytes': size,
                                               'sha256': digest(copied), 'features': actual_features, 'cpu_tests': count})
                qualified = ','.join('incant_render/' + part for part in features.split(','))
                self.run(mode + '-hal-lint', [cargo, 'clippy', '-p', 'wgpu-hal', '-p', 'incant_render', *common,
                         '--features', qualified, '--lib', '--', '-D', 'warnings'], 1200)
                self.run(mode + '-all-lint', [cargo, 'clippy', '-p', 'incant_render', *common, '--features', features,
                         '--all-targets', '--', '-D', 'warnings'], 1200)
            self.source_check()
            self.deadline = min(self.start + 4500, time.monotonic() + 900)
            frames = []
            for mode, binary in binaries.items():
                row = next(row for row in self.record['artifacts'] if row['mode'] == mode)
                require(digest(binary) == row['sha256'], 'Frozen executable changed')
                mode_root = self.output / mode
                mode_root.mkdir()
                identity = None
                for stage in STAGES:
                    root = mode_root
                    if stage in ('missing', 'empty'):
                        root = mode_root / (stage + '-case')
                        root.mkdir()
                    text = self.run(mode + '-' + stage, [binary, '--ignored', '--exact', TEST, '--nocapture', '--test-threads=1'],
                                    extra_env={'INCANT_DX12_LIBRARY_STAGE': stage, 'INCANT_DX12_LIBRARY_ROOT': str(root)})
                    require(re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', text) == ['1'], 'Exact native stage did not run')
                    folder = root / stage
                    if stage in ('lifetime', 'empty'):
                        require((folder / 'result.txt').is_file(), 'Native control receipt missing')
                        continue
                    if stage in ('missing', 'corrupt'):
                        require((folder / 'refusal.txt').stat().st_size > 0, 'Explicit refusal missing')
                        continue
                    report = json.loads((folder / 'result.json').read_text())
                    require(report['stage'] == stage and report['backend'] == 'Dx12'
                            and report['cooked'] is (mode == 'cooked')
                            and re.fullmatch('[0-9a-f]{64}', report['source']), 'Native report identity')
                    actual = {key: report[key] for key in ('source', 'native_identity', 'adapter')}
                    require(all(isinstance(value, str) and value for value in actual.values()), 'Native adapter/driver identity')
                    identity = identity or actual
                    require(actual == identity, 'Same-mode source or native adapter/driver identity changed')
                    validate_counts(report['counts_before'], stage)
                    validate_counts(report['counts_after'], stage, final=True)
                    require(all(report['counts_after'][key] >= report['counts_before'][key] for key in COUNTERS),
                            'Native counters cannot regress')
                    initial = folder / 'initial.rgba'
                    require(digest(initial) == report['rgba_sha256'], 'Reported raw hash differs')
                    frames.append(initial)
                    if stage != 'seed':
                        frames.append(folder / 'refused.rgba')
                self.record.setdefault('native_sources', {})[mode] = identity
            require(len(frames) == 14, 'Every initial/refused arm required')
            ordinary, cooked = (self.record['native_sources'][mode] for mode in ('ordinary', 'cooked'))
            require(ordinary['source'] != cooked['source'] and ordinary['native_identity'] == cooked['native_identity']
                    and ordinary['adapter'] == cooked['adapter'], 'Separate feature identity on one native adapter/driver')
            self.record['exact_RGBA_sha256'] = compare_frames(frames)
            self.source_check()
            require(digest(ROOT / 'Cargo.lock') == self.record['Cargo_lock_sha256'], 'Cargo.lock changed')
            self.record['status'] = 'passed'
        except BaseException as error:
            self.record.update(status='failed', error=repr(error))
            raise
        finally:
            self.observer.restore()
            for number, handler in handlers.items():
                signal.signal(number, handler)
            self.record.update(terminal=True, elapsed_seconds=time.monotonic() - self.start,
                               cleanup_confirmed=all(row['cleanup_confirmed'] for row in self.record['owned_processes']))
            self.save()
            inventory = [{'path': str(path.relative_to(self.output)), 'bytes': path.stat().st_size, 'sha256': digest(path)}
                         for path in sorted(self.output.rglob('*')) if path.is_file()]
            (self.output / 'files.json').write_text(json.dumps(inventory, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--target-dir', type=Path, required=True)
    parser.add_argument('--expected-source', required=True)
    args = parser.parse_args()
    Probe(args.output, args.target_dir, args.expected_source).execute()


if __name__ == '__main__':
    main()
