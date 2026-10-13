#!/usr/bin/env python3
"""Real retained CPU worker processes using publicly authored projects.

Supply matching freshly built binaries. This verifies protocol/lifecycle and
native behavior, not editor GPU rendering, a shipping profile or device gates.
"""
import argparse
from contextlib import contextmanager
import json
from pathlib import Path
import queue
import runpy
import shutil
import subprocess
import sys
import threading

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from native_package import require, sha256, write_json
from native_process import run
fixture = runpy.run_path(str(ROOT / 'tools/probes/physics-mode-selection.py'))
Rpc, SCRIPT, ENTITY, SOURCE = (fixture[k] for k in ('Rpc', 'SCRIPT', 'ENTITY', 'SOURCE'))


class Child:
    def __init__(self, command, output):
        self.output = output
        output.mkdir()
        self.stderr = (output / 'stderr').open('wb')
        self.stdout = (output / 'stdout').open('wb')
        self.requests = (output / 'stdin').open('wb')
        self.lines = queue.Queue()
        self.process = subprocess.Popen(list(map(str, command)), cwd=ROOT,
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=self.stderr)
        self.command = list(map(str, command))
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _read(self):
        try:
            for line in self.process.stdout:
                self.stdout.write(line)
                self.stdout.flush()
                self.lines.put(line)
        finally:
            self.lines.put(None)

    def receive(self):
        line = self.lines.get(timeout=60)
        require(line is not None, 'Worker closed before response')
        return json.loads(line)

    def send(self, value):
        data = value if isinstance(value, bytes) else (json.dumps(value) + '\n').encode()
        self.requests.write(data)
        self.requests.flush()
        self.process.stdin.write(data)
        self.process.stdin.flush()

    def exchange(self, value):
        self.send(value)
        return self.receive()

    def finish(self, expected):
        if not self.process.stdin.closed:
            self.process.stdin.close()
        require(self.process.wait(timeout=30) == expected, 'Unexpected worker exit')
        self.reader.join(timeout=5)
        require(not self.reader.is_alive(), 'Worker output did not close')
        require(self.lines.get(timeout=5) is None, 'Unexpected extra worker response')

    def close(self):
        # Failure, timeout and interruption still reap this exact owned child.
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        self.reader.join(timeout=5)
        for stream in (self.process.stdin, self.process.stdout, self.stderr,
                       self.stdout, self.requests):
            stream.close()
        write_json(self.output / 'process.json', {'command': self.command,
                   'exit_code': self.process.returncode, 'reaped': self.process.poll() is not None})


@contextmanager
def child(command, output):
    instance = Child(command, output)
    try:
        yield instance
    finally:
        instance.close()


