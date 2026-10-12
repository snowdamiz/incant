#!/usr/bin/env python3
"""Public compound state fades, ordinary shipping QuickJS and full session resume.

The clock oracle is a closed-form timeline for this fixture, not a call to the
engine or a second implementation of graph edge selection. All pose groups and
captured outgoing clocks are checked, including paused/reversed/completed states.
"""
import argparse
import datetime
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
BEHAVIOR = Path(__file__).with_suffix('') / 'behavior.ts'
TREE = runpy.run_path(str(ROOT / 'tools/probes/native-animation-tree-export.py'))
BASE = TREE['BASE']
require, write, digest, fingerprints = [BASE[k] for k in ('require', 'write', 'digest', 'fingerprints')]
id = lambda n: f'{n:026}'
SCRIPT, FIRST, CAMERA, REST = map(id, (1, 2, 3, 4))
PARAMETERS = list(map(id, (10, 11, 12, 13)))
STATES = list(map(id, (20, 21, 22, 23)))
OBSERVED = list(map(id, (220, 221, 222, 230)))
TICKS = 56
CHECK_TICKS = tuple(range(1, TICKS + 1))
SPLITS = ((5, 'zero-weight'), (9, 'paused'), (13, 'reverse'), (21, 'weighted-to-pair'),
          (24, 'completed-incoming'), (29, 'pair-to-clip'), (31, 'manual-cancel'),
          (38, 'restarted-fade'), (45, 'completed-outgoing'), (53, 'clip-to-weighted'))
fixture = TREE['fixture']


def graph(rest=False):
    if rest:
        return TREE['graph'](True)
    result = TREE['graph']()
    result['parameters'][PARAMETERS[3]] = {'initial': 0, 'min': 0, 'max': 4}
    result['states'][STATES[2]] = {'looping': False, 'motion': {'kind': 'blend1d',
        'parameter': PARAMETERS[0], 'cycle_seconds': .5,
        'points': [{'threshold': 0, 'clip': 0}, {'threshold': 1, 'clip': 1}]}}
    result['states'][STATES[3]] = {'looping': True, 'motion': {'kind': 'clip', 'clip': 0}}
    result['transitions'] = {id(50 + n): {'from': STATES[n], 'to': STATES[(n + 1) % 4],
        'priority': 0, 'fade_seconds': .2 if n == 0 else .1, 'after_seconds': None,
        'conditions': [{'parameter': PARAMETERS[3], 'comparison': 'equal', 'value': n + 1}]}
        for n in range(4)}
    return result


def parameters(tick):
    xyz = [.25, .5, .75] if tick <= 9 else [.5, .25, .5]
    # Controls are queued after observations in the public script.
    q = next(value for start, value in reversed(((1, 0), (5, 1), (13, 2), (24, 3),
             (31, 0), (37, 1), (40, 0), (44, 3), (51, 4))) if tick >= start)
    return [*xyz, q]


def group(state, seconds, values):
    if state in (0, 1):
        phase = seconds / (2 if state == 0 else 1)
        x, y, z = values[:3]
        return 2, [[n if n < 7 else -1, (n + 1) * phase if n < 7 else 0,
                    (x if n & 1 else 1-x) * (y if n & 2 else 1-y) * (z if n & 4 else 1-z)]
                   for n in reversed(range(8))]
    if state == 2:
        return 1, [[0, seconds * 2, 1-values[0]], [1, seconds * 4, values[0]]]
    return 0, [[0, seconds, 1]]


