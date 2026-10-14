#!/usr/bin/env python3
"""Functional all-row parity, then matched per-frame CPU intervals on actual Mac.

No compiler/GPU/startup instrumentation. Cross-tier ratios are descriptive, never
fed to the existing same-workload TypeScript regression or 2ms gate policy.
"""
import argparse
import json
from pathlib import Path

from cost_workload import (COUNT, TICKS, RATE, prepared, require, file_hash, read, write_json,
                    position, owner, run)
import mac_performance as performance

KINDS = ('typescript', 'wasm')
PHASE = {'typescript': 'script.tick', 'wasm': 'script.static_module'}


def package_pair(inputs, packages):
    workload = prepared(inputs)
    receipt = json.loads(read(packages / 'pair.json', 1024**2))
    require(receipt['format'] == 'incant-wasm-ts-exports' and receipt['version'] == 1
            and receipt['prepared_sha256'] == file_hash(inputs / 'prepared.json'), 'Export workload binding differs')
    values = {}
    for kind in KINDS:
        package = packages / kind
        require(file_hash(package / 'export.json') == receipt['export_sha256'][kind], 'Export receipt changed')
        player, exported, identity, rate = performance.package_inputs(package)
        require(rate == RATE and exported['project_sha256'] == workload['files'][f'{kind}/project.json']
                == receipt['project_sha256'][kind], 'Export project/rate differs')
        performance.phase_selection(PHASE[kind], identity['program'])
        require(kind != 'wasm' or (identity['program']['input_sha256'] == workload['guest']['wasm_sha256']
                and identity['program']['translator'] == 'wasm2c-1.0.42'), 'Wasm module/translator differs')
        cooked = json.loads(read(package / 'project-build.json', 16 * 1024**2))
        require(cooked['project_id'] == workload['project_id'] and cooked['scene_id'] == workload['scene_id']
                and cooked['scene_sha256'] == identity['data']['world.scene'], 'Cooked scene identity differs')
        values[kind] = {'player': player, 'exported': exported, 'identity': identity}
    return values, package_equivalence(*(values[k] for k in KINDS))


def package_equivalence(left, right):
    for key in ('target', 'compiler', 'codegen', 'physics_mode', 'scene_id', 'camera'):
        require(left['exported'][key] == right['exported'][key], 'Common shipping configuration differs: ' + key)
    require(left['identity']['data'] == right['identity']['data'], 'Cooked scene or input/data bytes differ')
    feature_delta = {}
    features = [v['exported']['profiling']['compiler']['features'] for v in (left, right)]
    require(set(features[0]) == set(features[1]), 'Profiling host package set differs')
    for name in features[0]:
        a, b = set(features[0][name]), set(features[1][name])
        expected = {'static-module'} if name in ('incant_script', 'incant_game_player') else set()
        require(not a - b and b - a == expected, 'Unexpected compiled host feature delta: ' + name)
        feature_delta[name] = sorted(b - a)
    # Full normal dependency graphs remain separately retained in each export.
    # Static Wasm necessarily adds incant_wasm_runtime and its trusted C runtime.
    return feature_delta


def functional(report, indices):
    require(report['tick'] == TICKS and report['observations']['tick_rate'] == RATE
            and report['observations']['passed'] is True, 'Functional player observation failed')
    require(report['frames'] == 48000 * TICKS // RATE and report['logs'] == {'entries': [], 'dropped': 0}
            and report['sources'] == [] and report['flipbooks'] == [], 'Unexpected audio/log/runtime output')
    entities = report['observations']['entities']
    require(set(entities) == {owner(n) for n in indices}, 'Observed membership differs')
    for n in indices:
        row = entities[owner(n)]
        require(row is not None and row['parent'] is None and row['rotation'] == [0, 0, 0, 1]
                and row['scale'] == [1, 1, 1] and row['velocity'] is None, 'Unchanged transform columns differ')
        require(all(abs(a - b) <= 1e-9 for a, b in zip(row['translation'], position(n)))
                and len(row['translation']) == 3, 'Actual row differs from independent arithmetic')
        require(row['world_transform'][:3] == [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]
                and row['world_transform'][3] == row['translation'] + [1], 'World transform coherence differs')
    # Compare all visible runtime fields, logs and rational audio output counters,
    # while excluding measurements and backend artifact identities.
    return {key: report[key] for key in ('tick', 'frames', 'logs', 'observations', 'sources', 'flipbooks')}


