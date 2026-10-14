#!/usr/bin/env python3
"""Short deterministic native Core/stress equivalence; no GPU or timing gate."""
import argparse
import json
import os
from pathlib import Path
import sys

import stress
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
import native_process

TICKS = 65


def run(binary, project, plan, out, name):
    observation = out / (name + '-observe.json'); stress.write(observation, plan)
    argv = [str(binary), 'native-play', str(project), '--camera', stress.ids.entity(3),
            '--ui-viewport', '640x480', '--ticks', str(TICKS), '--sample-rate', '48000',
            '--input-replay', str(out / 'input.json'), '--observe', str(observation),
            '--pcm-output', str(out / (name + '.pcm'))]
    stdout = out / (name + '.stdout'); stderr = out / (name + '.stderr')
    code = None; failure = None
    try:
        with stdout.open('wb') as output, stderr.open('wb') as error:
            result = native_process.run(argv, stdout=output, stderr=error, timeout=180,
                env={**os.environ, 'RAYON_NUM_THREADS': '2'})
        code = result.returncode
        if code != 0: raise ValueError(name + ' native command refused; see retained stderr')
        report = json.loads(stdout.read_text())
        if not (report['passed'] and report['runtime'] == 'native' and report['script_source_parsing']
                and report['tick'] == TICKS and report['cpu_render_input'] and not report['gpu_submitted']
                and report['observations']['passed'] and report['logs']['dropped'] == 0):
            raise ValueError(name + ' native proof contract differs')
        if stress.digest(out / (name + '.pcm')) != report['pcm_sha256']:
            raise ValueError(name + ' PCM digest differs')
        return report
    except BaseException as error:
        failure = repr(error); raise
    finally:
        stress.write(out / (name + '-command.json'), {'argv': argv, 'exit_code': code,
            'failure': failure, 'binary_sha256': stress.digest(binary),
            'project_sha256': stress.digest(project), 'stdout_sha256': stress.digest(stdout),
            'stderr_sha256': stress.digest(stderr), 'owned_process_reaped': True})


def verify(original, authored, binary, out):
    original = original.resolve(strict=True); authored = authored.resolve(strict=True)
    binary = binary.resolve(strict=True); out = out.resolve(); out.mkdir(parents=True, exist_ok=False)
    before_original = stress.inventory(original.parent); before_authored = stress.inventory(authored.parent)
    source = json.loads(original.read_text()); actual = json.loads(authored.read_text())
    stress.verify_extension(source, actual)
    if source['settings'].get('physics_mode', 'deterministic') != 'deterministic':
        raise ValueError('Retained numerical helper admits only the deterministic reference')
    receipt = {'passed': False, 'ticks': TICKS, 'original_project_sha256': stress.digest(original),
        'authored_project_sha256': stress.digest(authored), 'script_sha256': stress.digest(authored.parent / 'stress.ts'),
        'binary_sha256': stress.digest(binary), 'performance_measured': False}
    stress.write(out / 'input.json', {'format': 'incant-input', 'version': 1, 'tick_rate': 60,
        'start_tick': 0, 'ticks': TICKS, 'frames': []})
    entities = source['scenes'][stress.bindings.SCENE]['entities']
    original_ids = sorted(entities)
    baseline_plan = {'format': 'incant-native-observation', 'version': 1, 'entities': original_ids, 'checks': [
        {'name': 'reference-population', 'tick': TICKS, 'expect': {'type': 'entity_count', 'value': len(entities)}}]}
    plan = stress.observation(TICKS, len(entities))
    plan['checks'] = [check for tick in (1, 63, 64, 65) for check in stress.observation(tick, len(entities))['checks']]
    # Observation names must be unique across the plan.
    for check in plan['checks']: check['name'] += '-tick-' + str(check['tick'])
    plan['entities'] += original_ids
    try:
        baseline = run(binary, original, baseline_plan, out, 'reference')
        extension = run(binary, authored, plan, out, 'stress')
        for name in ('logs', 'pcm_sha256'):
            if baseline[name] != extension[name]: raise ValueError('Original Core changed: ' + name)
        for entity in original_ids:
            if baseline['observations']['entities'][entity] != extension['observations']['entities'][entity]:
                raise ValueError('Original Core state changed: ' + entity)
        receipt.update(passed=True, scripted_entities=10000, original_entity_states_equal=len(original_ids),
                       observation_checks=len(plan['checks']), pcm_equal=True, logs_equal=True, cpu_render_input_prepared=True)
    finally:
        receipt['original_files_unchanged'] = stress.inventory(original.parent) == before_original
        receipt['authored_files_unchanged'] = stress.inventory(authored.parent) == before_authored
        stress.write(out / 'receipt.json', receipt)
    if not receipt['original_files_unchanged'] or not receipt['authored_files_unchanged']:
        raise ValueError('Native play changed authoring files')
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('original', type=Path); parser.add_argument('authored', type=Path)
    parser.add_argument('output', type=Path); parser.add_argument('--binary', type=Path, required=True)
    a = parser.parse_args(); print(json.dumps(verify(a.original, a.authored, a.binary, a.output), indent=2))

if __name__ == '__main__': main()
