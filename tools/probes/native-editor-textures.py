#!/usr/bin/env python3
"""Actual editor-window import/watch/Undo/Redo; no pixel or performance claim.

Build a genuine custom-protocol,play-probe editor and matching headless first.
The runner creates one fresh fixture; project text is authored through RPC/Cmd.
Only its owned PNG/glTF sources are changed by the app's opt-in asset probe.
"""
import argparse
import base64
from contextlib import contextmanager
import hashlib
import json
import math
from pathlib import Path
import queue
import signal
import struct
import subprocess
import sys
import threading
import time
import zlib

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process

ENTITY = '00000000000000000000000002'
SCRIPT = '00000000000000000000000001'
CAMERA = '00000000000000000000000003'
STAGES = ['manual_import_red', 'watched_blue', 'undo_red', 'redo_blue',
          'malformed_source_retained_blue', 'source_recovered_blue']
MODEL_STAGES = ['manual_import_original', 'watched_replacement', 'undo_original', 'redo_replacement',
                'malformed_source_retained_replacement', 'source_recovered_replacement']
SOURCE = ("import type { NativeBehavior } from '@incant/runtime/native';\n"
          "let updates = 0;\n"
          "export default { update({ instances, chunks }) {\n"
          "  updates += 1;\n"
          "  for (const chunk of chunks) for (let i=0;i<chunk.ids.length;i++) {\n"
          "    if (instances.some(instance => instance.entity === chunk.ids[i]))\n"
          "      chunk.transforms[i*chunk.transformStride]+=2*updates;\n"
          "  }\n"
          "} } satisfies NativeBehavior;\n")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def failure_record(error):
    """Preserve timeout captures after the managed owner finishes cleanup."""
    record = {'error':repr(error)}
    if isinstance(error, subprocess.TimeoutExpired):
        record['reaped'] = True
        record['timeout_capture_scope'] = getattr(error, 'capture_scope', 'managed-run-final-output')
        for name, value in (('stdout', error.output), ('stderr', error.stderr)):
            record[name + '_capture_available'] = value is not None
            if value is not None:
                raw = value.encode('utf-8') if isinstance(value, str) else value
                record[name] = raw.decode('utf-8', errors='replace')
                record[name + '_base64'] = base64.b64encode(raw).decode('ascii')
                record[name + '_bytes'] = len(raw)
                record[name + '_sha256'] = hashlib.sha256(raw).hexdigest()
    return record


def rpc_process(argv, data, timeout, cancel):
    """Bound protocol IO around the existing managed Interactive owner.

    The public run helper supplies EOF stdin, so RPC uses Interactive instead.
    Pipe threads only perform IO; the main thread owns cancellation and cleanup.
    """
    require(type(timeout) in (int, float) and math.isfinite(timeout) and timeout > 0, 'Finite positive RPC timeout required')
    require(isinstance(data, str) and len(data.encode('utf-8')) <= 256*1024, 'Bounded UTF-8 RPC input required')
    require(callable(cancel), 'RPC cancellation checkpoint required')
    cancel()
    deadline = time.monotonic() + timeout
    events = queue.Queue()
    captures = {'stdout':bytearray(), 'stderr':bytearray()}
    threads = []
    with native_process.Interactive(argv, cwd=ROOT) as child:
        def read(name, stream):
            try:
                while chunk := stream.read1(65536):
                    events.put((name, chunk))
            except BaseException as error:
                events.put(('error', error))
            finally:
                events.put((name, None))
        def write():
            try:
                child.stdin.write(data.encode('utf-8'))
                child.stdin.flush()
                child.stdin.close()
            except BaseException as error:
                events.put(('error', error))
        try:
            for name in captures:
                thread = threading.Thread(target=read, args=(name, getattr(child, name)), daemon=True)
                threads.append(thread)
                thread.start()
            thread = threading.Thread(target=write, daemon=True)
            threads.append(thread)
            thread.start()
            closed = set()
            while len(closed) != 2:
                cancel()
                if time.monotonic() >= deadline:
                    raise subprocess.TimeoutExpired(argv, timeout)
                try:
                    name, chunk = events.get(timeout=max(0.001, min(0.05, deadline-time.monotonic())))
                except queue.Empty:
                    continue
                if name == 'error':
                    raise chunk
                if chunk is None:
                    closed.add(name)
                else:
                    captures[name].extend(chunk)
                    require(sum(map(len, captures.values())) <= 4*1024*1024, 'RPC output exceeds 4 MiB')
            cancel()
            # Both protocol streams closed; only a short bounded leader-exit tail
            # remains. wait() keeps group identity pinned and always reaps it.
            returncode = child.wait(timeout=min(1, max(0.001, deadline-time.monotonic())))
        except subprocess.TimeoutExpired as error:
            error.output = bytes(captures['stdout'])
            error.stderr = bytes(captures['stderr'])
            error.capture_scope = 'rpc-received-prefix-before-cleanup'
            raise
        finally:
            child.close()
            for thread in threads:
                if thread.ident is not None:
                    thread.join()
    return subprocess.CompletedProcess(argv, returncode, *(bytes(captures[n]).decode('utf-8') for n in ('stdout','stderr')))