def main_row(t):
    """All times are integer quarter seconds, fade/elapsed times integer ticks.

    The first fade advances twice, pauses for three ticks, then advances ten
    times with reversed incoming speed and retained positive outgoing speed.
    Later intervals cover automatic edge deferral and explicit cancellation.
    """
    paused = int(8 <= t <= 10)
    fade = None
    if t <= 4:
        state, q, elapsed, flags = 0, t % 8, t, 3
    elif t <= 20:
        k = min(t-5, 2) + max(t-10, 0)
        state, q, elapsed, flags = 1, (min(t-5, 2)-max(t-10, 0)) % 4, k, 2 if paused else 3
        if t < 20: fade = (0, (5+k) % 8 / 4, 15, 3, k, 12)
    elif t <= 27:
        k = t-21
        state, q, elapsed, flags = 2, 0, k, 1 if t == 21 else 4
        if t < 27: fade = (1, (3-k) % 4 / 4, -15, 3, k, 6)
    elif t <= 30:
        k = t-28
        state, q, elapsed, flags = 3, (-k) % 4, k, 3
        fade = (2, 0, -15, 1, k, 6)
    elif t <= 36:
        state, q, elapsed, flags = 0, (30-t) % 8, t-30, 3
    elif t <= 39:
        k = t-37
        state, q, elapsed, flags = 1, k % 4, k, 3
        fade = (0, (3+k) % 8 / 4, 15, 3, k, 12)
    elif t <= 43:
        state, q, elapsed, flags = 2, min(t-39, 2), t-39, 1 if t == 40 else 4
    elif t <= 50:
        k = t-44
        state, q, elapsed, flags = 3, k % 4, k, 3
        if t < 50: fade = (2, .5, 15, 1, k, 6)
    else:
        k = t-51
        state, q, elapsed, flags = 0, k % 8, k, 3
        fade = (3, (3+k) % 4 / 4, 15, 3, k, 6)
    seconds, values = q/4, parameters(t)
    kind, incoming = group(state, seconds, values)
    sample = incoming[1 if kind == 1 else 0][:2]
    poses, kinds, split, outer = (incoming, [kind, 255], 0, 0) if kind == 2 else ([], [255, 255], 0, 0)
    blend = [1, *incoming[0][:2], values[0]] if kind == 1 and fade is None else [0, -1, 0, 0]
    fade_values = [-1, 0, 0, 0, 0, 0]
    if fade is not None:
        source, source_seconds, speed, fade_flags, elapsed_ticks, duration_ticks = fade
        source_kind, outgoing = group(source, source_seconds, values)
        poses, kinds, split, outer = outgoing+incoming, [source_kind, kind], len(outgoing), elapsed_ticks/duration_ticks
        fade_values = [source, source_seconds, speed, fade_flags, elapsed_ticks/60, duration_ticks/60]
    return {'tick': t, 'entity': FIRST, 'state': state, 'seconds': seconds, 'elapsed': elapsed/60,
            'flags': flags, 'paused': paused, 'sample': sample, 'blend': blend, 'kinds': kinds,
            'split': split, 'outer': outer, 'fade': fade_values, 'poses': poses, 'parameters': values}


def rest_row(t):
    old = TREE['EXPECTED'][t][id(230)]
    return {'tick': t, 'entity': REST, 'state': 4, 'seconds': old[0],
            'elapsed': max(0, t-5-(min(t, 11)-8 if t >= 9 else 0))/60,
            'flags': old[1], 'paused': old[2], 'sample': [-1, 0], 'blend': [0, -1, 0, 0],
            'kinds': [2, 255], 'split': 0, 'outer': 0, 'fade': [-1, 0, 0, 0, 0, 0],
            'poses': [[-1, 0, 1]], 'parameters': []}


EXPECTED = {t: [main_row(t), rest_row(t)] for t in range(1, TICKS+1)}


def observation_values(tick):
    a, b = EXPECTED[tick]
    return {id(220): [a['split'], a['outer'], len(a['poses'])],
            id(221): [a['state'], a['seconds'], a['flags']],
            id(222): [a['fade'][0], a['fade'][4], a['paused']],
            id(230): [b['seconds'], b['flags'], b['paused']]}


def observations(start, end, wrong=False):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': OBSERVED,
            'checks': [{'name': f'{tick}-{entity}', 'tick': tick, 'expect': {'type': 'vector',
                       'entity': entity, 'property': 'translation',
                       'value': [value[0]+int(wrong), *value[1:]], 'tolerance': 1e-9}}
                       for tick in CHECK_TICKS if start < tick <= end for entity, value in observation_values(tick).items()]}


def close(actual, expected, path='row'):
    if type(expected) in (int, float):
        require(type(actual) in (int, float) and math.isfinite(actual) and abs(actual-expected) < 1e-9,
                f'{path}: {actual} != {expected}')
    elif isinstance(expected, dict):
        require(isinstance(actual, dict) and actual.keys() == expected.keys(), f'{path}: keys')
        for key in expected: close(actual[key], expected[key], f'{path}.{key}')
    elif isinstance(expected, list):
        require(isinstance(actual, list) and len(actual) == len(expected), f'{path}: length')
        for n, (a, b) in enumerate(zip(actual, expected)): close(a, b, f'{path}[{n}]')
    else:
        require(actual == expected, f'{path}: {actual} != {expected}')