def request(number, op, params=None, version=1):
    value = {'version': version, 'id': number, 'request': {'op': op}}
    if params is not None:
        value['request']['params'] = params
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--deterministic', type=Path, required=True)
    parser.add_argument('--fast', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binaries = {mode: getattr(args, mode).resolve(strict=True) for mode in ('deterministic', 'fast')}
    commands, calls, cases = [], [], []

    def execute(command, expected=0):
        result = run(list(map(str, command)), cwd=ROOT, timeout=120)
        record = {'argv': list(map(str, command)), 'exit_code': result.returncode,
                  'stdout': result.stdout, 'stderr': result.stderr}
        commands.append(record)
        write_json(out / 'commands.json', commands)
        require(result.returncode == expected, record)
        return result

    def call(rpc, method, params=None):
        row = {'method': method, 'params': params}
        calls.append(row)
        try:
            row['result'] = rpc.call(method, params)
            return row['result']
        finally:
            write_json(out / 'rpc.json', calls)

    for mode, binary in binaries.items():
        folder = out / mode
        folder.mkdir()
        project = folder / 'authoring/game.json'
        project.parent.mkdir()
        execute([binary, 'init', project, '--entities', 0])
        rpc = Rpc(binary, project, project.with_suffix('.journal.jsonl'))
        try:
            before = call(rpc, 'project.read')
            scene = next(iter(before['project']['scenes']))
            call(rpc, 'command.execute', {'expected_revision': before['revision'],
                 'description': 'Author retained worker proof', 'commands': [
                     {'op': 'set_physics_mode', 'mode': mode},
                     {'op': 'write_script', 'script_id': SCRIPT,
                      'expected_source': {'state': 'missing'}, 'source': SOURCE},
                     {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Mover', 'path': 'move.ts'}},
                     {'op': 'create_entity', 'scene_id': scene, 'entity': {
                         'id': ENTITY, 'name': 'Mover', 'parent': None, 'provenance': None,
                         'components': {'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                                        'Script': {'source': SCRIPT, 'props': {}}}}}]})
            call(rpc, 'project.save')
        finally:
            rpc.close()
        digest = sha256(project.read_bytes())
        observe = folder / 'observe.json'
        write_json(observe, {'format': 'incant-native-observation', 'version': 1,
                            'entities': [ENTITY], 'checks': []})
        baseline = json.loads(execute([binary, 'native-play', project, '--ticks', 12,
                                       '--observe', observe]).stdout)
        require(baseline['passed'], 'Native baseline failed')
        command = [binary, 'native-worker', project, '--project-sha256', digest]
        for name, argv, diagnostic in [
            ('changed', [*command[:-1], '0' * 64], 'project changed before worker startup'),
            ('wrong-mode', [binaries['fast' if mode == 'deterministic' else 'deterministic'], *command[1:]], 'physics mode'),
        ]:
            result = execute(argv, 1)
            require(not result.stdout and diagnostic in result.stderr, 'Incorrect startup refusal')
            cases.append(mode + ':' + name)

        def ready(worker):
            value = worker.receive()
            require(value['event'] == 'ready' and value['version'] == 1
                    and value['physics_mode'] == mode and value['project_sha256'] == digest
                    and value['state']['tick'] == 0 and value['state']['script_source_parsing']
                    and value['execution'] == 'native_cpu', 'Wrong ready identity')
            return value

        for name, payload, exit_code in [('eof', b'', 0), ('truncated', b'{"version":1', 1),
                                        ('oversized', b' ' * (256 * 1024 + 1), 1)]:
            with child(command, folder / name) as worker:
                ready(worker)
                if payload:
                    worker.send(payload)
                worker.finish(exit_code)
            if exit_code:
                require(('truncated request' if name == 'truncated' else 'request exceeds 256 KiB')
                        in (folder / name / 'stderr').read_text(), 'Wrong framing refusal')
            cases.append(mode + ':' + name)

        with child(command, folder / 'retained') as worker:
            ready(worker)
            # Delete only the disposable proof project, including source/cache.
            # The live worker must retain the cooked world, never reload files.
            shutil.rmtree(project.parent)
            require(not project.parent.exists(), 'Authoring was not removed')
            for value, code in [
                (request(1, 'step', {}, version=2), 'version'),
                (request(1, 'command.execute', {}), 'invalid_request'),
                ({'version': 1, 'id': 1, 'request': {'op': 'stop', 'unknown': True}}, 'invalid_request'),
                (b'{malformed}\n', 'invalid_request'),
                (request(1, 'step', {'observe': {'entities': ['bad-id']}}), 'native'),
            ]:
                require(worker.exchange(value)['error']['code'] == code, 'Incorrect request refusal')
            state = worker.exchange(request(2, 'observe', {}))['result']
            require(state['tick'] == 0 and state['entities'] == {}, 'Refusal advanced or exposed state')
            for tick in range(1, 13):
                state = worker.exchange(request(tick + 2, 'step', {
                    'observe': {'entities': [ENTITY], 'include_input': True, 'include_logs': True}}))['result']
                require(state['tick'] == tick and not state['gpu_submitted'], 'Wrong native tick')
                if tick == 1:
                    require(worker.exchange(request(3, 'step', {}))['error']['code'] == 'sequence',
                            'Duplicate step was accepted')
            entity = state['entities'][ENTITY]
            expected = baseline['observations']['entities'][ENTITY]
            require(entity['transform'] == {k: expected[k] for k in ('translation', 'rotation', 'scale')}
                    and entity['world_transform'] == expected['world_transform']
                    and state['frames'] == baseline['frames'] and state['logs'] == baseline['logs'],
                    'Retained state differs from ordinary native Play')
            worker.send(request(15, 'stop'))
            require(worker.receive()['result'] == {'playing': False}, 'Stop was not acknowledged')
            worker.finish(0)
            write_json(folder / 'state.json', state)
        cases.append(mode + ':retained-source-deleted')
    write_json(out / 'result.json', {'passed': True, 'cases': cases,
               'binary_sha256': {k: sha256(p.read_bytes()) for k, p in binaries.items()},
               'limits': ['Native CPU protocol only, no editor GPU/window or shipping/device claim.',
                          'Movement fixture has no active physics bodies. PCM is not exposed by this worker.']})
    print(json.dumps({'passed': True, 'cases': len(cases), 'output': str(out)}))


if __name__ == '__main__':
    main()
