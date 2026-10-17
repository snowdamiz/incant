"""Bounded portable records: never executes archived code or native artifacts."""
import argparse
import base64
import gzip
import json
from pathlib import Path, PurePosixPath
import tempfile

from . import oracle
from .replay import replay_records
from ..native_wasm_restart.verify import equal, require

REPO = Path(__file__).resolve().parents[3]
MANIFEST = REPO / 'docs/spikes/evidence/native-wasm-checkpoint-export.json'
HEAD = 'b733a36623790b076b049763878c06dd5ab09450'
TREE = 'b96bead4d24bdff24029bb80474c19e1aca7fa9f'
CORRECTED = '350702062dbd2b84514dfbe1fe0ef1ec2811e215'
MAX_ARCHIVE, MAX_DECODED = 8 * 1024**2, 96 * 1024**2
VERIFIERS = {'tools/probes/native_wasm_checkpoint/' + name + '.py' for name in
             ('portable', 'replay', 'oracle', 'fixture', 'route')} | {
                 'tools/probes/native_wasm_restart/verify.py', 'tools/export_native.py',
                 'tools/tests/test_native_wasm_checkpoint_portable.py'}


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


def identity(payload):
    return {'bytes': len(payload), 'sha256': oracle.sha(payload)}


def entry(payload, binary=False):
    return identity(payload) | ({'base64': base64.b64encode(payload).decode()} if binary
                                else {'text': payload.decode('utf-8')})


def payload(row):
    require(set(row) in ({'bytes', 'sha256', 'text'}, {'bytes', 'sha256', 'base64'}), 'Record fields')
    value = row['text'].encode() if 'text' in row else base64.b64decode(row['base64'], validate=True)
    equal(identity(value), {k: row[k] for k in ('bytes', 'sha256')}, 'Raw record identity')
    return value


def validate_records(manifest, records):
    equal(sorted(records), sorted(manifest['records']), 'Complete record inventory')
    for name, row in records.items():
        path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts and str(path) == name, 'Safe inert record path')
        require('base64' not in row or (name.startswith('output/') and path.suffix in ('.pcm', '.save')),
                'Only bounded output data encoded as binary')
        equal(identity(payload(row)), manifest['records'][name], 'Manifest record binding')


def provenance(records):
    def data(name):
        return strict_json(payload(records[name]))
    def bound(name, expected):
        equal(identity(payload(records[name])), {k: expected[k] for k in ('bytes', 'sha256')}, 'Cross-record identity')
    original = data('original-records.json')
    for name in records:
        if name in original:
            bound(name, original[name])
    source, corrected = data('source.json'), data('correction/verifier-source.json')
    equal([source['head'], source['tree'], corrected['head']], [HEAD, TREE, CORRECTED], 'Exact source producers')
    require(len(source['files']) == 22220, 'Whole native producer inventory')
    equal(sorted(source['files']), sorted(corrected['files']), 'Unchanged source inventory')
    changed = sorted(name for name in source['files'] if source['files'][name] != corrected['files'][name])
    equal(changed, ['tools/probes/native_wasm_checkpoint/replay.py',
                    'tools/tests/test_native_wasm_checkpoint_export.py'], 'Verifier-only correction')
    for name in records:
        if name.startswith('source/'):
            bound(name, source['files'][name.removeprefix('source/')])
    for name in ('tools/export_native.py', 'tools/shipping_symbols.py', 'tools/probes/native_wasm_restart/owner.py',
                 'tools/probes/native_wasm_checkpoint/route.py', 'tools/probes/native_wasm_checkpoint/guest.rs'):
        require('source/' + name in records, 'Ordinary authored route source retained')
    packet, plan, grant = data('packet.json'), data('plan.json'), data('root-grant.json')
    for name, value in packet['files'].items():
        bound(name, value)
    require(records['packet.json']['sha256'] == grant['packet_sha256'] and grant['CPU_authorized'] is True
            and grant['GPU_authorized'] is False, 'Exact CPU-only grant')
    equal([plan['source_head'], grant['source'], data('output/headless.json')['source_head'],
           data('output/proof.json')['source_head']], [HEAD] * 4, 'One native producing source')
    equal([plan['jobs'], plan['rayon_threads'], plan['aggregate_seconds'], plan['target_ceiling_allocated_bytes']],
          [2, 2, 5400, 12 * 1024**3], 'Original cold allocation')
    equal([plan[name] for name in ('scratch_bytes', 'retained_bytes', 'rss_bytes',
                                  'launch_floor_bytes', 'free_floor_bytes')],
          [n * 1024**3 for n in (8, 2, 8, 28, 6)], 'Original resource account')
    guest = data('output/guest.json')
    require(len(guest['sdk']['source_sha256']) == 39 and guest['sdk']['build_scripts_or_proc_macros'] is False,
            'Complete trusted SDK source route')
    for name, digest in guest['sdk']['source_sha256'].items():
        equal(source['files'][name]['sha256'], digest, 'Guest SDK provenance')
        bound('source/' + name, source['files'][name])
    bound('source/tools/probes/native_wasm_checkpoint/guest.rs', guest['source'])
    equal(guest['sdk']['target'], 'wasm32-unknown-unknown', 'Guest target')
    deleted = data('output/deleted-0.json') | data('output/deleted-1.json')
    equal(deleted['authoring/game.wasm'], guest['artifact'], 'Recorded removed authored guest')
    bound('source/tools/probes/native_wasm_checkpoint/guest.rs', deleted['authoring/guest.rs'])
    tools = data('tools.json')
    for tool in ('wasm2c', 'wat2wasm'):
        matches = [value for name, value in tools.items() if name.endswith('/wabt/bin/' + tool)]
        equal(matches, [deleted['trusted-wabt/bin/' + tool]], 'Pinned translator provenance')
    owner, result, terminal = data('output/owner.json'), data('result.json'), data('terminal-closeout.json')
    equal([result['passed'], result['error'], terminal['actual_exec_terminal_exit'], terminal['passed']],
          [False, "KeyError('stderr')", 1, False], 'Original native failure retained')
    equal(result['source_head'], HEAD, 'Original native result producer')
    require(result['source_tools_unchanged'] is True and terminal['source_tools_unchanged'] is True,
            'Original source/tool postcheck')
    require(result['fresh_target'] is True and terminal['all_observed_pids_and_managed_groups_absent'] is True,
            'Original cold target and terminal cleanup')
    equal([owner['wrapper_pid'], data('wrapper-started.json')['pid']], [result['wrapper_pid']] * 2,
          'One native wrapper identity')
    bound('result.json', terminal['result']); bound('output/owner.json', terminal['owner'])
    bound('output/process-inventory.jsonl', terminal['inventory'])
    equal([records['output/owner.json']['sha256'], records['output/process-inventory.jsonl']['sha256']],
          [result['owner_sha256'], result['process_inventory_sha256']], 'Result owner binding')
    equal(result['peaks'], owner['peaks'], 'Original resource peaks')
    require(0 < result['seconds'] <= 5400 and result['target_allocated_bytes'] <= 12 * 1024**3,
            'Original total time and storage')
    require(len(owner['calls']) == 37 and len(owner['owned']) == 39, 'Complete native call and RPC inventory')
    equal([call['exit'] for call in owner['calls']], [0] * 31 + [1] * 6, 'Actual expected command terminals')
    inventory = [strict_json(line) for line in payload(records['output/process-inventory.jsonl']).splitlines()]
    oracle.cleanup(inventory, owner, terminal['absence'], result['wrapper_pid'])
    equal(result['children_absence'], [row for row in terminal['absence'] if row['pid'] != result['wrapper_pid']],
          'Exact recorded child absence')
    receipt = data('correction/receipt.json')
    equal([receipt['native_source'], receipt['verifier_source'], receipt['no_new_native_commands']],
          [HEAD, CORRECTED, True], 'Correction provenance and no native rerun')
    bound('result.json', receipt['original_result']); bound('terminal-closeout.json', receipt['original_terminal_closeout'])
    independent = data('root-independent-review.json')
    equal([independent['original_exit'], independent['data_replay_exit'],
           independent['fresh_absent_pids_and_groups'], independent['player_sha256']],
          [1, 0, len(inventory), receipt['data_revalidation']['player_sha256']], 'Independent scoped review')
    equal(str(independent['original_session']), terminal['tool_session'], 'Independent original exec session')
    files = data('output/package-files.json')
    for name, value in files.items():
        expected = original['output/package/player/' + name]
        equal({key: value[key] for key in ('bytes', 'sha256')},
              {key: expected[key] for key in ('bytes', 'sha256')}, 'Recorded artifact inventory')
        require(bool(value['mode'] & 0o111) == expected['executable'], 'Recorded artifact executable mode')
    equal(data('output/headless.json')['copied'], original['output/incant_headless'], 'Recorded headless identity')
    return receipt, len(inventory)


