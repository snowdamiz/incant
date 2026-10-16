"""Bounded, portable data replay. Does not execute archived source or native artifacts."""
import argparse
import gzip
import json
from pathlib import Path, PurePosixPath

from build_audit import audit
from oracle import GUEST, HEAD, PREPARED, TREE, REFUSALS, authoring, equal, require, run, sha

ROOT = Path(__file__).resolve().parent
MAX_ARCHIVE = 8 * 1024**2
MAX_DECODED = 64 * 1024**2


def strict_json(text):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result
    def constant(_):
        raise ValueError('Non-finite JSON number')
    return json.loads(text, object_pairs_hook=pairs, parse_constant=constant)


def entry(text):
    payload = text.encode('utf-8')
    return {'bytes': len(payload), 'sha256': sha(payload), 'text': text}


def validate_records(manifest, records):
    equal(sorted(records), sorted(manifest['records']), 'Complete record inventory')
    for name, record in records.items():
        path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts and str(path) == name,
                'Safe inert record name')
        equal(record, entry(record['text']), 'Raw record bytes/hash')
        equal({key: record[key] for key in ('bytes', 'sha256')}, manifest['records'][name],
              'Manifest record binding')


def replay(records):
    def text(name):
        return records[name]['text']
    def data(name):
        return strict_json(text(name))
    def bound(name, identity):
        equal({k: records[name][k] for k in ('bytes', 'sha256')},
              {k: identity[k] for k in ('bytes', 'sha256')}, 'Cross-record byte binding')
    source = data('source.json')
    equal([source['head'], source['tree']], [HEAD, TREE], 'Frozen producer identity')
    require(len(source['files']) == 22152, 'Whole producing source inventory')
    packet = data('packet.json')
    for name, identity in packet['files'].items():
        bound(name, identity)
    for name in records:
        if name.startswith('source/'):
            bound(name, source['files'][name.removeprefix('source/')])
    for name in ('tools/export_native.py', 'tools/probes/native_wasm_restart/route.py',
                 'tools/probes/native_wasm_restart/owner.py', 'sdk/rust/samples/session_restart.rs'):
        require('source/' + name in records, 'Retained ordinary route/guest source')
    # Recheck guest/SDK provenance, including the original dirty 25-file host qualification.
    sdk = data('provenance/rust-sdk.json')['source_sha256']
    qualified = data('provenance/source.json')['files']
    equal([len(sdk), len(qualified)], [36, 25], 'Complete reused source provenance')
    for name, digest in [*sdk.items(), *qualified.items()]:
        equal(source['files'][name]['sha256'], digest, 'Exact qualified SDK/runtime source reuse')
    tools = data('tools.json')
    for name in ('rust-sdk.json', 'prepared-manifest.json', 'source.json'):
        matches = [identity for path, identity in tools.items()
                   if path.endswith('/conformance-001/' + name)]
        require(len(matches) == 1, 'Unique guest qualification record')
        bound('provenance/' + name, matches[0])
    modules = [row for row in data('provenance/prepared-manifest.json')['modules']
               if row['name'] == 'rust-session-once']
    require(len(modules) == 1, 'Unique qualified guest')
    equal([modules[0]['input_sha256'], modules[0]['prepared_sha256']],
          [GUEST, PREPARED], 'Original guest preparation identity')
    owner, closeout = data('evidence/owner.json'), data('closeout.json')
    require(owner['passed'] is True and owner['error'] is None, 'Managed owner success')
    calls = owner['calls']
    require(len(calls) == 27 and len(owner['owned']) == 29, 'All calls and RPC owners')
    for index, call in enumerate(calls):
        equal([call['finished'], call['cleanup_confirmed'], call['exit']],
              [True, True, 1 if index in REFUSALS else 0], 'Actual terminal/cleanup result')
        # Existing symbol-tool adapter merges stderr into stdout for these seven calls.
        streams = ('stdout',) if 15 <= index <= 21 else ('stdout', 'stderr')
        equal(sorted(k for k in ('stdout', 'stderr') if k in call), sorted(streams),
              'Exact recorded stream mode')
        for stream in streams:
            receipt = call[stream]
            equal(receipt['path'], f'{index:03d}.{stream}.log', 'Exact child log identity')
            equal(records['evidence/' + receipt['path']]['sha256'], receipt['sha256'],
                  'Child log binding')
        if index in REFUSALS:
            equal(text(f'evidence/{index:03d}.stderr.log').strip(), REFUSALS[index], 'Exact refusal')
            equal(text(f'evidence/{index:03d}.stdout.log'), '', 'No refusal success output')
    pids = [row['pid'] for row in owner['owned']]
    require(len(set(pids)) == 29 and all(row['cleanup_confirmed'] is True
                                       for row in owner['owned']), 'Owned process cleanup')
    equal(sorted(closeout['checked_pids']), sorted(pids + [owner['wrapper_pid']]),
          'Independent complete absence inventory')
    equal([closeout['actual_exec_terminal_exit'], closeout['all_pids_and_groups_absent'],
           closeout['source_unchanged'], closeout['source_head'], closeout['source_tree']],
          [0, True, True, HEAD, TREE], 'Wrapper/source/absence closeout')
    for name, key in [('evidence/owner.json', 'owner_sha256'), ('evidence/result.json', 'result_sha256'),
                      ('wrapper-result.json', 'wrapper_result_sha256'), ('packet.json', 'packet_sha256')]:
        equal(records[name]['sha256'], closeout[key], 'Closeout record binding')
    equal(data('root-independent-review.json')['raw_owner_sha256'],
          records['evidence/owner.json']['sha256'], 'Independent root review owner')
    equal(data('root-independent-review.json')['exact_report_pcm_and_package_hashes'], True,
          'Independent actual bytes review')
    plan = data('plan.json')
    equal([plan['source_head'], plan['source_tree']], [HEAD, TREE], 'Planned producer')
    equal([plan['target_baseline_bytes'], plan['target_ceiling_bytes'], owner['target_ceiling_bytes']],
          [63801221120, 65948704768, 65948704768], 'Original fixed allocated account')
    limits = {'target': 65948704768, 'scratch': 8 * 1024**3,
              'output': 2 * 1024**3, 'rss': 8 * 1024**3}
    require(all(0 <= owner['peaks'][key] <= limit for key, limit in limits.items()), 'Sampled limits')
    require(0 < closeout['wrapper']['seconds'] <= 4200, 'Bounded wrapper duration')
    equal(closeout['sampled_peaks'], owner['peaks'], 'Sampled peak receipt')
    authoring(data)
    player = audit(text, data)
    # Native argv is evidence, not executable input to this verifier.
    build = calls[13]['argv']
    require(build[1] == 'build' and '--profile' in build and
            build[build.index('--profile') + 1] == 'shipping' and '--no-default-features' in build,
            'Fresh ordinary shipping build command')
    equal(build[build.index('--features') + 1],
          'project,render-input,physics-deterministic,static-module', 'No conformance shortcut')
    equal(build[build.index('--target-dir') + 1] + '/aarch64-apple-darwin/shipping/incant_game_player',
          player['executable'], 'Fresh command/artifact destination')
    result = data('evidence/result.json')
    equal(result['deleted_private_files'], data('evidence/deleted-0.json') | data('evidence/deleted-1.json'),
          'Complete preserved deletion inventory')
    equal([closeout['private_export_scratch_empty'], closeout['private_authoring_and_tools_absent'],
           result['guest_recompiled']], [True, True, False], 'Source-free/reused guest scope')
    reports = []
    for index in range(3):
        report = data(f'evidence/run-{index}.json')
        pcm = bytes.fromhex(text(f'pcm/run-{index}.hex').strip())
        run(report, pcm)
        equal(report, data(f'evidence/{index + 23:03d}.stdout.log'), 'Actual child report')
        bound(f'evidence/run-{index}.json', closeout['runs'][index]['report'])
        equal({'bytes': len(pcm), 'sha256': sha(pcm), 'executable': False},
              closeout['runs'][index]['pcm'], 'Actual PCM binding')
        argv = calls[23 + index]['argv']
        equal(argv[1::2], ['--package', '--ticks', '--input-recording', '--observe', '--output'],
              'Ordinary package invocation')
        equal(argv[4], '3', 'Three requested ticks')
        reports.append(report)
    equal(reports, [reports[0]] * 3, 'Exact fresh-process parity')
    require(len({calls[i]['pid'] for i in (23, 24, 25)}) == 3, 'Three distinct runtime processes')
    return {'producer': HEAD, 'tree': TREE, 'fresh_processes': 3, 'restarts_each': 3,
            'additional_exporter_startup_restarts': 1, 'pcm_bytes_each': 19200,
            'managed_calls': 27, 'owned_groups': 29, 'refusals': ['native-play', 'static-checkpoint'],
            'shipping_export': True, 'window_gpu_device_audio': False,
            'performance_or_phase_gate': False, 'new_native_execution': False}


def verify(root=ROOT):
    manifest = strict_json((root / 'manifest.json').read_text())
    archive = root / 'records.json.gz'
    require(archive.stat().st_size <= MAX_ARCHIVE, 'Compressed archive bound')
    payload = archive.read_bytes()
    equal({'bytes': len(payload), 'sha256': sha(payload)}, manifest['archive'], 'Archive hash')
    with gzip.open(archive, 'rb') as stream:
        decoded = stream.read(MAX_DECODED + 1)
    require(len(decoded) <= MAX_DECODED, 'Expanded archive bound')
    records = strict_json(decoded)['records']
    validate_records(manifest, records)
    for name, identity in manifest['verifier'].items():
        require(name in ('verify.py', 'oracle.py', 'build_audit.py', 'test_verify.py'), 'Verifier file')
        payload = (root / name).read_bytes()
        equal({'bytes': len(payload), 'sha256': sha(payload)}, identity, 'Verifier identity')
    summary = replay(records)
    equal(summary, manifest['result'], 'Scoped derived result')
    return manifest, records, summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    arguments = parser.parse_args()
    print(json.dumps(verify(arguments.root)[2], sort_keys=True))
