#!/usr/bin/env python3
"""Bounded inert Core PGO evidence replay; never runs archived code or players."""
import argparse
import gzip
import hashlib
import io
import json
from fractions import Fraction
from pathlib import Path, PurePosixPath
import re
import subprocess

MANIFEST_CAP = 1024 * 1024
PACKED_CAP = 8 * 1024**2
EXPANDED_CAP = 32 * 1024**2
RECORD_CAP = 4 * 1024**2
PRODUCER = 'af775fd1c43318ddaa38ce61628e5f4e7b8b4275'
NATIVE = '4ed9ae187c94c5c9e195d9c73a0726e0940afce9'
ORDER = ('O0', 'U0', 'U1', 'O1', 'U2', 'O2', 'O3', 'U3')
PHASES = ('player.frame', 'script.tick')


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def bounded(path, cap):
    with Path(path).open('rb') as stream:
        raw = stream.read(cap + 1)
    require(len(raw) <= cap, 'Input exceeds bound: ' + str(path))
    return raw


def decode(raw):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result

    def nonfinite(value):
        raise ValueError('Nonfinite JSON: ' + value)
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=nonfinite)


def load(root):
    manifest = decode(bounded(root / 'manifest.json', MANIFEST_CAP))
    packed = bounded(root / 'records.json.gz', PACKED_CAP)
    require(len(packed) == manifest['archive']['bytes'] and digest(packed) == manifest['archive']['sha256'], 'Packed identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as stream:
        expanded = stream.read(EXPANDED_CAP + 1)
    require(len(expanded) <= EXPANDED_CAP and len(expanded) == manifest['expanded']['bytes']
            and digest(expanded) == manifest['expanded']['sha256'], 'Expanded identity/bound')
    rows = decode(expanded)
    require(len(rows) == len(manifest['records']) <= 512, 'Record count')
    records = {}
    for row, binding in zip(rows, manifest['records'], strict=True):
        name = row['name']
        require(isinstance(name, str) and '\\' not in name and not PurePosixPath(name).is_absolute()
                and '..' not in PurePosixPath(name).parts, 'Record path')
        raw = row['text'].encode('utf-8')
        require(name not in records and name == binding['name'], 'Record membership')
        require(len(raw) <= RECORD_CAP and len(raw) == binding['bytes'] and digest(raw) == binding['sha256'], 'Record identity')
        records[name] = raw
    return manifest, records


def interval(row, ticks):
    counts = row['counts']
    require(len(counts) == 10001 and all(type(n) is int and n >= 0 for n in counts), 'Histogram shape')
    require(sum(counts) == row['samples'] and row['samples'] + row['missing_frames'] + row['invalid_frames'] == ticks
            and row['overflow_samples'] == 0, 'Histogram coverage')
    if row['samples'] == 0:
        require(row['p95_lower_ns'] is None and row['p95_upper_ns'] is None, 'Empty quantile')
        return None
    rank, total = (95 * row['samples'] + 99) // 100, 0
    for index, count in enumerate(counts):
        total += count
        if total >= rank:
            low, high = (0, 0) if index == 0 else ((index - 1) * 10000 + 1, index * 10000)
            require((low, high) == (row['p95_lower_ns'], row['p95_upper_ns']), 'Raw quantile mismatch')
            return low, high
    raise ValueError('Missing quantile')


def cleanup(calls, count):
    require(len(calls) == count and len({c['leader_pid'] for c in calls}) == count, 'Managed call membership')
    for call in calls:
        require(call['call_finished'] and call['native_process_returned'] and call['cleanup_confirmed']
                and not call['cleanup_failures'] and call['exit_code'] == 0, 'Actual call cleanup/exit')


def validate(manifest, records):
    get = lambda name: decode(records[name])
    require(manifest['format'] == 'incant-Core-C-PGO-portable-evidence' and manifest['version'] == 1
            and manifest['scope'] == 'default-off-Core-functional-feasibility-and-mixed-matched-CPU-timing', 'Scope')
    require(manifest['producer_source'] == PRODUCER and manifest['native_qualified_source'] == NATIVE
            and manifest['ordinary_is_default'] is True and manifest['adopted'] is False
            and manifest['script_2ms_gate_passed'] is False, 'Producer/default/adoption scope')
    summary = get('core-integration-001/integration-summary.json')
    require(summary['source'] == PRODUCER and summary['native_qualified_source'] == NATIVE
            and summary['ordinary_is_default'] and not summary['adopted'] and not summary['script_2ms_gate_passed'], 'Summary scope')
    for number in range(1, 7):
        old = get(f'core-shipping-{number:03}/metadata/result.json')
        close = get(f'core-shipping-{number:03}/closeout-001.json')
        require(old['status'] == 'failed' and old['exit'] != 0 and old['error'], 'Historical producer failure changed')
        require(close.get('actual_terminal', close.get('actual_terminal_exit', 1)) != 0, 'Historical closeout changed')
    # Qualification results remain tied to their own sources, not relabeled af775.
    cpu = get('core-cpu-007/result.json')
    require(cpu['status'] == 'complete' and cpu['exit'] == 0 and len(cpu['stages']) == 2
            and all(s['exit'] == 0 for s in cpu['stages']), 'Actual role fixture/lint')
    owner = get('core-cpu-012/closeout.json')
    require(owner['source'] == NATIVE and owner['actual_terminal_exit'] == 0
            and owner['strict_selected_script_lib_build_script_lint_exit'] == 0
            and owner['managed_cleanup_confirmed'] is True, 'Actual final owner/lint qualification')
    require(len(owner['two_actual_new_source_custom_build_artifacts']) == 2
            and all(r['fresh'] is False for r in owner['two_actual_new_source_custom_build_artifacts']), 'Fresh actual script owners')
    result = get('core-shipping-007/metadata/result.json')
    close = get('core-shipping-007/closeout-001.json')
    shipped = get('core-shipping-007/packages/manifest.json')
    require(result['status'] == 'complete' and result['exit'] == 0 and result['source'] == PRODUCER
            and result['supervisor_terminal'] is True, 'Functional producer success')
    producer_calls = get('core-shipping-007/metadata/managed-calls.json')
    cleanup(producer_calls, 146)
    require(close['actual_exec'] == 41416 and close['actual_terminal'] == 0 and close['managed_calls'] == 146
            and close['all_build_input_scratch_removed'] and close['ordinary_use_sourcefree_guards_passed']
            and close['generation_nonshipping'] and close['no_performance_adoption_GPU_window_claim'], 'Functional closeout/scope')
    require(shipped['complete'] and shipped['full_logical_and_PCM_parity'] and shipped['no_performance_claim']
            and shipped['GPU_window_capture_executions'] == 0 and shipped['source'] == PRODUCER
            and shipped['routes'] == {'win': 2156, 'loss': 900, 'restart': 3056}, 'Functional manifest')
    receipts = {mode: get(f'core-shipping-007/packages/{mode}/build-receipt.json') for mode in ('ordinary', 'generate', 'use')}
    require(len({json.dumps(r['data'], sort_keys=True) for r in receipts.values()}) == 1, 'Exact raw packaged-data parity')
    external = {row['path']: row for row in manifest['external_artifacts']}
    package_root = Path(close['players']['ordinary']['path']).parents[2]

    def pcm_identity(path, report, ticks):
        row = external[str(path)]
        require(row['bytes'] == ticks * 6400 and row['sha256'] == report['pcm_sha256'], 'Whole PCM identity')

    for mode, receipt in receipts.items():
        require(receipt['startup_source_parsing'] is False and receipt['physics_mode'] == receipt['physics_variant'] == 'fast', 'Source-free parsing/scalar baseline')
        require(receipt['player_sha256'] == close['players'][mode]['sha256']
                and external[close['players'][mode]['path']] == close['players'][mode], 'Actual player identity')
        instrument, link = receipt['instrumentation'], receipt['actual_final_link']
        graph = receipt['pgo_feature_graph']
        selected = set() if mode == 'ordinary' else {'diagnostic-quickjs-' + mode}
        require(len(graph) == 7 and len({(r['package'], r['role'], r['kind']) for r in graph}) == 7
                and all(set(r['features']) & {'diagnostic-quickjs-generate', 'diagnostic-quickjs-use'} == selected for r in graph), 'Actual PGO feature graph')
        if mode == 'generate':
            require(receipt['training_is_nonshipping'] and not receipt['eligible_for_distribution']
                    and not receipt['source_free_claim'] and not receipt['performance_claim'], 'Generation is nonshipping')
            require(not link['dead_strip'] and link['profile_runtime'] and link['bundled_runtime'], 'Training-only linkage')
            require(instrument['profile_sections'] and instrument['profile_runtime_symbols']
                    and instrument['identical_code_initializer_and_profile_sections'], 'Stage-specific instrumentation')
            pre, post = instrument['symbol_artifact'], instrument['packaged_artifact']
            require(pre['uuid'] == post['uuid'] and pre['sections'] == post['sections']
                    and post['sha256'] == receipt['player_sha256'], 'Prestrip-to-package identity')
            sections = post['sections']
            require(all(sections[name]['bytes'] > 0 for name in ('__text','__mod_init_func','__llvm_prf_cnts','__llvm_prf_data','__llvm_prf_names')), 'Real profile payload')
        else:
            require(link['dead_strip'] and not link['profile_runtime'] and link['bundled_runtime'] is None
                    and not instrument['profile_sections'] and not instrument['profile_runtime_symbols'], 'Ordinary/use shipping-like final linkage')
        if mode != 'ordinary':
            descriptor = get(f'core-shipping-007/packages/{mode}/descriptor.json')
            require(descriptor['version'] == 2 and descriptor['profile_scope'] == 'runtime-only'
                    and descriptor['recipes']['cooker']['optimization'] == 0
                    and descriptor['recipes']['runtime']['optimization'] == 3
                    and descriptor['profile'] == (shipped['profile'] if mode == 'use' else None), 'Actual role profile scope')
            for package in ('incant_script', 'rquickjs-sys'):
                for role in ('cooker', 'runtime'):
                    r = get(f'core-shipping-007/packages/{mode}/{package}-{role}.json')
                    require(r['package'] == package and r['mode'] == mode and r['role'] == role and r['profile_scope'] == 'runtime-only'
                            and r['recipe'] == descriptor['recipes'][role] and r['default_c_flags'] is False
                            and r['identity'] == receipt['pgo_roles']['descriptor_identity']
                            and r['descriptor_sha256'] == digest(records[f'core-shipping-007/packages/{mode}/descriptor.json'])
                            and r['compiler'] == descriptor['compiler']
                            and r['profile'] == (descriptor['profile'] if role == 'runtime' else None), 'Independent actual role receipts')
                    if package == 'rquickjs-sys':
                        require(r['c_unit_profiled'] == (role == 'runtime'), 'Only runtime C unit profiled')
    training = get('core-shipping-007/packages/training/stdout.json')
    expected = {}
    for route, ticks in (('win',2156),('loss',900),('restart',3056)):
        o = get(f'core-shipping-007/packages/ordinary-{route}/stdout.json')
        u = get(f'core-shipping-007/packages/use-{route}/stdout.json')
        require(o == u and o['observations']['passed'] and not o['render_input']['gpu_submitted'], 'Full gameplay/PCM route parity')
        require(o['frames'] == ticks * 800 and o['tick'] == (2156 if route == 'restart' else ticks), 'Tick/sample chronology')
        for mode in ('ordinary', 'use'):
            pcm_identity(package_root / f'{mode}-{route}/audio.pcm', o, ticks)
        if route == 'restart':
            require(o['session']['generation'] == 2 and o['session']['host_tick'] == 3056
                    and o['session']['restarts'][0]['host_step'] == 900, 'Actual restart owner chronology')
        expected[route] = o
    require(training == expected['win'] and len(shipped['training_before_deletion']) == 1
            and len(shipped['training_before_deletion'][0]['raw_inputs']) == 1, 'Training-only input/parity')
    pcm_identity(package_root / 'training/audio.pcm', training, 2156)
    require(shipped['profile'] == close['actual_profile'] and external[shipped['profile']['path']] == shipped['profile'], 'Actual merged profile identity')
    raw = shipped['training_before_deletion'][0]['raw_inputs'][0]
    require(external[raw['path']] == raw, 'Actual sole raw profile identity')
    merges = [c['argv'] for c in producer_calls if Path(c['argv'][0]).name == 'llvm-profdata' and c['argv'][1:2] == ['merge']]
    require(len(merges) == 1 and merges[0][-1:] == [raw['path']] and '--failure-mode=any' in merges[0]
            and merges[0][merges[0].index('-j') + 1] == '2' and '--sparse' not in merges[0], 'Strict training-only profile merge')
    covered = records['core-shipping-007/packages/profile-covered.stdout'].decode().splitlines()
    require(len(covered) == len(set(covered)) and {'quickjs/quickjs.c;JS_CallInternal', 'JS_ReadObject', 'JS_EvalFunction'} <= set(covered), 'Actual relative covered names')
    old = get('core-timing-001/prelaunch-closeout.json')
    require(old['actual_exec'] == 29495 and old['actual_terminal'] == 1 and old['managed_native_children'] == old['timing_samples'] == 0
            and old['wrapper_PID_observed'] is False and old['actual_wrapper_PID'] is None
            and old['conservative_reserved_seconds_not_measured'] == 5 and old['failed_attempt_time_measured'] is False, 'First wrapper failure preserved')
    timing = get('core-timing-002/metadata/result.json')
    timing_plan = get('core-timing-packet-002/plan.json')
    require(timing['status'] == 'complete' and timing['exit'] == 0 and timing['order'] == list(ORDER)
            and timing['no_adoption_or_phase_approval'], 'Timing completion/scope')
    calls = get('core-timing-002/metadata/managed-calls.json'); cleanup(calls,80)
    for call in calls:
        if call['index'] in {8 + 9*i for i in range(8)}:
            continue
        for name in ('stdout', 'stderr'):
            metadata = call[name]
            raw = records['core-timing-002/metadata/' + Path(metadata['path']).name]
            require(len(raw) == metadata['bytes'] and digest(raw) == metadata['sha256'], 'Actual metadata stream')
    for offset in (0, 9, 18, 27, 36, 45, 54, 63, 72):
        raw = [records['core-timing-002/metadata/' + Path(c['stdout']['path']).name].decode().strip() for c in calls[offset:offset+8]]
        fields = dict(zip(('model','chip','memory_bytes','physical_cpus','os_build','os_version'), raw[:6], strict=True))
        require(fields == {k:timing['machine'][k] for k in fields} and "Now drawing from 'AC Power'" in raw[6]
                and not any('lowpowermode' in line.strip().lower() for line in raw[7].splitlines()), 'Matched saved Mac/power identity')
    absent = get('core-timing-002/fresh-owned-absence-001.json')
    require(absent['observed_identities'] == len(absent['rows']) == 81
            and all(r['pid_absent'] and r['group_absent'] for r in absent['rows']), 'Fresh observed absence')
    values, pcm_bytes, raw_phase_rows = {}, 0, 0
    for index, label in enumerate(ORDER):
        route = 'win' if int(label[1]) % 2 == 0 else 'loss';ticks = 2156 if route == 'win' else 900
        r = get(f'core-timing-002/cases/{label}/stdout.json');p = r['profile'];f = p['frame_wall']
        require({k:v for k,v in r.items() if k!='profile'} == expected[route], 'Timed full gameplay/PCM parity')
        pcm_identity(Path(timing_plan['output']) / f'cases/{label}/audio.pcm', r, ticks)
        require(p['version']==3 and p['completed_frames']==ticks and p['failed_frames']==p['dropped_source_samples']==0
                and p['tracy_process_transport'] is False, 'Complete timed profile')
        heap = p['quickjs_heap']
        require(heap['mode']=='maintained' and heap['sampling']=='script_tick_end_per_vm' and heap['samples']==ticks
                and heap['detailed_samples']==heap['invalid_samples']==0, 'Matched maintained instrumentation')
        require(f['version']==1 and f['unit']=='ns' and f['bucket_width_ns']==10000 and f['max_ns']==100000000
                and f['attempted_frames']==f['successful_frames']==ticks
                and f['failed_frames']==f['invalid_frames']==f['active_frames']==0 and f['saturated'] is False, 'Frame coverage')
        phase = {row['name']: row for row in f['phases']}
        require(len(phase)==len(f['phases']), 'Duplicate phase')
        raw_phase_rows += len(phase)
        for row in phase.values():interval(row,ticks)
        values[label] = {}
        for name in PHASES:
            row = phase[name]
            require(row['samples']==ticks and row['missing_frames']==row['invalid_frames']==row['overflow_samples']==0, 'Required phase coverage')
            values[label][name] = interval(row,ticks)
        require(all(row['totals']['calls']==ticks and row['totals']['unwound']==row['totals']['unavailable_cpu_samples']==0 for row in p['scripts'])
                and len(p['scripts'])==2, 'Source callback context')
        require(calls[8+9*index]['argv']==timing['attempts'][index]['argv'], 'Actual timing arm order')
        pcm_bytes += ticks*6400
    for i in range(4):
        for phase in PHASES:
            u, o = values['U'+str(i)][phase], values['O'+str(i)][phase]
            ratio = timing['paired_p95_ratios'][str(i)][phase+'.p95_wall_ns']
            require(Fraction(*ratio['lower_fraction']) == Fraction(u[0],o[1])
                    and Fraction(*ratio['upper_fraction']) == Fraction(u[1],o[0]), 'Actual raw paired ratio')
    require(raw_phase_rows == 208, 'Raw phase catalog membership')
    require(all(values[label]['script.tick'][0] > 2000000 for label in ('O0','U0','U2','O2')), 'No win2ms claim')
    require(values['U0']['script.tick'][1] < values['O0']['script.tick'][0]
            and values['U2']['script.tick'][0] > values['O2']['script.tick'][1], 'Mixed win result retained')
    require(summary['source_controls']==42 and summary['source_control_exit']==0
            and b'Ran 42 tests' in records['core-integration-001/source-controls.stderr']
            and records['core-integration-001/source-controls.stderr'].rstrip().endswith(b'OK'), 'Portable source controls')
    return {'records':len(records),'shipping_attempts_failed':6,'coherent_producer_calls':146,'timing_calls':80,
            'timing_arms':8,'timed_frames':12224,'timed_PCM_bytes':pcm_bytes,'raw_phase_rows':208,
            'pure_source_controls':42,'all_gameplay_and_PCM_report_parity':True,'repeatable_win_improvement':False,'adopted':False}


def originals(manifest, records):
    for row in manifest['records']:
        require(bounded(row['original'],row['bytes'])==records[row['name']], 'Original changed')


def artifacts(manifest):
    for row in manifest['external_artifacts']:
        h=hashlib.sha256();size=0
        with Path(row['path']).open('rb') as stream:
            while block:=stream.read(1024*1024):
                size+=len(block);require(size<=row['bytes'],'External size bound');h.update(block)
        require(size==row['bytes'] and h.hexdigest()==row['sha256'],'External artifact identity')


def git_source(manifest, repo, revision):
    require(re.fullmatch('[0-9a-f]{40}',revision) is not None and revision==manifest['producer_source'], 'Exact producing source required')
    for row in manifest['source_files']:
        raw=subprocess.check_output(['git','-C',str(repo),'show',revision+':'+row['path']])
        require(len(raw)==row['bytes'] and digest(raw)==row['sha256'],'Git source mismatch')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals',action='store_true');parser.add_argument('--artifacts',action='store_true')
    parser.add_argument('--git-source',metavar='EXACT_PRODUCER');parser.add_argument('--repo',type=Path,default=Path.cwd())
    args=parser.parse_args();manifest,records=load(Path(__file__).parent);result=validate(manifest,records)
    if args.originals:originals(manifest,records)
    if args.artifacts:artifacts(manifest)
    if args.git_source:git_source(manifest,args.repo,args.git_source)
    print(json.dumps(result,sort_keys=True))


if __name__=='__main__':main()
