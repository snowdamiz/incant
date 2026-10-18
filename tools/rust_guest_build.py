"""Explicit, bounded untrusted Rust-to-Wasm build service; no project mutation.

Tool paths and output/work roots are trusted host configuration, never project
arguments to a command runner. Project authority consists only of a bounded
source package and its entry name. Successful bytes remain untrusted Wasm data.
"""
from dataclasses import asdict, dataclass
import hashlib
import json
import math
import os
from pathlib import Path
import select
import shutil
import stat
import sys
import time
from types import SimpleNamespace

import native_process
import rust_guest
from rust_guest_sandbox import (MacPolicy, Refused, Unsupported, absolute,
                                clean_environment, group_sample, require, trusted_file)
from rust_guest_source import capture, materialize, open_directory, relative_entry

MIB = 1024 * 1024


@dataclass(frozen=True)
class Request:
    source_root: Path
    entry: str
    output: Path
    work: Path


@dataclass(frozen=True)
class Tools:
    """Engine/operator configuration, not serializable project capability data."""
    rustc: Path
    validator: Path


@dataclass(frozen=True)
class Limits:
    seconds: float = 180
    child_seconds: float = 90
    scratch_bytes: int = 512 * MIB
    log_bytes: int = 16 * MIB
    stream_bytes: int = 4 * MIB
    raw_bytes: int = MIB
    prepared_bytes: int = 4 * MIB
    rss_bytes: int = 8 * 1024 * MIB
    floor_bytes: int = 6 * 1024 * MIB

    def validate(self):
        for value in (self.seconds, self.child_seconds):
            require(isinstance(value, (int, float)) and not isinstance(value, bool)
                    and math.isfinite(value) and value > 0, 'Finite positive build deadline required')
        for name, value in asdict(self).items():
            if name not in ('seconds', 'child_seconds'):
                require(type(value) is int and value > 0, 'Positive byte bound required')
        require(self.raw_bytes <= MIB and self.stream_bytes <= self.log_bytes,
                'Normal module/stream bound exceeded')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_file(path, cap):
    """Do not follow a generated link or read an unbounded compiler output."""
    info = path.stat(follow_symlinks=False)
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size <= cap,
            'Regular bounded single-link file required')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require((before.st_dev, before.st_ino) == (info.st_dev, info.st_ino), 'File replaced before read')
        data = bytearray()
        while len(data) <= cap:
            chunk = os.read(fd, min(65536, cap + 1 - len(data)))
            if not chunk:
                break
            data.extend(chunk)
        after = os.fstat(fd)
        require(len(data) <= cap and (before.st_dev, before.st_ino, before.st_size,
                before.st_mtime_ns, before.st_ctime_ns) == (after.st_dev, after.st_ino,
                after.st_size, after.st_mtime_ns, after.st_ctime_ns), 'File changed while read')
        return bytes(data)
    finally:
        os.close(fd)


def write_json(path, value):
    with path.open('x', encoding='utf-8') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')


def inventory(root):
    total = count = 0
    for directory, subdirs, files in os.walk(root, followlinks=False):
        for name in (*subdirs, *files):
            count += 1
            require(count <= 4096, 'Build scratch inventory exceeded')
            info = (Path(directory) / name).lstat()
            require(stat.S_ISDIR(info.st_mode) or (stat.S_ISREG(info.st_mode) and info.st_nlink == 1),
                    'Unexpected scratch link or special file')
            total += info.st_size
    return total


