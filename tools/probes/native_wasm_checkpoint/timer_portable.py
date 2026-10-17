"""Replay the retained authored timer export; never executes archived code."""
import argparse
import gzip
import json
from pathlib import Path
import struct
import tempfile

from . import oracle, portable
from .replay import replay_records
from ..native_wasm_restart.verify import equal, require

REPO = Path(__file__).resolve().parents[3]
MANIFEST = REPO / 'docs/spikes/evidence/native-wasm-timers-export.json'
HEAD = '1684560629db00f822d8bd2eeb0e4b3bfdc01bce'
MAX_ARCHIVE, MAX_DECODED = 8 * 1024**2, 96 * 1024**2
VERIFIERS = portable.VERIFIERS - {'docs/spikes/evidence/native-wasm-checkpoint-export-tests.py'} | {
    'tools/probes/native_wasm_checkpoint/timer_portable.py',
    'docs/spikes/evidence/native-wasm-timers-export-tests.py'}


def provenance(records):
    def data(name):
        return portable.strict_json(portable.payload(records[name]))
    def bound(name, expected):
        equal(portable.identity(portable.payload(records[name])),
              {k: expected[k] for k in ('bytes', 'sha256')}, 'Cross-record binding')
    source, plan, packet = data('source.json'), data('plan.json'), data('packet.json')
    grant, result, terminal = (data(n) for n in ('root-grant.json', 'result.json', 'terminal-closeout.json'))
    equal([source['head'], plan['source_head'], result['source_head'], data('output/headless.json')['source_head'],
           data('output/proof.json')['source_head']], [HEAD] * 5, 'Exact native producing source')
    for name, value in packet['files'].items():
        bound(name, value)
    equal(records['packet.json']['sha256'], grant['packet_sha256'], 'Exact granted packet')
    require(grant['CPU_authorized'] is True and grant['GPU_authorized'] is False, 'CPU-only authorization')
    equal([result['passed'], result.get('error'), terminal['actual_exec_terminal_exit'], terminal['passed']],
          [True, None, 0, True], 'Actual successful native terminal')
    require(result['source_tools_unchanged'] is True and terminal['source_tools_unchanged'] is True,
            'Source and compiler postcheck')
    require(result['fresh_target'] is True and terminal['all_observed_pids_and_managed_groups_absent'] is True,
            'Fresh target and actual terminal closeout')
    equal([plan['jobs'], plan['rayon_threads'], plan['aggregate_seconds'], plan['target_ceiling_allocated_bytes']],
          [2, 2, 5400, 12 * 1024**3], 'Bounded cold allocation')
    equal([plan[n] for n in ('scratch_bytes', 'retained_bytes', 'rss_bytes', 'launch_floor_bytes', 'free_floor_bytes')],
          [n * 1024**3 for n in (8, 2, 8, 28, 6)], 'Original resource bounds')
    owner = data('output/owner.json')
    for name, field in (('result.json', 'result'), ('output/owner.json', 'owner'),
                        ('output/process-inventory.jsonl', 'inventory')):
        bound(name, terminal[field])
    equal([records['output/owner.json']['sha256'], records['output/process-inventory.jsonl']['sha256']],
          [result['owner_sha256'], result['process_inventory_sha256']], 'Result owner binding')
    equal(result['peaks'], owner['peaks'], 'Actual resource peaks')
    require(0 < result['seconds'] <= 5400 and result['target_allocated_bytes'] <= 12 * 1024**3,
            'Total time and target storage')
    equal([owner['wrapper_pid'], data('wrapper-started.json')['pid']], [result['wrapper_pid']] * 2,
          'One wrapper identity')
    equal([c['exit'] for c in owner['calls']], [0] * 31 + [1] * 6, 'Complete command terminals')
    require(len(owner['owned']) == 39, 'Complete command and RPC inventory')
    inventory = [portable.strict_json(line) for line in portable.payload(records['output/process-inventory.jsonl']).splitlines()]
    oracle.cleanup(inventory, owner, terminal['absence'], result['wrapper_pid'])
    equal(result['children_absence'], [r for r in terminal['absence'] if r['pid'] != result['wrapper_pid']],
          'Exact child closeout')
    guest = data('output/guest.json')
    equal(guest['sdk']['target'], 'wasm32-unknown-unknown', 'Real SDK target')
    require(guest['sdk']['build_scripts_or_proc_macros'] is False, 'Fixed trusted SDK')
    for name, digest in guest['sdk']['source_sha256'].items():
        equal(source['files'][name]['sha256'], digest, 'Complete SDK binding')
        bound('source/' + name, source['files'][name])
    for name in records:
        if name.startswith('source/'):
            bound(name, source['files'][name.removeprefix('source/')])
    bound('source/tools/probes/native_wasm_checkpoint/guest.rs', guest['source'])
    deleted = data('output/deleted-0.json') | data('output/deleted-1.json')
    equal(deleted['authoring/game.wasm'], guest['artifact'], 'Removed authored guest')
    bound('source/tools/probes/native_wasm_checkpoint/guest.rs', deleted['authoring/guest.rs'])
    tools = data('tools.json')
    for tool in ('wasm2c', 'wat2wasm'):
        equal([v for n, v in tools.items() if n.endswith('/wabt/bin/' + tool)],
              [deleted['trusted-wabt/bin/' + tool]], 'Pinned translator')
    for name, (_, end) in oracle.RUNS.items():
        saved = portable.payload(records['output/' + name + '.save'])
        start = 128 + struct.unpack_from('<I', saved, 80)[0]
        require(saved[start:start + 8] == b'INCWPS02', 'Current timer-bearing checkpoint')
        oracle.checkpoint(saved, end)
    return len(inventory)