def png(rgba):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 1, 1, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(bytes([0, *rgba]))) + chunk(b'IEND', b''))


def model(replacement=False):
    """Self-contained static glTF: stable scene/bindings, different vertices/material."""
    points = ([[-0.3, -0.7, 0], [0.9, -0.7, 0], [0.3, 0.3, 0]] if replacement else
              [[-0.7, -0.5, 0], [0.7, -0.5, 0], [0, 0.7, 0]])
    data = struct.pack('<9f', *(value for point in points for value in point))
    color = [0, 0, 1] if replacement else [1, 0, 0]
    document = {
        'asset': {'version':'2.0'},
        'buffers': [{'byteLength':len(data), 'uri':'data:application/octet-stream;base64,' + base64.b64encode(data).decode('ascii')}],
        'bufferViews': [{'buffer':0, 'byteLength':len(data)}],
        'accessors': [{'bufferView':0, 'componentType':5126, 'count':3, 'type':'VEC3',
                       'min':[min(p[a] for p in points) for a in range(3)],
                       'max':[max(p[a] for p in points) for a in range(3)]}],
        'materials': [{'pbrMetallicRoughness':{'baseColorFactor':color+[1], 'metallicFactor':0, 'roughnessFactor':0.8},
                       'emissiveFactor':color, 'doubleSided':True}],
        'meshes':[{'primitives':[{'attributes':{'POSITION':0}, 'material':0}]}],
        'nodes':[{'mesh':0}], 'scenes':[{'nodes':[0]}], 'scene':0,
    }
    return (json.dumps(document, separators=(',', ':')) + '\n').encode('utf-8')


def fixture(kind):
    require(kind in ('texture', 'model'), 'Known asset fixture kind required')
    if kind == 'texture':
        return 'refresh.png', 'refresh-blue.png', png([255, 0, 0, 255]), png([0, 0, 255, 255])
    return 'refresh.gltf', 'refresh-replacement.gltf', model(), model(True)


@contextmanager
def execution_deadline(seconds):
    """Latch cancellation; native_process closes owned groups before propagating.

    The execution deadline is cooperative. Cleanup and blocked filesystem work
    may finish later; it is not a hard wall-clock process-tree guarantee.
    """
    end = time.monotonic() + seconds
    cancelled = []
    previous = {}

    def latch(signum, _):
        if not cancelled:
            cancelled.append(signum)

    def check():
        if cancelled:
            raise RuntimeError(f'Cancelled by signal {cancelled[0]}')
        if time.monotonic() >= end:
            raise TimeoutError('Native-window asset execution deadline')

    for signum in (signal.SIGTERM, signal.SIGINT):
        previous[signum] = signal.signal(signum, latch)
    try:
        yield check, lambda: max(0.001, end - time.monotonic())
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)


