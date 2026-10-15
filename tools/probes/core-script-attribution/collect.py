#!/usr/bin/env python3
"""One explicitly granted functional OR diagnostic child; never builds or retries."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import time

from common import ROOT, TICKS, checked_file, load, package_identity, plan, source_check
from analyze import analyze
from common import NAV_FEATURES, BUDGET_FEATURES
import mac_performance
import native_process
from native_package import read, require, sha256, write_json

spec = importlib.util.spec_from_file_location('maintained_sequence', ROOT / 'tools/probes/quickjs-maintained-profile/measure.py')
sequence = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sequence)


def collect(plan_path, package, output, stage, functional):
    data, plan_hash = plan(plan_path)
    source = source_check(data)
    player, build, identity = package_identity(package)
    require(build['plan_sha256'] == plan_hash and build['source'] == source, 'Build/source plan mismatch')
    expected_path, expected_raw = checked_file(data['functional_report'], 16 * 1024**2)
    expected = json.loads(expected_raw)
    _, pcm = checked_file(data['functional_pcm'], 128 * 1024**2)
    require('profile' not in expected and expected['tick'] == TICKS
            and expected['observations']['passed'] is True and sha256(pcm) == expected['pcm_sha256']
            and len(pcm) == TICKS * 48000 // 60 * 8, 'Incomplete historical functional anchor')
    _, profile_raw = checked_file(data['source_identity_report'], 16 * 1024**2)
    expected_sources = [s['source'] for s in json.loads(profile_raw)['profile']['scripts']]
    require(len(expected_sources) == 2, 'Expected exactly two callback sources')
    inputs = {key: checked_file(data[key], 8 * 1024**2)[1] for key in ('input', 'observe')}
    require(stage in ('functional', 'diagnostic'), 'Unknown stage')
    if stage == 'diagnostic':
        require(functional is not None, 'The same-build functional control is required first')
        prior = load(functional)
        require(prior['status'] == 'complete' and prior['stage'] == 'functional'
                and prior['build_identity'] == identity and prior['plan_sha256'] == plan_hash
                and prior['logical_and_pcm_equal'] is True, 'Functional control does not bind this build')
    else:
        require(functional is None, 'Functional stage takes no predecessor')
    output = output.absolute()
    require(str(output) == data['outputs'][stage], 'Output differs from single fixed stage location')
    require(str(Path(package).absolute()) == data['build_output'], 'Build location changed')
    if stage == 'diagnostic':
        require(str(Path(functional).absolute()) == str(Path(data['outputs']['functional']) / 'result.json'),
                'Functional result location changed')
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(), 'Output exists')
    require(output != player.parent and player.parent not in output.parents
            and output not in player.parents and ROOT not in output.parents, 'Output/input overlap')
    output.mkdir()
    for key, raw in inputs.items():
        (output / (key + '.json')).write_bytes(raw)
    argv = [str(player), '--package', str(player.parent / 'data'), '--ticks', str(TICKS),
            '--output', str(output / 'output.pcm'), '--input-recording', str(output / 'input.json'),
            '--observe', str(output / 'observe.json'), '--ui-viewport', '640x480']
    if stage == 'diagnostic':
        argv.append('--profile')  # Existing default is maintained; no detailed heap walk.
    receipt = {'format': 'incant-script-attribution-child', 'version': 1, 'stage': stage,
               'status': 'running', 'plan_sha256': plan_hash, 'build_identity': identity,
               'source': source, 'argv': argv, 'child_timeout_seconds': 45,
               'execution_deadline_seconds': 90, 'started_ns': time.time_ns(), 'exit': None,
               'eligible_for_performance_gate': False, 'retries': 0,
               'historical_functional_sha256': sha256(expected_raw)}
    write_json(output / 'result.json', receipt)
    stdout, stderr, audio = output / 'stdout.json', output / 'stderr.log', output / 'output.pcm'
    limits = [(stdout, 32 * 1024**2), (stderr, 1024**2), (audio, 128 * 1024**2)]
    try:
        with sequence.bounded_sequence(90) as cancelled:
            def bounded():
                cancelled()
                for path, cap in limits:
                    require(not path.exists() or path.stat().st_size <= cap, 'Child output exceeded bound')
            receipt['machine_before'] = mac_performance.mac_identity()
            bounded()
            receipt['load_before'] = os.getloadavg()
            with stdout.open('xb') as out, stderr.open('xb') as err:
                result = native_process.run(argv, cwd=player.parent,
                                            env={'PATH': str(player.parent), 'TMPDIR': str(output)},
                                            stdout=out, stderr=err, timeout=45, cancel=bounded)
            receipt.update(exit=result.returncode, terminal_reaped=True)
            bounded()
            require(result.returncode == 0, 'Child failed; no retry')
            raw = read(stdout, 32 * 1024**2)
            actual = json.loads(raw)
            require(not read(stderr, 1024**2), 'Unexpected child diagnostics')
            require({k: v for k, v in actual.items() if k != 'profile'} == expected,
                    'Full historical logical result differs')
            require(read(audio, 128 * 1024**2) == pcm, 'PCM differs')
            if stage == 'diagnostic':
                observation = analyze(actual, expected_sources, navigation=data['features'] == NAV_FEATURES,
                                      budget=data['features'] == BUDGET_FEATURES)
                write_json(output / 'attribution.json', observation)
            else:
                require('profile' not in actual, 'Unprofiled control emitted a capture')
            receipt['machine_after'] = mac_performance.mac_identity()
            bounded()
            require(receipt['machine_after'] == receipt['machine_before'], 'Machine settings changed')
            receipt['load_after'] = os.getloadavg()
            require(package_identity(package)[2] == identity and source_check(data) == source,
                    'Frozen source/package changed during child')
            bounded()
            receipt.update(status='complete', logical_and_pcm_equal=True)
    except BaseException as error:
        receipt.update(status='failed', error=type(error).__name__ + ': ' + str(error))
        raise
    finally:
        receipt['ended_ns'] = time.time_ns()
        receipt['artifacts'] = {p.name: {'sha256': sha256(p.read_bytes()), 'bytes': p.stat().st_size}
                                for p, _ in limits if p.exists() and p.stat().st_size <= dict(limits)[p]}
        write_json(output / 'result.json', receipt)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('plan', type=Path)
    parser.add_argument('package', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--stage', choices=('functional', 'diagnostic'), required=True)
    parser.add_argument('--functional-result', type=Path)
    args = parser.parse_args()
    collect(args.plan, args.package, args.output, args.stage, args.functional_result)