def check_logs(logs, start, end):
    require(logs['dropped'] == 0 and len(logs['entries']) == 2*(end-start), 'Accepted log count')
    seen = set()
    for entry in logs['entries']:
        row = json.loads(entry['message'])
        t, entity = row['tick'], row['entity']
        require(entry['tick'] == t and entry['level'] == 'info' and start < t <= end, 'Log tick/level')
        require((t, entity) not in seen and entity in (FIRST, REST), 'Log identity')
        seen.add((t, entity))
        close(row, EXPECTED[t][int(entity == REST)], f'{entity}@{t}')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--stage', choices=('author', 'export', 'all'), default='all')
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    import export_native
    run = native_process.run
    commands = []
    def recorded(command, **kwargs):
        index = len(commands)
        record = {'argv': list(map(str, command)), 'at': datetime.datetime.now(datetime.timezone.utc).isoformat()}
        commands.append(record)
        start = time.monotonic()
        result = run(list(map(str, command)), **kwargs)
        record.update(returncode=result.returncode, seconds=time.monotonic()-start)
        for stream in ('stdout', 'stderr'):
            value = getattr(result, stream)
            if isinstance(value, (str, bytes)):
                path = output / f'command-{index:03}-{stream}.txt'
                path.write_bytes(value.encode() if isinstance(value, str) else value)
                record[stream] = {'file': path.name, 'sha256': digest(path)}
        write(output / 'commands.json', commands)
        return result
    native_process.run = recorded
    def execute(command, **kwargs):
        result = recorded(command, **kwargs)
        require(result.returncode == 0, str(result.stderr)[-8000:])
        return result
    inputs, project = output / 'authoring', output / 'authoring/project.json'
    if args.stage != 'export':
        output.mkdir(parents=True, exist_ok=False)
        inputs.mkdir()
        execute([binary, 'init', project, '--entities', '0'])
        fixture(inputs)
        imported = json.loads(execute([binary, 'import', project, 'rig.gltf']).stdout)
        document = json.loads(project.read_text())
        scene = next(iter(document['scenes']))
        require(document['settings']['tick_rate'] == 60, 'Default tick rate')
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        journal = project.with_suffix('.journal.jsonl')
        rpc = Rpc(binary, project, journal)
        calls = []
        def call(method, params=None):
            result = rpc.call(method, params)
            calls.append({'method': method, 'params': params, 'result': result})
            write(output / 'public-commands.json', calls)
            return result
        try:
            source = BEHAVIOR.read_text()
            revision = call('project.read')['revision']
            batch = [{'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                     {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Compound fade observations', 'path': 'compound.ts'}}]
            for entity in (FIRST, REST):
                rest = entity == REST
                batch.append({'op': 'create_entity', 'scene_id': scene, 'entity': {'id': entity,
                    'name': 'Completed rest tree' if rest else 'Compound animation transitions', 'parent': None, 'provenance': None,
                    'components': {'Transform': BASE['transform']([0, 0, 0]),
                        'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': False},
                        'Animation': {'clip': None, 'seconds': 0, 'speed': 15, 'playing': not rest, 'looping': not rest},
                        'AnimationGraph': graph(rest), 'Script': {'source': SCRIPT, 'props': {'rest': rest}}}}})
            for entity in OBSERVED + [CAMERA]:
                components = {'Transform': BASE['transform']([0, 0, 8] if entity == CAMERA else [0, 0, 0])}
                if entity == CAMERA:
                    components['Camera'] = {'fov_degrees': 60, 'near': .1, 'far': 200, 'projection': {'kind': 'perspective'}}
                batch.append({'op': 'create_entity', 'scene_id': scene, 'entity': {'id': entity,
                    'name': 'Pose observation', 'parent': None, 'provenance': None, 'components': components}})
            created = call('command.execute', {'expected_revision': revision, 'description': 'Author compound animation export proof', 'commands': batch})
            require(created['revision'] == revision + 1, 'One atomic transaction')
            call('history.undo')
            require(call('project.read')['project'] == document and not (inputs / 'compound.ts').exists(), 'Atomic Undo')
            call('history.redo')
            authored = call('project.read')
            played = call('play.run', {'seconds': TICKS / 60, 'camera': CAMERA, 'observe': observations(0, TICKS)})
            require(played['passed'] and played['state']['cpu_render_input'] and not played['state']['gpu_submitted'], 'Public CPU Play')
            check_logs(played['state']['logs'], 0, TICKS)
            require(call('project.read') == authored, 'Play changed authoring')
            require(call('project.save')['saved'], 'Save')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(call('project.read') == authored, 'Fresh recovery')
            require(call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == source, 'Script recovery')
        finally:
            rpc.close()
        write(output / 'authoring.json', {'project_sha256': digest(project), 'binary_sha256': digest(binary),
            'files': fingerprints(inputs), 'logs': played['state']['logs'], 'source_sha256': digest(BEHAVIOR),
            'command_bus_only': True, 'undo_redo_reopen': True, 'authoring_unchanged_by_play': True})
        write(output / 'independent-oracle.json', EXPECTED)
        if args.stage == 'author':
            print(json.dumps({'authoring_passed': True, 'output': str(output)}))
            return
    authored = json.loads((output / 'authoring.json').read_text())
    require(digest(project) == authored['project_sha256'] and fingerprints(inputs) == authored['files']
            and digest(binary) == authored['binary_sha256'] and digest(BEHAVIOR) == authored['source_sha256'], 'Preflight input changed')
    if args.stage == 'export':
        commands.extend(json.loads((output / 'commands.json').read_text()))
    exported = export_native.export(project, output / 'package', camera=CAMERA)
    require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Export identity')
    removed_maps = BASE['remove_source_maps'](output / 'package')
    shutil.rmtree(inputs)
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    environment = {'PATH': str(player.parent), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    environment.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    runs = []
    def launch(name, ticks, start=0, load=None, wrong=False, diagnostic=None):
        observe, pcm, save = [output / (name + ext) for ext in ('-observe.json', '.pcm', '.save')]
        write(observe, observations(start, start + ticks, wrong))
        command = [player, '--package', player.parent / 'data', '--ticks', ticks, '--output', pcm,
                   '--observe', observe, '--save-checkpoint', save]
        if load is not None: command.extend(['--load-checkpoint', load])
        result = recorded(command, cwd=player.parent, env=environment, timeout=60)
        if diagnostic is not None:
            require(result.returncode == 1 and diagnostic == result.stderr.strip() and not pcm.exists() and not save.exists(),
                    name + ': expected typed refusal: ' + result.stderr)
            if wrong:
                # An assertion failure deliberately emits its complete diagnostic
                # report before withholding checkpoint/PCM publication.
                report = json.loads(result.stdout)
                require(report['tick'] == start + ticks and not report['observations']['passed']
                        and len(report['observations']['checks']) == len(OBSERVED)
                        and all(not row['passed'] for row in report['observations']['checks']),
                        'Expected complete failed-observation report')
                check_logs(report['logs'], start, start + ticks)
            else:
                require(not result.stdout, 'Admission refusal published stdout')
            return None
        require(result.returncode == 0, name + ': ' + result.stderr)
        report = json.loads(result.stdout)
        require(report['tick'] == start + ticks and report['observations']['passed'], 'Public observation result')
        require(report['render_input']['model_count'] == 2 and not report['render_input']['gpu_submitted'], 'Retained CPU models')
        check_logs(report['logs'], start, start + ticks)
        require(BASE['checkpoint_version'](save) == (10 if EXPECTED[start+ticks][0]['fade'][0] >= 0 and start+ticks not in (30, 39) else 6), 'Graph save version')
        write(output / (name + '.json'), report)
        runs.append({'name': name, 'start': start, 'ticks': ticks, 'report_sha256': digest(output / (name + '.json')),
                     'pcm_sha256': digest(pcm), 'save_sha256': digest(save)})
        return report
    baseline = launch('baseline', TICKS)
    require(baseline['logs'] == authored['logs'], 'Public Play/export observations differ')
    for n in range(2):
        repeat = launch(f'repeat-{n}', TICKS)
        require(repeat == baseline and (output / f'repeat-{n}.save').read_bytes() == (output / 'baseline.save').read_bytes(), 'Repeated process differs')
    for split, label in SPLITS:
        first = launch(label, split)
        resumed = launch(label + '-continued', TICKS - split, split, output / (label + '.save'))
        require((output / (label + '-continued.save')).read_bytes() == (output / 'baseline.save').read_bytes(), 'Resumed final save differs')
        require((output / (label + '.pcm')).read_bytes() + (output / (label + '-continued.pcm')).read_bytes() == (output / 'baseline.pcm').read_bytes(), 'Resumed PCM differs')
        require(first['logs']['entries'] + resumed['logs']['entries'] == baseline['logs']['entries'], 'Resumed observations differ')
        require(resumed['render_input'] == baseline['render_input'], 'Resumed render input differs')
    corrupt = output / 'corrupt-input.save'
    data = bytearray((output / 'paused.save').read_bytes()); data[-1] ^= 1; corrupt.write_bytes(data)
    launch('corrupt', 1, 9, corrupt, diagnostic='Error: Checkpoint(Integrity)')
    launch('wrong-observation', 1, wrong=True, diagnostic='Error: "checkpoint withheld: native observation assertions failed"')
    require(not list(output.glob('.incant-checkpoint-*')), 'Leaked checkpoint staging')
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'export': exported, 'runs': runs,
        'vector_assertions_per_full_run': len(OBSERVED) * len(CHECK_TICKS), 'full_pose_log_ticks': TICKS, 'fresh_full_processes': 3, 'continuation_pairs': len(SPLITS),
        'authoring_deleted': not inputs.exists(), 'removed_source_maps': removed_maps,
        'package_files': fingerprints(output / 'package/player'), 'probe_sha256': digest(Path(__file__)),
        'behavior_sha256': digest(BEHAVIOR), 'commands_sha256': digest(output / 'commands.json'),
        'limits': ['CPU render-input and complete sample observations, not GPU pixels or device/performance gate',
                   'Combined authored IK, root motion and module checkpoint state remain separate unfinished work']} )
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