def validate_proof(proof, kind='texture'):
    require(kind in ('texture', 'model'), 'Known proof asset kind required')
    require(proof.get('asset_kind', 'texture') == kind, 'Matching asset-kind proof required')
    require(proof.get('passed') is True and proof.get('platform') == 'macos', 'Actual Mac app proof required')
    for field in ('world_identity_unchanged', 'module_counter_continued', 'actual_window_presentations',
                  'manual_import_execute_history_handlers', 'real_source_watcher', 'undo_not_overwritten',
                  'source_recovery', 'stale_generation_refused', 'stop_releases_world'):
        require(proof.get(field) is True, 'Missing observable control: ' + field)
    for field in ('pixel_capture', 'appearance_review', 'performance_or_phase_gate'):
        require(proof.get(field) is False, 'Unexpected scope claim: ' + field)
    require('backend: Metal' in proof.get('adapter', ''), 'Actual Metal adapter required')
    require(type(proof.get('play_generation')) is int and proof['play_generation'] > 0, 'Play generation identity')
    require(type(proof.get('paused_tick')) is int and type(proof.get('final_tick')) is int
            and proof['final_tick'] == proof['paused_tick'] + 1, 'One continuing VM tick')
    counter = proof.get('vm_counter', {})
    tick = proof['paused_tick']
    require(counter.get('before_translation') == tick*(tick+1)
            and counter.get('expected_delta') == 2*(tick+1)
            and counter.get('observed_delta') == counter['expected_delta'], 'Observable module-state continuation')
    stages = proof.get('stages', [])
    require([s.get('stage') for s in stages] == (STAGES if kind == 'texture' else MODEL_STAGES), 'Exact user workflow stages')
    require(all(s.get('recipe_matches') is True and s.get('tick') == proof['paused_tick'] for s in stages),
            'Accepted paused catalog/world')
    frames = [s.get('frames') for s in stages]
    require(all(type(f) is int for f in frames) and all(a < b for a, b in zip(frames, frames[1:])),
            'A later actual presentation per catalog stage')
    hashes = [s.get('accepted_fingerprint') for s in stages]
    require(all(isinstance(h, str) and len(h) == 64 and all(c in '0123456789abcdef' for c in h) for h in hashes),
            'Cooked recipe identities')
    require(hashes[0] == hashes[2] and hashes[1] == hashes[3] == hashes[4] == hashes[5]
            and hashes[0] != hashes[1], 'Original/replacement/Undo/Redo catalog selection')
    revisions = [s.get('history', {}).get('revision') for s in stages]
    require(all(type(r) is int for r in revisions), 'Committed revision identities')
    first = revisions[0]
    require(revisions == [first, first+1, first+2, first+3, first+3, first+3],
            'Source/history commits and refusal preservation')
    require(proof.get('malformed_source_refusal', {}).get('code') == 'asset.import', 'Structured import refusal')
    return proof