class Owner:
    """Focused adapter to native_process, with finite IO and sampled resources.

    No compiler-text decoding until after whole-group termination. Deadlines,
    RSS, disk and file sizes are sampled, not hard kernel allocation quotas.
    """
    def __init__(self, work, limits, cancel):
        self.work, self.limits, self.cancel = work, limits, cancel
        self.started = time.monotonic()
        self.calls = []
        self.peak_scratch = self.peak_rss = self.log_bytes = 0

    def check(self):
        if self.cancel():
            raise Refused('Build cancelled')
        require(time.monotonic() - self.started < self.limits.seconds, 'Build aggregate deadline')
        self.peak_scratch = max(self.peak_scratch, inventory(self.work))
        require(self.peak_scratch <= self.limits.scratch_bytes, 'Build scratch cap')
        require(shutil.disk_usage(self.work).free >= self.limits.floor_bytes, 'Build free disk floor')

    def run(self, command, *, cwd, env, label):
        self.check()
        call = {'label': label, 'argv': list(map(str, command)), 'pid': None,
                'observed_pids': [], 'returncode': None, 'group_absent': False}
        self.calls.append(call)
        index = len(self.calls)
        outputs = [self.work / f'{index:02}-{label}.{name}' for name in ('stdout', 'stderr')]
        handles = []
        counts = [0, 0]
        start = time.monotonic()
        child = None
        streams = {}
        operation_error = None
        try:
            for path in outputs:
                handles.append(path.open('xb'))
            with native_process.Interactive(command, cwd=cwd, env=env) as child:
                child.stdin.close()  # No project-selected input protocol or shell.
                call['pid'] = child.tree.process.pid
                write_json(self.work / f'{index:02}-{label}.owner.json', call)
                # Duplicates survive Interactive.wait() closing its pipes; this
                # lets us drain the last bounded bytes after managed cleanup.
                for item, stream in enumerate((child.stdout, child.stderr)):
                    streams[os.dup(stream.fileno())] = item
                for stream in streams:
                    os.set_blocking(stream, False)
                observed = set()
                while True:
                    self.check()
                    require(time.monotonic() - start < self.limits.child_seconds, 'Build child deadline')
                    rows = [] if child.closed else group_sample(call['pid'])
                    observed.update(row['pid'] for row in rows)
                    call['observed_pids'] = sorted(observed)
                    self.peak_rss = max(self.peak_rss, sum(row['rss'] for row in rows))
                    require(self.peak_rss <= self.limits.rss_bytes, 'Build sampled RSS cap')
                    ready = select.select(list(streams), [], [], .02)[0]
                    for stream in ready:
                        value = os.read(stream, 65536)
                        if not value:
                            os.close(stream)
                            del streams[stream]
                            continue
                        item = streams[stream]
                        counts[item] += len(value)
                        self.log_bytes += len(value)
                        require(counts[item] <= self.limits.stream_bytes and self.log_bytes <= self.limits.log_bytes,
                                'Build diagnostic output cap')
                        handles[item].write(value)
                        handles[item].flush()
                    if not child.closed and child.tree.exited():
                        # Preserve native_process cleanup/combined-error rules.
                        # Pipe-holding descendants cannot delay this cleanup.
                        child.wait(timeout=max(.001, self.limits.child_seconds - (time.monotonic() - start)))
                        call['returncode'] = child.returncode
                    if child.closed and not streams:
                        break
        except BaseException as error:
            operation_error = error
            call['operation_error'] = repr(error)
            raise
        finally:
            def finalize():
                errors = []
                for close in [*(lambda fd=fd: os.close(fd) for fd in streams),
                              *(handle.close for handle in handles)]:
                    try:
                        close()
                    except BaseException as error:
                        errors.append(error)
                call['elapsed_seconds'] = time.monotonic() - start
                call['stream_bytes'] = counts
                try:
                    if child is not None:
                        # native_process context cleanup ran first. Unknown or
                        # live membership refuses, without erasing its cause.
                        call['returncode'] = child.returncode
                        if sys.platform == 'darwin':
                            deadline = time.monotonic() + 1
                            call['group_absent'] = native_process._darwin_group_is_empty(call['pid'])
                            while not call['group_absent']:
                                require(time.monotonic() < deadline, 'Owned build group absence unproven')
                                time.sleep(.01)
                                call['group_absent'] = native_process._darwin_group_is_empty(call['pid'])
                        require(call['group_absent'], 'Owned build group absence unproven')
                except BaseException as error:
                    errors.append(error)
                call['cleanup_errors'] = list(map(repr, errors))
                try:
                    write_json(self.work / f'{index:02}-{label}.result.json', call)
                except BaseException as error:
                    errors.append(error)
                if errors:
                    combined = errors[0]
                    for error in errors[1:]:
                        combined = native_process.CleanupError(combined, error)
                    raise combined
            primary = operation_error
            if primary is None and child is not None and child.returncode not in (None, 0):
                primary = Refused(f'{label} native exit {child.returncode}')
            native_process._cleanup(finalize, primary)
        self.check()
        return SimpleNamespace(returncode=call['returncode'],
                               stdout=read_file(outputs[0], self.limits.stream_bytes),
                               stderr=read_file(outputs[1], self.limits.stream_bytes))


