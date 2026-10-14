#!/usr/bin/env python3
"""Measure an exported shipping headless player on this Mac; compare matched CPU runs.

This records CPU script/frame distributions and requested Rust memory, not display
fps, GPU time, RSS or a complete phase gate. It never builds, signs or edits a game.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time

import native_process
from native_package import read, require, write_json
from export_native import verify_codegen, ui_startup_options

WIDTH = 10_000
MAX_NS = 100_000_000
PHASES = ('player.frame', 'script.tick')
SCRIPT_PHASES = ('script.tick', 'script.static_module')
HEAP_MODES = ('maintained', 'detailed')


def phase_selection(script_phase, program):
    require(script_phase in SCRIPT_PHASES, 'Unknown script timing phase')
    require(program is None or isinstance(program, dict) and program.get('kind') == 'wasm',
            'Unknown exported program kind')
    wasm = program is not None
    require(wasm == (script_phase == 'script.static_module'),
            'Script timing phase differs from actual exported program kind')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def integer(value, name, minimum=0):
    require(type(value) is int and minimum <= value <= 2**64 - 1, 'Invalid ' + name)
    return value


def command_text(args):
    return subprocess.check_output(args, text=True, timeout=10).strip()


def mac_identity():
    require(platform.system() == 'Darwin', 'Actual macOS execution is required')
    fields = {name: command_text(['/usr/sbin/sysctl', '-n', key]) for name, key in
              [('model', 'hw.model'), ('chip', 'machdep.cpu.brand_string'),
               ('memory_bytes', 'hw.memsize'), ('physical_cpus', 'hw.physicalcpu'),
               ('os_build', 'kern.osversion')]}
    fields.update(os_version=command_text(['/usr/bin/sw_vers', '-productVersion']),
                  architecture=platform.machine())
    power = command_text(['/usr/bin/pmset', '-g', 'batt'])
    match = re.search(r"Now drawing from '([^']+)'", power)
    require(match is not None, 'Cannot identify current Mac power source')
    fields['power_source'] = match[1]
    settings = command_text(['/usr/bin/pmset', '-g'])
    low_power = re.search(r'^\s*lowpowermode\s+(\d+)\s*$', settings, re.MULTILINE)
    fields['low_power_mode'] = int(low_power[1]) if low_power else None
    require(all(fields[key] for key in ('model', 'chip', 'os_build', 'os_version')), 'Missing Mac identity')
    return fields


def interval(row, frames):
    """Recompute nearest-rank p95 from bounded bins; never trust a supplied p95."""
    require(row['samples'] == frames and all(row[k] == 0 for k in
            ('missing_frames', 'invalid_frames', 'overflow_samples')), 'Incomplete phase sampling')
    counts = row['counts']
    require(isinstance(counts, list) and len(counts) == MAX_NS // WIDTH + 1, 'Unknown histogram shape')
    require(sum(integer(n, 'bucket count') for n in counts) == frames, 'Histogram coverage differs')
    rank = (95 * frames + 99) // 100
    total = 0
    for index, count in enumerate(counts):
        total += count
        if total >= rank:
            lower, upper = (0, 0) if index == 0 else ((index - 1) * WIDTH + 1, index * WIDTH)
            require(row['p95_lower_ns'] == lower and row['p95_upper_ns'] == upper,
                    'Reported quantile disagrees with histogram')
            return {'lower': lower, 'upper': upper}
    raise ValueError('Empty histogram')


def profile_instrumentation(report):
    """Observation overhead is part of matched timing, including legacy reports."""
    p = report['profile']
    version = integer(p['version'], 'profile version', 1)
    if version == 2:
        return {'profile_version': 2, 'quickjs_heap_mode': 'legacy_full_scan'}
    require(version == 3, 'Incomplete or unsupported profile')
    heap = p['quickjs_heap']
    require(integer(heap['version'], 'heap version', 1) == 1 and heap['mode'] in ('maintained', 'detailed')
            and heap['sampling'] == 'script_tick_end_per_vm', 'Unsupported heap instrumentation')
    require(integer(heap['invalid_samples'], 'invalid heap samples') == 0,
            'Invalid heap observations')
    samples = integer(heap['samples'], 'heap samples')
    require(integer(heap['detailed_samples'], 'detailed heap samples') ==
            (samples if heap['mode'] == 'detailed' else 0), 'Heap mode coverage disagrees')
    keys = ('sampled_peak_allocated_bytes', 'sampled_peak_allocation_count',
            'sampled_peak_estimated_used_bytes')
    if samples == 0:
        require(heap['latest'] is None and all(heap[k] is None for k in keys),
                'Missing heap samples must remain explicit')
    else:
        latest = heap['latest']
        for field, peak in [('allocated_bytes', keys[0]), ('allocation_count', keys[1])]:
            require(integer(heap[peak], peak) >= integer(latest[field], field), 'Heap peak below latest')
        integer(latest['limit_bytes'], 'heap limit')
        if heap['mode'] == 'maintained':
            require(latest['estimated_used_bytes'] is None and heap[keys[2]] is None,
                    'Maintained mode cannot claim estimated used size')
        else:
            require(integer(heap[keys[2]], keys[2]) >=
                    integer(latest['estimated_used_bytes'], 'estimated used bytes'), 'Heap peak below latest')
    return {'profile_version': 3, 'quickjs_heap_version': 1,
            'quickjs_heap_mode': heap['mode'], 'quickjs_heap_sampling': heap['sampling']}


def metrics(report, ticks, *, script_phase='script.tick'):
    require(script_phase in SCRIPT_PHASES, 'Unknown script timing phase')
    phases = ('player.frame', script_phase)
    integer(ticks, 'ticks', 1)
    require(report['tick'] == ticks, 'Player did not finish requested ticks')
    p = report['profile']
    profile_instrumentation(report)
    require(p['completed_frames'] == ticks and p['failed_frames'] == 0,
            'Incomplete or unsupported profile')
    require(p['dropped_source_samples'] == 0 and p['tracy_process_transport'] is False,
            'Dropped script samples or unexpected transport')
    frame = p['frame_wall']
    require(integer(frame['version'], 'frame version', 1) == 1 and frame['unit'] == 'ns'
            and frame['bucket_width_ns'] == WIDTH and frame['max_ns'] == MAX_NS,
            'Unsupported frame histogram protocol')
    require(frame['attempted_frames'] == ticks and frame['successful_frames'] == ticks
            and all(frame[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
            and frame['saturated'] is False, 'Invalid frame coverage')
    rows = frame['phases']
    require(isinstance(rows, list) and len({r['name'] for r in rows}) == len(rows), 'Duplicate phase rows')
    by_name = {row['name']: row for row in rows}
    require(all(name in by_name for name in phases), 'Missing required frame phase')
    values = {name + '.p95_wall_ns': interval(by_name[name], ticks) for name in phases}
    allocations = p['rust_allocations']
    require(allocations['installed'] is True and allocations['saturated'] is False,
            'Unavailable or saturated allocation counter')
    peak = integer(allocations['process_peak_requested_bytes'], 'Rust requested peak', 1)
    values['rust.process_peak_requested_bytes'] = {'lower': peak, 'upper': peak}
    return values


def comparison(current, baseline):
    require(current['format'] == baseline['format'] == 'incant-mac-cpu-measurement'
            and integer(current['version'], 'measurement version', 1) in (1, 2)
            and integer(baseline['version'], 'measurement version', 1) in (1, 2), 'Unknown measurement format')
    for item in (current, baseline):
        if item['version'] == 2:
            require('profile_instrumentation' in item['match'], 'Missing instrumentation identity')
        require(item.get('script_phase', 'script.tick') == item['match'].get('script_phase', 'script.tick'),
                'Measurement phase disagrees with its identity')
    def matched(item):
        identity = dict(item['match'])
        if item['version'] == 1:
            require('profile_instrumentation' not in identity, 'Legacy measurement has new identity fields')
            identity['profile_instrumentation'] = {'profile_version': 2, 'quickjs_heap_mode': 'legacy_full_scan'}
        return identity
    require(matched(current) == matched(baseline),
            'Baseline machine, workload, build configuration or instrumentation differs')
    require(current['match'].get('script_phase', 'script.tick') == 'script.tick',
            'The TypeScript regression policy does not compare static-module phases')
    result = {}
    for name, value in current['metrics'].items():
        prior = baseline['metrics'][name]
        for bounds in (value, prior):
            require(integer(bounds['lower'], 'metric lower') <= integer(bounds['upper'], 'metric upper'),
                    'Reversed metric interval')
        # Compare intervals: quantization must not create a false pass near 5%.
        if value['upper'] * 100 <= prior['lower'] * 105:
            verdict = 'pass'
        elif value['lower'] * 100 > prior['upper'] * 105:
            verdict = 'regression'
        else:
            verdict = 'inconclusive'
        result[name] = verdict
    script = current['metrics']['script.tick.p95_wall_ns']
    budget = 'pass' if script['upper'] <= 2_000_000 else (
        'fail' if script['lower'] > 2_000_000 else 'inconclusive')
    return {'regression_limit_percent': 5, 'metrics': result,
            'script_2ms_p95': budget,
            'passed': budget == 'pass' and all(v == 'pass' for v in result.values()),
            'phase_gate_passed': False}


def load_baseline(path):
    baseline = json.loads(read(path, 16 * 1024**2))
    raw = read(path.parent / 'stdout.json', 16 * 1024**2)
    require(digest(raw) == baseline['profile_report_sha256'], 'Baseline raw report identity differs')
    report = json.loads(raw)
    instrumentation = profile_instrumentation(report)
    version = integer(baseline['version'], 'measurement version', 1)
    if version == 1:
        require(instrumentation['profile_version'] == 2, 'Legacy measurement cannot carry new instrumentation')
    else:
        require(version == 2 and baseline['match'].get('profile_instrumentation') == instrumentation,
                'Baseline instrumentation identity differs')
    require(metrics(report, baseline['match']['ticks'],
                    script_phase=baseline['match'].get('script_phase', 'script.tick')) == baseline['metrics'],
            'Baseline metrics differ from actual retained profile')
    return baseline


def package_inputs(package):
    """Bind an existing public exporter result, actual compiler log and runtime tree."""
    exported_bytes = read(package / 'export.json', 16 * 1024**2)
    exported = json.loads(exported_bytes)
    target = {'arm64': 'aarch64-apple-darwin', 'x86_64': 'x86_64-apple-darwin'}.get(platform.machine())
    require(exported['format'] == 'incant-native-export' and exported['version'] == 1
            and exported['mode'] == 'headless' and target and exported['target'] == target,
            'Expected a native Mac headless export')
    require(exported['startup_source_parsing'] is False and exported['build_storage_removed_before_startup'] is True,
            'Export retains source parsing or build storage')
    profile = exported['profiling']
    require(profile['mode'] == 'summary' and profile['compiler']['profile'] == 'shipping'
            and profile['compiler']['target'] == target and profile['runtime_opt_in'] == '--profile',
            'Expected summary profiling in shipping configuration')
    build = read(package / 'build.log', 128 * 1024**2)
    flags = verify_codegen(build.decode(), target, 'packed')
    require(flags == exported['codegen'], 'Shipping compiler log differs from receipt')
    player = package / 'player/incant_game_player'
    require(digest(read(player, 256 * 1024**2)) == exported['player_sha256'], 'Player bytes differ from export')
    expected = exported['data']['files']
    actual = {}
    for path in sorted((package / 'player/data').rglob('*')):
        require(not path.is_symlink(), 'Linked runtime data')
        if path.is_file():
            actual[path.relative_to(package / 'player/data').as_posix()] = digest(read(path, 256 * 1024**2))
    require(actual == expected, 'Runtime data differs from export')
    build_info = json.loads(read(package / 'project-build.json', 16 * 1024**2))
    rate = integer(build_info['tick_rate'], 'tick rate', 1)
    require(rate <= 240 and build_info['project_sha256'] == exported['project_sha256']
            and build_info['target'] == target and build_info['startup_source_parsing'] is False,
            'Producer identity differs')
    return player, exported, {'export_sha256': digest(exported_bytes), 'build_log_sha256': digest(build),
                              'player_sha256': exported['player_sha256'], 'data': actual,
                              'script_source_sha256': build_info['script']['source_sha256'],
                              'program': build_info['program']}, rate


def measure(package, output, ticks, recording=None, observe=None, baseline=None, *,
            script_phase='script.tick', heap_mode=None, process_timeout=600, cancel=None):
    require(heap_mode is None or type(heap_mode) is str and heap_mode in HEAP_MODES,
            'Unknown requested heap mode')
    integer(process_timeout, 'process timeout', 1)
    require(process_timeout <= 600, 'Process timeout exceeds 600 seconds')
    require(cancel is None or callable(cancel), 'Cancellation check must be callable')
    if cancel is not None:
        cancel()
    machine = mac_identity()
    if cancel is not None:
        cancel()
    package = package.absolute()
    player, exported, identities, rate = package_inputs(package)
    if cancel is not None:
        cancel()
    phase_selection(script_phase, identities['program'])
    require(baseline is None or script_phase == 'script.tick',
            'Static-module timing has no TypeScript regression baseline policy')
    ui_options = ui_startup_options('headless', exported.get('startup_ui_viewport'))
    integer(ticks, 'ticks', 100)
    require(ticks <= 300 * rate, 'Measurement is limited to 300 simulated seconds')
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(), 'Choose a new output directory')
    output = output.absolute()
    require(package != output and package not in output.parents and output not in package.parents, 'Package/output overlap')
    inputs = {}
    for name, path in [('input', recording), ('observe', observe)]:
        if path is not None:
            inputs[name] = read(path.absolute(), 8 * 1024**2)
    match = {'machine': machine, 'target': exported['target'], 'codegen': exported['codegen'],
             'profiling': exported['profiling']['compiler']['features'], 'physics': exported['physics_mode'],
             'compiler': exported['compiler'], 'project_sha256': exported['project_sha256'],
             'script_source_sha256': identities['script_source_sha256'], 'program': identities['program'],
             'ui_options': ui_options,
             'data': identities['data'], 'ticks': ticks, 'tick_rate': rate,
             'inputs': {name: digest(data) for name, data in inputs.items()}, 'protocol': 1}
    # Preserve the existing default-phase baseline identity. Absence means script.tick.
    if script_phase != 'script.tick':
        match['script_phase'] = script_phase
    prior = load_baseline(baseline.absolute()) if baseline else None
    if prior is not None:
        # Actual instrumentation is learned from the retained report after execution.
        # Refuse changed workloads now; compare instrumentation before any verdict.
        prior_match = {k: v for k, v in prior['match'].items() if k != 'profile_instrumentation'}
        require(prior_match == match, 'Baseline machine, workload or build configuration differs')
    output.mkdir()
    for name, data in inputs.items():
        (output / (name + '.json')).write_bytes(data)
    argv = [str(player), '--package', str(player.parent / 'data'), '--ticks', str(ticks),
            '--output', str(output / 'output.pcm'), '--profile']
    if heap_mode == 'detailed':
        argv.append('--profile-heap-detail')
    argv += ui_options
    for name, flag in [('input', '--input-recording'), ('observe', '--observe')]:
        if name in inputs:
            argv += [flag, str(output / (name + '.json'))]
    environment = {'PATH': str(player.parent), 'TMPDIR': str(output)}
    started = time.time_ns()
    receipt = {'argv': argv, 'cwd': str(player.parent), 'environment': environment,
               'started_ns': started, 'load_average_before': os.getloadavg(), 'exit': None,
               'timeout_seconds': process_timeout}
    if heap_mode is not None:
        receipt['requested_heap_mode'] = heap_mode
    try:
        with (output / 'stdout.json').open('xb') as stdout, (output / 'stderr.log').open('xb') as stderr:
            cancellation = {'cancel': cancel} if cancel is not None else {}
            result = native_process.run(argv, cwd=player.parent, env=environment,
                                        stdout=stdout, stderr=stderr, timeout=process_timeout, **cancellation)
        if cancel is not None:
            cancel()
        receipt['exit'] = result.returncode
        require(result.returncode == 0, 'Player failed; retained stdout/stderr contain diagnostics')
        report = json.loads(read(output / 'stdout.json', 16 * 1024**2))
        require(not read(output / 'stderr.log', 1024**2), 'Unexpected player diagnostics')
        require('observe' not in inputs or report['observations']['passed'] is True, 'Game observation failed')
        instrumentation = profile_instrumentation(report)
        if heap_mode is not None:
            require(instrumentation['profile_version'] == 3
                    and instrumentation['quickjs_heap_mode'] == heap_mode,
                    'Returned heap instrumentation differs from requested mode')
        values = metrics(report, ticks, script_phase=script_phase)
        require(package_inputs(package)[2] == identities, 'Package changed during execution')
        if cancel is not None:
            cancel()
        require(mac_identity() == machine, 'Mac configuration changed during execution')
        if cancel is not None:
            cancel()
        match['profile_instrumentation'] = instrumentation
        result = {'format': 'incant-mac-cpu-measurement', 'version': 2, 'match': match,
                  'identities': identities, 'metrics': values, 'script_phase': script_phase,
                  'profile_report_sha256': digest(read(output / 'stdout.json', 16 * 1024**2)),
                  'phase_gate_passed': False,
                  'limits': ['CPU simulation timing; no display fps, GPU time or shader-stall qualification',
                             'Memory metric is requested Rust allocations, not process RSS or GPU memory',
                             'Workload identity does not establish Core Sample or ten thousand scripted entities',
                             'Single process sample, no warmup exclusion; compare matched repeated runs',
                             'Mobile and other-device execution deferred; no device inference']}
        result['comparison'] = comparison(result, prior) if prior else None
        write_json(output / 'measurement.json', result)
        return result
    except BaseException as error:
        receipt['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        receipt['ended_ns'] = time.time_ns()
        receipt['load_average_after'] = os.getloadavg()
        for name in ('stdout.json', 'stderr.log'):
            if (output / name).exists():
                receipt[name + '_sha256'] = digest(read(output / name, 16 * 1024**2))
        write_json(output / 'command.json', receipt)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--ticks', type=int, default=3600)
    parser.add_argument('--input-recording', type=Path)
    parser.add_argument('--observe', type=Path)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--script-phase', choices=SCRIPT_PHASES, default='script.tick')
    parser.add_argument('--heap-mode', choices=HEAP_MODES,
                        help='Require explicit profile v3 heap mode; omission preserves legacy ingestion')
    parser.add_argument('--process-timeout', type=int, default=600)
    args = parser.parse_args()
    measured = measure(args.package, args.output, args.ticks, args.input_recording, args.observe, args.baseline,
                       script_phase=args.script_phase, heap_mode=args.heap_mode,
                       process_timeout=args.process_timeout)
    print(json.dumps({'measurement': str(args.output / 'measurement.json'), 'comparison': measured['comparison']}))