def silent_pcm(path):
    raw = read(path, 128 * 1024**2)
    require(len(raw) == 48000 * TICKS // RATE * 8 and not any(raw), 'Expected exact silent stereo float32 PCM')
    return performance.digest(raw)


def ratio(numerator, denominator):
    for bounds in (numerator, denominator):
        require(performance.integer(bounds['lower'], 'interval lower') <=
                performance.integer(bounds['upper'], 'interval upper'), 'Reversed interval')
    if denominator['lower'] == 0:
        return {'defined': False, 'reason': 'denominator interval includes zero'}
    # Rational endpoints preserve quantization exactly; floats are display only.
    return {'defined': True, 'lower': [numerator['lower'], denominator['upper']],
            'upper': [numerator['upper'], denominator['lower']]}


def measure(inputs, packages, output):
    inputs, packages, output = inputs.absolute(), packages.absolute(), output.absolute()
    values, delta = package_pair(inputs, packages)
    require(not output.exists() and output.parent.is_dir(), 'Choose new measurement output')
    output.mkdir()
    parity = []
    for page in range(4):
        observed = {}
        for kind in KINDS:
            player = values[kind]['player']
            pcm = output / f'functional-{page}-{kind}.pcm'
            raw = run([player, '--package', player.parent / 'data', '--ticks', TICKS,
                       '--input-recording', inputs / 'input.json', '--observe', inputs / f'observe-{page}.json',
                       '--output', pcm], output / f'functional-{page}-{kind}', cwd=player.parent,
                      env={'PATH': str(player.parent), 'TMPDIR': str(output)})
            observed[kind] = functional(json.loads(raw), range(page * 256, (page + 1) * 256))
        require(observed['typescript'] == observed['wasm'], 'All-row native outputs differ between tiers')
        hashes = [silent_pcm(output / f'functional-{page}-{k}.pcm') for k in KINDS]
        require(hashes[0] == hashes[1], 'Actual PCM differs')
        parity.append({'page': page, 'entities': 256, 'pcm_sha256': hashes[0]})
    runs, comparisons = [], []
    # Reverse alternating order instead of permanently favoring the second tier.
    for repeat in range(3):
        measured = {}
        for kind in (KINDS if repeat % 2 == 0 else tuple(reversed(KINDS))):
            measured[kind] = performance.measure(packages / kind, output / f'{repeat}-{kind}', TICKS,
                                                 inputs / 'input.json', inputs / 'observe.json',
                                                 script_phase=PHASE[kind])
        a, b = (measured[k] for k in KINDS)
        require(a['match']['machine'] == b['match']['machine'], 'Mac identity changed between tiers')
        for kind in KINDS:
            raw = json.loads(read(output / f'{repeat}-{kind}/stdout.json', 16 * 1024**2))
            expected = functional(raw, [0, COUNT // 2, COUNT - 1])
            if kind == 'typescript': prior = expected
            else: require(expected == prior, 'Timed workload output differs')
        pcm = [silent_pcm(output / f'{repeat}-{k}/output.pcm') for k in KINDS]
        require(pcm[0] == pcm[1], 'Timed PCM differs')
        comparisons.append({'repeat': repeat, 'wasm_over_typescript_p95': ratio(
            b['metrics']['script.static_module.p95_wall_ns'], a['metrics']['script.tick.p95_wall_ns'])})
        runs.append({k: file_hash(output / f'{repeat}-{k}/measurement.json') for k in KINDS})
    package_pair(inputs, packages)  # Detect changes across the complete experiment.
    result = {'format': 'incant-wasm-ts-cost', 'version': 1, 'entities': COUNT, 'ticks_each': TICKS,
              'prepared_sha256': file_hash(inputs / 'prepared.json'), 'exports_sha256': file_hash(packages / 'pair.json'),
              'compiled_host_feature_delta': delta, 'functional_all_rows': parity,
              'measurements': runs, 'descriptive_intervals': comparisons, 'phase_gate_passed': False,
              'limits': ['Different host tiers; not a matched-regression policy result',
                         'Per-frame CPU cost includes each tier bulk bridge/commit, not just guest arithmetic',
                         'No startup measurement; process wall time is diagnostic only',
                         'No GPU/FPS, Core Sample, 10000-entity or mobile gate claim']}
    write_json(output / 'result.json', result)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('inputs', 'packages', 'output'): parser.add_argument(name, type=Path)
    args = parser.parse_args()
    print(json.dumps(measure(args.inputs, args.packages, args.output), indent=2))