def _overlap(left, right):
    return left == right or left in right.parents or right in left.parents


def _tool_row(path):
    data = read_file(path, 256 * MIB)
    info = path.stat()
    return {'path': str(path), 'sha256': digest(data), 'bytes': len(data),
            'device': info.st_dev, 'inode': info.st_ino}


def admit_tools(tools, owner, env, run):
    rustc, validator = trusted_file(tools.rustc), trusted_file(tools.validator)
    sysroot = rustc.parent.parent
    version = owner.run([str(rustc), '-vV'], cwd=run, env=env, label='compiler')
    require(version.returncode == 0, 'Pinned compiler identification failed')
    text = version.stdout.decode('utf-8')
    fields = dict(line.split(': ', 1) for line in text.splitlines() if ': ' in line)
    require(fields.get('release') == '1.99.0' and fields.get('host') in (
        'aarch64-apple-darwin', 'x86_64-apple-darwin'), 'Pinned Rust 1.99.0 macOS toolchain required')
    linker = trusted_file(sysroot / 'lib/rustlib' / fields['host'] / 'bin/rust-lld')
    target = absolute(sysroot / 'lib/rustlib/wasm32-unknown-unknown/lib')
    require(target.is_dir() and list(target.glob('libcore-*.rlib')), 'Installed pinned wasm32 core required')
    return rustc, validator, linker, sysroot, version.stdout


def guest_command(rustc, linker, source, sdk, output):
    return [str(rustc), str(source), '--crate-name', 'incant_untrusted_module',
            '--crate-type=cdylib', *rust_guest.FLAGS, '--extern',
            f'incant_module={sdk / "libincant_module.rlib"}', '-L', f'dependency={sdk}',
            '-C', 'codegen-units=1', '-C', f'linker={linker}', '-o', str(output)]


def publish(data, output, cancel):
    """No overwrite or partially visible final module; held output directory."""
    from rust_guest_source import identity
    parent = absolute(output.parent)
    before = parent.stat()
    directory = open_directory(parent)
    stage = '.incant-module-' + os.urandom(16).hex()
    published = False
    own_identity = None
    try:
        require(identity(os.fstat(directory)) == identity(before), 'Output parent changed')
        fd = os.open(stage, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=directory)
        try:
            with os.fdopen(fd, 'wb') as stream:
                own = os.fstat(stream.fileno())
                own_identity = own.st_dev, own.st_ino
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            require(not cancel(), 'Build cancelled before publication')
            current = parent.stat()
            require((current.st_dev, current.st_ino) == (before.st_dev, before.st_ino), 'Output parent replaced')
            os.link(stage, output.name, src_dir_fd=directory, dst_dir_fd=directory, follow_symlinks=False)
            published = True
            os.fsync(directory)
            result = os.stat(output.name, dir_fd=directory, follow_symlinks=False)
            require((result.st_dev, result.st_ino) == own_identity, 'Published module replaced before completion')
            current = parent.stat()
            require((current.st_dev, current.st_ino) == (before.st_dev, before.st_ino),
                    'Output parent replaced after publication')
        except BaseException as error:
            if published:
                def rollback():
                    current = os.stat(output.name, dir_fd=directory, follow_symlinks=False)
                    require((current.st_dev, current.st_ino) == own_identity,
                            'Refusing to delete replaced publication during rollback')
                    os.unlink(output.name, dir_fd=directory)
                native_process._cleanup(rollback, error)
            raise
        finally:
            def remove_stage():
                current = os.stat(stage, dir_fd=directory, follow_symlinks=False)
                require((current.st_dev, current.st_ino) == own_identity, 'Publication staging identity changed')
                os.unlink(stage, dir_fd=directory)
            native_process._cleanup(remove_stage, sys.exception())
    finally:
        os.close(directory)
    return result.st_dev, result.st_ino