def author_commands(scene):
    transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
    def entity(identity, name, components):
        return {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': identity, 'name': name,
                'parent': None, 'provenance': None, 'components': components}}
    return [
        {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': SOURCE},
        {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Texture workflow VM sentinel', 'path': 'move.ts'}},
        entity(ENTITY, 'Mover', {'Transform': transform, 'Script': {'source': SCRIPT, 'props': {}}}),
        entity(CAMERA, 'Camera', {'Transform': transform, 'Camera': {'projection': {'kind': 'perspective'},
               'fov_degrees': 60, 'near': 0.1, 'far': 100}}),
    ]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--editor', type=Path, required=True)
    parser.add_argument('--headless', type=Path, required=True)
    parser.add_argument('--asset-kind', choices=('texture', 'model'), default='texture')
    args = parser.parse_args(argv)
    source_name, replacement_name, original_bytes, replacement_bytes = fixture(args.asset_kind)
    output = args.output.resolve()
    editor, headless = args.editor.resolve(strict=True), args.headless.resolve(strict=True)
    output.mkdir(parents=True, exist_ok=False)
    project, journal = output/'project.json', output/'project.journal.jsonl'
    ledger = []
    result = {'passed': False, 'asset_kind':args.asset_kind, 'editor_sha256': digest(editor), 'headless_sha256': digest(headless),
              'pixel_capture': False, 'appearance_review': False, 'performance_or_phase_gate': False}
    def write(name, value):
        (output/name).write_text(json.dumps(value, indent=2) + '\n')
    try:
        with execution_deadline(180) as (check, remaining):
            def run(command, data=None, bound=30):
                check()
                entry = {'argv':list(map(str, command)), 'input':data, 'timeout_seconds':min(bound, remaining())}
                ledger.append(entry)
                try:
                    native = (native_process.run(entry['argv'], cwd=ROOT,
                        timeout=entry['timeout_seconds'], cancel=check) if data is None else
                        rpc_process(entry['argv'], data, entry['timeout_seconds'], check))
                    entry.update(exit_code=native.returncode, stdout=native.stdout, stderr=native.stderr, reaped=True)
                except BaseException as error:
                    entry.update(failure_record(error))
                    raise
                finally:
                    write('processes.json', ledger)
                require(native.returncode == 0, f'Owned fixture/app failed: {entry}')
                return native.stdout
            requests = 0
            def rpc(method, params=None):
                nonlocal requests
                requests += 1
                request = {'id':requests, 'method':method, 'params':params or {}}
                raw = run([headless, 'rpc', project, '--journal', journal], json.dumps(request) + '\n')
                rows = [json.loads(line) for line in raw.splitlines()]
                require(len(rows) == 1 and rows[0].get('id') == requests, 'RPC response identity')
                require('error' not in rows[0], f'{method}: {rows[0].get("error")}')
                return rows[0]['result']
            run([headless, 'init', project, '--entities', '0'])
            initial = rpc('project.read')
            require(len(initial['project']['scenes']) == 1, 'One fresh fixture scene required')
            scene = next(iter(initial['project']['scenes']))
            authored = rpc('command.execute', {'expected_revision':initial['revision'],
                'description':'Author actual native asset workflow fixture', 'commands':author_commands(scene)})
            require(authored['revision'] == initial['revision'] + 1, 'Fixture authoring transaction')
            require(rpc('project.save', {'expected_revision':authored['revision']})['saved'], 'Fixture save')
            (output/source_name).write_bytes(original_bytes)
            (output/replacement_name).write_bytes(replacement_bytes)
            write(f'{args.asset_kind}-workflow.fixture.json', {'format':f'incant-native-{args.asset_kind}-workflow',
                  'version':1, 'project_id':initial['project']['id']})
            before = {name:digest(output/name) for name in ('project.json', 'move.ts', replacement_name)}
            write('fixture.json', {'source_revision':authored['revision'], 'inputs':before,
                  'journal_bytes_before':journal.stat().st_size, 'fixture_authored_through_cmd':True})
            receipt = output/'native-window.json'
            run([editor, project, f'--{args.asset_kind}-probe', receipt, ENTITY], bound=120)
            check()
            proof = validate_proof(json.loads(receipt.read_text()), args.asset_kind)
            require(before == {name:digest(output/name) for name in before}, 'Unrelated fixture/checkpoint sources changed')
            require((output/source_name).read_bytes() == (output/replacement_name).read_bytes(), 'Source recovery bytes')
            recovered = rpc('project.read')
            last = proof['stages'][-1]
            require(recovered['revision'] == last['history']['revision'], 'Durable app journal revision recovery')
            asset = next(a for a in recovered['project']['assets'].values() if a['path'] == source_name)
            require(asset['sha256'] == last['accepted_fingerprint'], 'Durable accepted recipe recovery')
            require(journal.stat().st_size > json.loads((output/'fixture.json').read_text())['journal_bytes_before'], 'Real app journal was not extended')
            result.update(passed=True, window=proof, recovered_revision=recovered['revision'],
                actual_app=True, native_processes=len(ledger), unrelated_authored_bytes_preserved=True,
                durable_journal_recovery=True)
    except BaseException as error:
        result['error'] = repr(error)
        raise
    finally:
        write('result.json', result)
    print(json.dumps({'passed':True, 'evidence':str(output/'result.json')}))


if __name__ == '__main__':
    main()
