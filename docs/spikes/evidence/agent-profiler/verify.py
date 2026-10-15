"""Read-only agent profiler CPU evidence replay; never executes native artifacts."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def unique(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError('duplicate JSON key: ' + key)
        out[key] = value
    return out


def parse(data):
    return json.loads(data, object_pairs_hook=unique)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--originals', action='store_true')
    ap.add_argument('--binaries', action='store_true')
    ap.add_argument('--git-source', action='store_true')
    args = ap.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[3]
    manifest = parse((here / 'manifest.json').read_bytes())
    assert manifest['format'] == 'incant-agent-profiler-cpu' and manifest['version'] == 1
    archive = (here / 'records.json.gz').read_bytes()
    meta = manifest['archive']
    assert len(archive) <= 8 * 1024**2 and len(archive) == meta['bytes']
    assert sha(archive) == meta['sha256']
    with gzip.GzipFile(fileobj=io.BytesIO(archive)) as stream:
        raw = stream.read(32 * 1024**2 + 1)
    assert len(raw) <= 32 * 1024**2 and len(raw) == meta['inflated_bytes']
    rows = parse(raw)
    assert len(rows) == meta['records']
    records = {}
    for row in rows:
        path = PurePosixPath(row['path'])
        assert path.parts and not path.is_absolute() and '..' not in path.parts
        assert row['path'] not in records
        data = base64.b64decode(row['base64'], validate=True)
        assert len(data) <= 16 * 1024**2 and len(data) == row['bytes']
        assert sha(data) == row['sha256']
        records[row['path']] = data
        if args.originals:
            assert file_sha(Path(manifest['original_root']) / row['path']) == row['sha256']
    for path, expected in manifest['bindings'].items():
        assert sha(records[path]) == expected, path
    freeze = parse(records['source-001/freeze.json'])
    assert freeze['source'] == manifest['source'] and freeze['base'] == manifest['base']
    assert re.fullmatch('[0-9a-f]{40}', freeze['source']) and re.fullmatch('[0-9a-f]{40}', freeze['base'])
    assert len(freeze['rows']) == 45 and len(freeze['changed_paths']) == 21
    assert len({r['path'] for r in freeze['rows']}) == len(freeze['rows'])
    for row in freeze['rows']:
        path = PurePosixPath(row['path'])
        assert path.parts and not path.is_absolute() and '..' not in path.parts
        data = records['source-001/inputs/' + row['path']]
        assert sha(data) == row['sha256'] and len(data) == row['size']
        if args.git_source:
            actual = subprocess.check_output(['git', 'show', freeze['source'] + ':' + row['path']], cwd=root, timeout=15)
            assert sha(actual) == row['sha256'], row['path']
    plan = parse(records['cpu-plan-001.json'])
    grant = parse(records['root-cpu-grant-001.json'])
    result = parse(records['cpu-001/result.json'])
    assert plan['source'] == grant['source'] == result['source'] == manifest['source']
    assert plan['freeze_sha256'] == sha(records['source-001/freeze.json'])
    assert grant['accepted_for_cpu']
    assert result['plan_sha256'] == grant['plan_sha256'] == sha(records['cpu-plan-001.json'])
    assert result['runner_sha256'] == grant['runner_sha256'] == sha(records['qualify-001.py'])
    assert result['native_process_sha256'] == sha(records['native_process.py'])
    assert plan['jobs'] == plan['rayon_threads'] == 2 and plan['stop_first_failure']
    assert not plan['gpu_or_window']
    assert result['outcome'] == 'success' and result['error'] is None
    assert not any(result[k] for k in ('managed_call_active', 'gpu_execution', 'window_execution', 'listener_execution', 'native_player_execution'))
    assert result['elapsed_seconds'] <= plan['overall_seconds']
    assert result['minimum_sampled_free_bytes'] >= plan['minimum_free_bytes']
    assert result['target_initial_bytes'] == plan['cumulative_target_initial_bytes']
    assert result['target_last_sampled_bytes'] - result['target_initial_bytes'] <= plan['maximum_target_growth_bytes']
    commands = result['commands']
    assert len(commands) == len(plan['steps']) + 1 == 12 and commands[0]['name'] == 'rust-version'
    counts = {}
    copies = []
    for command in commands:
        assert command['exit_code'] == 0 and command['managed_returned_after_cleanup'] and 'error' not in command
        for stream in ('stdout', 'stderr'):
            data = records[f"cpu-001/{command['name']}.{stream}"]
            assert sha(data) == command[stream + '_sha256']
            assert len(data) <= plan['maximum_stream_bytes']
        copies.extend(command.get('artifacts', []))
    for step, command in zip(plan['steps'], commands[1:]):
        assert step['name'] == command['name'] and step['argv'] == command['argv']
        if step.get('tests'):
            log = records[f"cpu-001/{step['name']}.stdout"].decode()
            summaries = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored', log)
            assert summaries and all(s[0] == 'ok' and s[2:] == ('0', '0') for s in summaries)
            counts[step['name']] = sum(int(s[1]) for s in summaries)
    assert counts == {'session-profile-on': 21, 'session-profile-off': 2, 'agent-transport': 69, 'native-profile-on': 11, 'native-profile-off': 9}
    for step, required in {
        'session-profile-on': ['only_owner_tick_collects_real_phases_and_source_identities', 'simultaneous_thread_captures_do_not_share_sources_or_frames', 'bounded_ring_reads_exact_suffix_and_exposes_eviction', 'failure_and_unwind_remain_invalid_and_retire_tls', 'largest_recording_has_a_bounded_response_separate_from_retained_storage'],
        'native-profile-on': ['real_measurements_are_project_bound_read_only_and_publish_with_successful_game', 'failed_cancelled_and_snapshot_rejected_candidates_preserve_previous_measurements', 'stopping_the_controller_retires_its_measurements'],
        'native-profile-off': ['ordinary_host_refuses_profile_before_running_and_retains_normal_play'],
    }.items():
        text = records[f'cpu-001/{step}.stdout'].decode()
        for name in required:
            assert re.search(r'test [^\n]*' + re.escape(name) + r' \.\.\. ok', text), name
    for step, enabled in [('native-profile-on', True), ('native-profile-off', False)]:
        artifacts = []
        for line in records[f'cpu-001/{step}.stdout'].splitlines():
            try:
                event = parse(line)
            except (ValueError, UnicodeDecodeError):
                continue
            if isinstance(event, dict) and event.get('reason') == 'compiler-artifact':
                artifacts.append(event)
        for target, feature in [('incant_profile', 'session'), ('incant_runtime', 'profile'), ('incant_script', 'profile'), ('incant_game_runtime', 'profile'), ('incant_agent_native', 'agent-profiler')]:
            units = [a for a in artifacts if a['target']['name'] == target]
            assert units and all((feature in a['features']) == enabled for a in units), (step, target)
    assert len(copies) == len(manifest['copied_binaries']) == 14
    assert [(r['path'], r['sha256']) for r in copies] == [(r['path'], r['sha256']) for r in manifest['copied_binaries']]
    if args.binaries:
        for row in manifest['copied_binaries']:
            path = Path(row['path'])
            assert path.stat().st_size == row['bytes'] and file_sha(path) == row['sha256']
    # The original producer remains separately replayed above. The joined source
    # adds main playbooks/renderer ancestry, with no collector algorithm change.
    prefix = 'integration-main333-001/'
    integration = manifest['integration']
    joined = parse(records[prefix + 'source-001/freeze.json'])
    composition = parse(records[prefix + 'composition.json'])
    assert joined['source'] == integration['source'] == composition['source']
    assert joined['base'] == integration['main'] == composition['main']
    assert joined['producer'] == manifest['source']
    assert re.fullmatch('[0-9a-f]{40}', joined['source'])
    assert len(joined['rows']) == 68 and len({r['path'] for r in joined['rows']}) == 68
    for row in joined['rows']:
        path = PurePosixPath(row['path'])
        assert path.parts and not path.is_absolute() and '..' not in path.parts
        data = records[prefix + 'source-001/inputs/' + row['path']]
        assert sha(data) == row['sha256'] and len(data) == row['size']
        if args.git_source:
            actual = subprocess.check_output(['git', 'show', joined['source'] + ':' + row['path']], cwd=root, timeout=15)
            assert sha(actual) == row['sha256'], row['path']
    assert len(composition['unchanged_qualified_implementation_paths']) == 19
    for path in composition['unchanged_qualified_implementation_paths']:
        assert records[prefix + 'source-001/inputs/' + path] == records['source-001/inputs/' + path]
    lib = records[prefix + 'source-001/inputs/crates/incant_agent/src/lib.rs'].decode()
    for addition in ['mod skills;\n', '    tools.extend(skills::tools());\n', '        "skill_list" | "skill_read" => skills::dispatch(name, args),\n']:
        assert lib.count(addition) == 1
        lib = lib.replace(addition, '')
    assert lib.encode() == records['source-001/inputs/crates/incant_agent/src/lib.rs']
    ip = parse(records[prefix + 'cpu-plan.json'])
    ig = parse(records[prefix + 'root-cpu-grant.json'])
    ir = parse(records[prefix + 'cpu-001/result.json'])
    assert ip['source'] == ig['source'] == ir['source'] == joined['source']
    assert ip['freeze_sha256'] == sha(records[prefix + 'source-001/freeze.json'])
    assert ig['accepted_for_cpu']
    assert ir['plan_sha256'] == ig['plan_sha256'] == sha(records[prefix + 'cpu-plan.json'])
    assert ir['runner_sha256'] == ig['runner_sha256'] == sha(records[prefix + 'qualify.py'])
    assert ir['native_process_sha256'] == sha(records[prefix + 'native_process.py'])
    assert ip['jobs'] == ip['rayon_threads'] == 2 and ip['stop_first_failure'] and not ip['gpu_or_window']
    assert ir['outcome'] == 'success' and ir['error'] is None
    assert not any(ir[k] for k in ('managed_call_active', 'gpu_execution', 'window_execution', 'listener_execution', 'native_player_execution'))
    assert ir['elapsed_seconds'] <= ip['overall_seconds']
    assert ir['minimum_sampled_free_bytes'] >= ip['minimum_free_bytes']
    assert ir['target_initial_bytes'] == ip['cumulative_target_initial_bytes']
    assert ir['target_last_sampled_bytes'] - ir['target_initial_bytes'] <= ip['maximum_target_growth_bytes']
    assert len(ir['commands']) == len(ip['steps']) + 1 == 10
    icounts = {}
    icopies = []
    for command in ir['commands']:
        assert command['exit_code'] == 0 and command['managed_returned_after_cleanup'] and 'error' not in command
        for stream in ('stdout', 'stderr'):
            data = records[prefix + f"cpu-001/{command['name']}.{stream}"]
            assert sha(data) == command[stream + '_sha256'] and len(data) <= ip['maximum_stream_bytes']
        icopies.extend(command.get('artifacts', []))
    for step, command in zip(ip['steps'], ir['commands'][1:]):
        assert step['name'] == command['name'] and step['argv'] == command['argv']
        if step.get('tests'):
            text = records[prefix + f"cpu-001/{step['name']}.stdout"].decode()
            summaries = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored', text)
            assert summaries and all(s[0] == 'ok' and s[2:] == ('0', '0') for s in summaries)
            icounts[step['name']] = sum(int(s[1]) for s in summaries)
    assert icounts == {'joined-agent-tests': 120, 'native-profile-on': 11, 'native-profile-off': 9}
    assert len(icopies) == len(integration['copied_binaries']) == 17
    assert [(r['path'], r['sha256']) for r in icopies] == [(r['path'], r['sha256']) for r in integration['copied_binaries']]
    if args.binaries:
        for row in integration['copied_binaries']:
            path = Path(row['path'])
            assert path.stat().st_size == row['bytes'] and file_sha(path) == row['sha256']
    print(json.dumps(dict(records=len(records), source_inputs=len(freeze['rows']) + len(joined['rows']), producer_commands=len(commands), producer_harness_tests=sum(counts.values()), integration_commands=len(ir['commands']), integration_harness_tests=sum(icounts.values()), copied_binaries=len(copies) + len(icopies), external_binaries_verified=args.binaries, historical_git_source_verified=args.git_source, normal_editor_default_enabled=False, native_window_or_gpu_execution=False, performance_gate=False)))


if __name__ == '__main__':
    main()