def replay(records):
    receipt, count = provenance(records)
    # Extract only bounded inert records to a private temporary directory. Native
    # player/assets/debug bytes are excluded; no recorded path becomes a command.
    with tempfile.TemporaryDirectory(prefix='incant-checkpoint-data-') as temporary:
        root = Path(temporary)
        for name, row in records.items():
            if name.startswith('output/'):
                path = root / name.removeprefix('output/')
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(payload(row))
                path.chmod(0o600)
        outcome = replay_records(root)
    equal(outcome, receipt['data_revalidation'], 'Corrected recorded-data acceptance')
    return {'native_source': HEAD, 'corrected_verifier_source': CORRECTED, 'original_native_exit': 1,
            'data_revalidation_passed': True, 'positive_processes': 4, 'precise_refusals': 6,
            'checkpoint_and_pcm_exact': True, 'recorded_absent_pids': count, 'native_commands_repeated': 0,
            'external_artifact_bytes_rechecked': False, 'filesystem_process_absence_reobserved': False,
            'player_sha256': outcome['player_sha256'], 'performance_or_phase_gate': False}


def verify(path=MANIFEST):
    manifest = strict_json(path.read_bytes())
    require(manifest['format'] == 'incant-authored-wasm-checkpoint-portable-v1', 'Manifest format')
    archive = path.with_suffix('.json.gz')
    require(archive.stat().st_size <= MAX_ARCHIVE, 'Compressed archive bound')
    equal(identity(archive.read_bytes()), manifest['archive'], 'Archive identity')
    with gzip.open(archive, 'rb') as stream:
        decoded = stream.read(MAX_DECODED + 1)
    require(len(decoded) <= MAX_DECODED, 'Expanded archive bound')
    records = strict_json(decoded)['records']
    validate_records(manifest, records)
    equal(sorted(manifest['verifier']), sorted(VERIFIERS), 'Complete verifier source inventory')
    for name, expected in manifest['verifier'].items():
        require(name.startswith('tools/') and '..' not in PurePosixPath(name).parts, 'Verifier source path')
        equal(identity((REPO / name).read_bytes()), expected, 'Current verifier source identity')
    outcome = replay(records)
    equal(outcome, manifest['result'], 'Scoped derived claims')
    return manifest, records, outcome


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, default=MANIFEST)
    print(json.dumps(verify(parser.parse_args().manifest)[2], indent=2))