def retire_publication(output, expected):
    """Only our own new inode, if success-receipt publication itself fails."""
    directory = open_directory(output.parent)
    try:
        current = os.stat(output.name, dir_fd=directory, follow_symlinks=False)
        require((current.st_dev, current.st_ino) == expected, 'Published module replaced during failure cleanup')
        os.unlink(output.name, dir_fd=directory)
    finally:
        os.close(directory)


def build(request, tools, *, limits=Limits(), cancel=lambda: False):
    """Shared worker entry point. Caller supplies trusted Tools, never project JSON.

    Retain the create-only work directory on every terminal outcome. The success
    receipt and returned result explicitly mark module/log data untrusted.
    """
    MacPolicy.backend()  # Unsupported refuses before source/output/work setup.
    limits.validate()
    require(callable(cancel), 'Cancellation callback required')
    source, output, work = map(absolute, (request.source_root, request.output, request.work))
    relative_entry(request.entry)
    require(output.suffix == '.wasm' and output.parent.is_dir(), 'Existing output parent and .wasm name required')
    require(not output.exists() and not output.is_symlink(), 'Output collision')
    require(not any(_overlap(a, b) for a, b in ((source, work), (source, output), (work, output))),
            'Source, work and publication paths must be disjoint')
    require(not cancel(), 'Build cancelled before setup')
    work.mkdir(mode=0o700)  # Caller cannot adopt an existing partial attempt.
    owner = Owner(work, limits, cancel)
    receipt = {'version': 1, 'status': 'failed', 'artifact_trust': 'untrusted-data',
               'backend': 'macos-sandbox-exec-deprecated', 'source': [], 'calls': owner.calls}
    published = None
    try:
        run = work / 'run'
        run.mkdir(mode=0o700)
        for name in ('home', 'tmp'):
            (run / name).mkdir(mode=0o700)
        rustc = trusted_file(tools.rustc)
        require(not any(_overlap(path, source) or _overlap(path, work)
                        for path in (rustc.parent.parent, absolute(tools.validator))),
                'Tool authority overlaps request storage')
        env = clean_environment(run, rustc)
        rustc, validator, linker, sysroot, version = admit_tools(tools, owner, env, run)
        require(not any(_overlap(path, source) or _overlap(path, work)
                        for path in (sysroot, validator)), 'Tool authority overlaps request storage')
        tool_rows = [_tool_row(path) for path in (rustc, linker, validator)]
        receipt['tools'] = tool_rows
        receipt['compiler_version'] = version.decode('utf-8')
        files = capture(source, request.entry)
        receipt['source'] = materialize(files, work / 'source')
        owner.check()

        def execute(command, **_):
            result = owner.run(command, cwd=run, env=env, label='sdk')
            return SimpleNamespace(returncode=result.returncode, stderr=result.stderr.decode('utf-8', errors='replace'))

        sdk = rust_guest.prepare_sdk(rustc, work / 'sdk', env, execute=execute, compiler=version)
        receipt['sdk_sources'] = rust_guest.provenance()
        receipt['service_sources'] = {name: digest(read_file(Path(__file__).parent / name, MIB))
                                     for name in ('rust_guest_build.py', 'rust_guest_source.py',
                                                  'rust_guest_sandbox.py', 'native_process.py')}
        receipt['sdk_libraries'] = [_tool_row_data(path) for path in sorted(sdk.glob('*.rlib'))]
        policy = MacPolicy(rustc, linker, validator, sysroot, sdk, work / 'source', run)
        policy_text = policy.text()
        policy_path = work / 'policy.sb'
        with policy_path.open('x') as stream:
            stream.write(policy_text)
        policy_path.chmod(0o400)
        receipt['policy_sha256'] = digest(policy_text.encode())
        result = owner.run(policy.command(policy_path, [str(rustc), '--print', 'cfg',
                           '--target=wasm32-unknown-unknown']), cwd=run, env=env, label='sandbox-preflight')
        if result.returncode != 0 or b'target_arch="wasm32"' not in result.stdout.splitlines():
            raise Unsupported('Required compiler sandbox initialization failed; no unrestricted fallback')
        raw = run / 'module.wasm'
        result = owner.run(policy.command(policy_path, guest_command(
            rustc, linker, work / 'source' / request.entry, sdk, raw)),
            cwd=run, env=env, label='untrusted-compile')
        require(result.returncode == 0, 'Sandboxed Rust compiler refused; diagnostic streams are untrusted')
        data = read_file(raw, limits.raw_bytes)
        require(data[:8] == b'\0asm\x01\0\0\0', 'Compiler output is not raw Wasm v1')
        prepared = run / 'validated.wasm'
        result = owner.run(policy.command(policy_path, [str(validator), str(raw), str(prepared)]),
                           cwd=run, env=env, label='normal-validator')
        require(result.returncode == 0, 'Normal Wasm cook validator refused')
        validated = read_file(prepared, limits.prepared_bytes)
        require(validated[:8] == b'\0asm\x01\0\0\0', 'Validator produced no prepared Wasm')
        require(read_file(raw, limits.raw_bytes) == data, 'Raw module changed during validation')
        require([_tool_row(Path(row['path'])) for row in tool_rows] == tool_rows, 'Trusted tools changed')
        require(all(call['group_absent'] and call['returncode'] == 0 for call in owner.calls),
                'Build operation or owned cleanup incomplete')
        owner.check()
        receipt.update(raw_sha256=digest(data), prepared_sha256=digest(validated), raw_bytes=len(data),
                       output=str(output), elapsed_seconds=time.monotonic() - owner.started,
                       limits=asdict(limits), peak_scratch_bytes=owner.peak_scratch,
                       sampled_peak_rss_bytes=owner.peak_rss, diagnostic_bytes=owner.log_bytes)
        published = publish(data, output, cancel)
        receipt['status'] = 'complete'
        write_json(work / 'result.json', receipt)
        return receipt
    except BaseException as error:
        receipt['status'] = 'failed'
        receipt.update(error_type=type(error).__name__, error=str(error),
                       elapsed_seconds=time.monotonic() - owner.started,
                       peak_scratch_bytes=owner.peak_scratch, sampled_peak_rss_bytes=owner.peak_rss,
                       diagnostic_bytes=owner.log_bytes)
        def finalize_failure():
            cleanup_error = None
            if published is not None:
                try:
                    retire_publication(output, published)
                except BaseException as failure:
                    cleanup_error = failure
                    receipt['publication_cleanup_error'] = repr(failure)
            try:
                write_json(work / 'failure.json', receipt)
            except BaseException as failure:
                if cleanup_error is not None:
                    raise native_process.CleanupError(cleanup_error, failure) from cleanup_error
                raise
            if cleanup_error is not None:
                raise cleanup_error
        native_process._cleanup(finalize_failure, error)
        raise


def _tool_row_data(path):
    data = read_file(path, 64 * MIB)
    return {'path': str(path), 'bytes': len(data), 'sha256': digest(data)}


def main(argv=None):
    import argparse
    import signal
    import threading
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--untrusted', action='store_true', required=True)
    parser.add_argument('--source-root', type=Path, required=True)
    parser.add_argument('--entry', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--work-dir', type=Path, required=True, help='New private directory, retained on failure')
    parser.add_argument('--rustc', type=Path, required=True, help='Trusted pinned 1.99.0 toolchain executable')
    parser.add_argument('--validator', type=Path, required=True, help='Trusted engine incant_wasm_cook executable')
    args = parser.parse_args(argv)
    cancelled = threading.Event()
    old_handlers = {number: signal.signal(number, lambda *_: cancelled.set())
                    for number in (signal.SIGINT, signal.SIGTERM)}
    try:
        result = build(Request(args.source_root, args.entry, args.output, args.work_dir),
                       Tools(args.rustc, args.validator), cancel=cancelled.is_set)
    except (Refused, OSError, UnicodeError) as error:
        parser.exit(2, f'Untrusted build refused: {error}\n')
    finally:
        for number, handler in old_handlers.items():
            signal.signal(number, handler)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
