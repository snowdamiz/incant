"""Fixed editor host adapter. Rust owns every child; no project mutation.

The private mailbox is trusted engine IPC, never an agent command capability.
Rust independently admits exact recipes/environment and owns each process group.
"""
import json
import os
from pathlib import Path
import sys
import time
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_guest_build as service
import rust_guest_source as source


def private_write(path, value):
    temporary = path.with_name(path.name + '.tmp')
    service.write_json(temporary, value)
    os.rename(temporary, path)


class ProxyOwner(service.Owner):
    def __init__(self, work, limits, cancel, job):
        super().__init__(work, limits, cancel)
        self.job = job

    def run(self, command, *, cwd, env, label):
        self.check()
        index = len(self.calls)
        service.require(index < 16, 'Fixed child count')
        directory = self.job / 'calls' / f'{index:03}'
        directory.mkdir(mode=0o700)
        private_write(directory / 'request.json', {'argv': list(map(str, command)),
                      'cwd': str(cwd), 'env': env, 'stdout': str(directory / 'stdout'),
                      'stderr': str(directory / 'stderr')})
        started = time.monotonic()
        response = directory / 'response.json'
        try:
            while not response.exists():
                self.check()
                service.require(time.monotonic() - started < self.limits.child_seconds,
                                'Delegated child deadline')
                pid_receipt = directory / 'started.json'
                if pid_receipt.exists():
                    pid = json.loads(service.read_file(pid_receipt, 4096))['pid']
                    rows = service.group_sample(pid) + service.group_sample(os.getpgrp())
                    self.peak_rss = max(self.peak_rss, sum(row['rss'] for row in rows))
                    service.require(self.peak_rss <= self.limits.rss_bytes, 'Build sampled RSS cap')
                for name in ('stdout', 'stderr'):
                    path = directory / name
                    if path.exists():
                        service.require(path.stat().st_size <= self.limits.stream_bytes,
                                        'Delegated diagnostic stream bound')
                time.sleep(.01)
        except BaseException:
            (directory / 'cancel').touch(exist_ok=False)
            raise
        row = json.loads(service.read_file(response, 4096))
        service.require(set(row) == {'pid', 'returncode', 'group_absent'}
                        and row['group_absent'] is True, 'Actual Rust group closure required')
        data = [service.read_file(directory / name, self.limits.stream_bytes)
                for name in ('stdout', 'stderr')]
        self.log_bytes += sum(map(len, data))
        service.require(self.log_bytes <= self.limits.log_bytes, 'Combined delegated diagnostic cap')
        self.calls.append(dict(row, label=label, argv=list(map(str, command)),
                               stream_bytes=list(map(len, data)), elapsed_seconds=time.monotonic()-started))
        self.check()
        return SimpleNamespace(returncode=row['returncode'], stdout=data[0], stderr=data[1])


def package_view(root, package, entry):
    source.relative_entry(entry)
    # Package has the same safe component grammar as Rust source, without an extension.
    source.relative_entry(package + '/entry.rs')
    files = source.capture(root / package, entry)
    return {'entry': entry, 'files': [{'path': name, 'text': data.decode('utf-8')}
                                     for name, data in sorted(files.items())]}


def run(job):
    request = json.loads(service.read_file(job / 'request.json', 2 * service.MIB))
    service.require(request['mode'] in ('inspect', 'build'), 'Fixed worker operation')
    if request['mode'] == 'inspect':
        service.require(set(request) == {'mode', 'root', 'package', 'entry', 'rustc', 'validator'},
                        'Fixed inspection protocol')
        return package_view(Path(request['root']), request['package'], request['entry'])
    service.require(set(request) == {'mode', 'source', 'rustc', 'validator'}, 'Fixed build protocol')
    package = request['source']
    service.require(set(package) == {'entry', 'files'} and len(package['files']) <= 32, 'Fixed source protocol')
    files = {}
    for row in package['files']:
        service.require(set(row) == {'path', 'text'} and row['path'] not in files, 'Unique source record')
        source.relative_entry(row['path'])
        files[row['path']] = row['text'].encode('utf-8')
        service.require(len(files[row['path']]) <= source.MAX_FILE
                        and sum(map(len, files.values())) <= source.MAX_TOTAL, 'Immutable source byte bounds')
    inputs = job / 'input'
    source.materialize(files, inputs)
    # Run normal capture again so the same file/total/depth/entry rules admit the snapshot.
    source.capture(inputs, package['entry'])
    return service.build(service.Request(inputs, package['entry'], job / 'module.wasm', job / 'work'),
        service.Tools(Path(request['rustc']), Path(request['validator'])),
        owner_factory=lambda work, limits, cancel: ProxyOwner(work, limits, cancel, job))


def main():
    service.require(len(sys.argv) == 2, 'One host-selected private job directory required')
    job = Path(sys.argv[1])
    try:
        reply = {'status': 'complete', 'data': run(job)}
    except BaseException as error:
        diagnostics = str(error)
        # Compiler diagnostics are data. The project cannot choose the files read here.
        logs = sorted((job / 'calls').glob('*/stderr'))
        if logs:
            diagnostics += '\n' + service.read_file(logs[-1], 4 * service.MIB).decode('utf-8', errors='replace')
        reply = {'status': 'failed', 'untrusted_diagnostics': bounded_diagnostics(diagnostics)}
        private_write(job / 'reply.json', reply)
        raise SystemExit(1)
    private_write(job / 'reply.json', reply)


def bounded_diagnostics(text):
    # Truncating UTF-8 may split a code point; replacement can exceed the byte cap.
    return text.encode('utf-8')[:65536].decode('utf-8', errors='ignore')


if __name__ == '__main__':
    main()