def replay(records):
    count = provenance(records)
    with tempfile.TemporaryDirectory(prefix='incant-timer-records-') as temporary:
        root = Path(temporary)
        for name, row in records.items():
            if name.startswith('output/'):
                path = root / name.removeprefix('output/')
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(portable.payload(row)); path.chmod(0o600)
        outcome = replay_records(root)
    equal(outcome, portable.strict_json(portable.payload(records['output/data-replay.json'])),
          'Original data replay')
    return {'native_source': HEAD, 'actual_native_exit': 0, 'positive_processes': 4,
            'precise_refusals': 6, 'timer_schedules_and_clock_exact': True,
            'checkpoint_and_pcm_continuation_exact': True, 'recorded_absent_pids': count,
            'player_sha256': outcome['player_sha256'], 'native_commands_repeated': 0,
            'external_artifacts_rechecked': False, 'process_absence_reobserved': False,
            'performance_or_phase_gate': False}


def verify(path=MANIFEST):
    manifest = portable.strict_json(path.read_bytes())
    equal(manifest['format'], 'incant-authored-wasm-timers-portable-v1', 'Manifest format')
    archive = path.with_suffix('.json.gz')
    require(archive.stat().st_size <= MAX_ARCHIVE, 'Archive bound')
    equal(portable.identity(archive.read_bytes()), manifest['archive'], 'Archive identity')
    with gzip.open(archive, 'rb') as stream:
        raw = stream.read(MAX_DECODED + 1)
    require(len(raw) <= MAX_DECODED, 'Expanded archive bound')
    records = portable.strict_json(raw)['records']
    portable.validate_records(manifest, records)
    equal(sorted(manifest['verifier']), sorted(VERIFIERS), 'Complete verifier inventory')
    for name, expected in manifest['verifier'].items():
        equal(portable.identity((REPO / name).read_bytes()), expected, 'Current verifier identity')
    outcome = replay(records)
    equal(outcome, manifest['result'], 'Derived claims')
    return manifest, records, outcome


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, default=MANIFEST)
    print(json.dumps(verify(parser.parse_args().manifest)[2], indent=2))
