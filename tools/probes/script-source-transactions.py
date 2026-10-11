#!/usr/bin/env python3
"""Actual public RPC source edits, native Play, Undo/Redo and fresh recovery.

All .ts contents are authored through incant_cmd; the only fixture creation is
`init` in a fresh directory. This is CPU functional evidence, not a device gate.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

SCRIPT = '00000000000000000000000001'
ENTITY = '00000000000000000000000002'

def source(speed):
    return ("import type { NativeBehavior } from '@incant/runtime/native';\n"
            "export default { update({ chunks, dt }) {\n"
            "  for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++)\n"
            f"    chunk.transforms[i * chunk.transformStride] += {speed} * dt;\n"
            "} } satisfies NativeBehavior;\n")

def require(condition, message):
    if not condition:
        raise ValueError(message)

class Rpc:
    def __init__(self, binary, project, journal):
        self.process = subprocess.Popen([str(binary), 'rpc', str(project), '--journal', str(journal)],
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                        text=True, encoding='utf-8')
        self.requests = 0
    def call(self, method, params=None):
        self.requests += 1
        request = {'id': self.requests, 'method': method, 'params': params or {}}
        self.process.stdin.write(json.dumps(request) + '\n')
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise ValueError(f'RPC stopped: {self.process.stderr.read()}')
        response = json.loads(line)
        require('error' not in response, f'{method}: {response}')
        return response['result']
    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=30)
        require(self.process.returncode == 0, self.process.stderr.read())
        self.process.stdout.close()
        self.process.stderr.close()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    project, journal = output / 'project.json', output / 'project.journal.jsonl'
    subprocess.run([str(binary), 'init', str(project), '--entities', '0'], check=True, capture_output=True)
    genesis = project.read_bytes()
    document = json.loads(genesis)
    scene = next(iter(document['scenes']))
    rate = document['settings']['tick_rate']
    first, second = source(6), source(12)
    rpc = Rpc(binary, project, journal)
    played = []
    def play(expected):
        before = rpc.call('project.read')
        result = rpc.call('play.run', {'seconds': 20 / rate})
        require(result['passed'] and result['state']['tick'] == 20, 'native run failed')
        require(not result['state']['gpu_submitted'], 'CPU probe submitted GPU work')
        require(result['state']['script_source_parsing'], 'authored startup compiler was not used')
        value = rpc.call('play.state_query', {'path': f'/entities/{ENTITY}/transform/translation'})
        require(abs(value['untrusted_runtime_data'][0] - expected) < 1e-10, 'wrong authored motion')
        require(rpc.call('project.read') == before, 'Play changed authoring')
        played.append(value['untrusted_runtime_data'][0])
    created = rpc.call('command.execute', {'expected_revision': 0, 'description': 'Author attached movement', 'commands': [
        {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': first},
        {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Movement', 'path': 'move.ts'}},
        {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': ENTITY, 'name': 'Mover', 'parent': None, 'provenance': None,
         'components': {'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}, 'Script': {'source': SCRIPT, 'props': {}}}}}
    ]})
    require(created['revision'] == 1, 'creation was not one transaction')
    read = rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']
    require(read['source'] == first and read['matches_history'], 'created source not read back')
    play(6 * 20 / rate)
    rpc.call('script.write', {'script_id': SCRIPT, 'source': second, 'expected_source': read['version'], 'expected_revision': 1, 'description': 'Double movement'})
    play(12 * 20 / rate)
    rpc.call('history.undo')
    play(6 * 20 / rate)
    rpc.call('history.redo')
    recovered = rpc.call('project.read')
    requests = rpc.requests
    rpc.close()
    rpc = Rpc(binary, project, journal)
    require(rpc.call('project.read') == recovered, 'fresh journal recovery differs')
    require(rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == second, 'recovered source differs')
    play(12 * 20 / rate)
    rpc.call('history.undo')
    rpc.call('history.undo')
    require(not (output / 'move.ts').exists(), 'Undo of creation did not remove matching source')
    require(rpc.call('project.read')['project'] == document, 'Undo of creation did not restore genesis')
    rpc.call('history.redo')
    play(6 * 20 / rate)
    requests += rpc.requests
    rpc.close()
    require(project.read_bytes() == genesis, 'checkpoint changed without save')
    evidence = {'passed': True, 'processes': 2, 'rpc_requests': requests, 'ticks_per_play': 20,
                'translations_x': played, 'content_authored_only_through_command_bus': True,
                'registration_content_attachment_one_transaction': True, 'undo_redo_and_recovery': True,
                'authoring_unchanged_by_play': True, 'gpu_submitted': False, 'reference_phone_gate': False,
                'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    (output / 'result.json').write_text(json.dumps(evidence, indent=2) + '\n')
    print(json.dumps(evidence))

if __name__ == '__main__':
    main()
