"""Data-only oracle for the fixed public authored restart proof."""
import hashlib
import json
from pathlib import Path

ENTITY = '00000000000000000000000002'
TICKS = 3
FRAMES_PER_TICK = 800  # Ordinary --package: 48 kHz, authored scene: 60 Hz.
PCM = bytes(TICKS * FRAMES_PER_TICK * 2 * 4)
GUEST_SHA256 = 'f46df26fad21d80ed817a5a63ae77f88d8920adcbeaae46fd8b343cd75cede70'
SOURCE_SHA256 = '9ac2db8148cd0bcfa22a669d3f75f4f360dd9860a6ea335306419af76380dd9a'
DEV_REFUSAL = ('Error: native play startup/runtime failure: development Wasm rejected or trapped: '
               'Wasm ABI: host does not accept session restart requests')
CHECKPOINT_REFUSAL = 'Error: Checkpoint(Unsupported("static module state ABI"))'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def canonical(value):
    # Python equality accepts True == 1; the wire contract does not.
    return json.dumps(value, sort_keys=True, allow_nan=False, separators=(',', ':'))


def equal(actual, expected, reason):
    require(canonical(actual) == canonical(expected), reason)


def numeric_equal(actual, expected, reason):
    def normalize(value):
        if type(value) is float and value.is_integer():
            return int(value)
        if isinstance(value, list):
            return [normalize(v) for v in value]
        if isinstance(value, dict):
            return {k: normalize(v) for k, v in value.items()}
        return value
    equal(normalize(actual), normalize(expected), reason)


def observation():
    return {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
            'checks': [{'name': f'fresh-vm-{tick}', 'tick': tick,
                        'expect': {'type': 'vector', 'entity': ENTITY, 'property': 'translation',
                                   'value': [1, 0, 0], 'tolerance': 0}}
                       for tick in range(1, TICKS + 1)]}


def validate_run(report, pcm):
    require(type(report.get('tick')) is int and report['tick'] == 0, 'Final reset tick')
    require(type(report.get('frames')) is int and report['frames'] == 2400, 'Host output frames')
    equal(report.get('session'), {
        'generation': 4, 'host_tick': 3, 'simulation_tick': 0,
        'restarts': [{'host_step': n, 'completed_generation': n, 'simulation_tick': 1,
                      'output_frames': str(n * FRAMES_PER_TICK)} for n in range(1, 4)],
    }, 'Exact three replacement boundaries and host chronology')
    require(pcm == PCM and report.get('pcm_sha256') == hashlib.sha256(PCM).hexdigest(),
            'Exact silent PCM length, content and reported digest')
    obs = report['observations']
    require(obs.get('passed') is True and type(obs.get('tick')) is int and obs['tick'] == 0
            and type(obs.get('tick_rate')) is int and obs['tick_rate'] == 60, 'Observation owner/clock')
    checks = obs.get('checks')
    require(isinstance(checks, list) and len(checks) == 3, 'All three accepted-step observations')
    for actual, expected in zip(checks, observation()['checks']):
        # Rust serializes vectors/tolerance as floats, while the authored JSON may use integers.
        equal({k: actual.get(k) for k in ('name', 'tick')},
              {k: expected[k] for k in ('name', 'tick')}, 'Observation name and host step')
        require(actual.get('passed') is True, 'Accepted-step predicate failed')
        numeric_equal(actual.get('actual'), [1, 0, 0], 'Observed transform before replacement')
        numeric_equal(actual.get('expect'), expected['expect'], 'Executed observation predicate')
    entities = obs.get('entities')
    require(isinstance(entities, dict) and set(entities) == {ENTITY}, 'Final observed entity inventory')
    entity = entities[ENTITY]
    numeric_equal(entity, {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1],
                          'scale': [1, 1, 1], 'parent': None, 'velocity': None,
                          'world_transform': [[1, 0, 0, 0], [0, 1, 0, 0],
                                              [0, 0, 1, 0], [0, 0, 0, 1]]},
                  'Fresh final entity state')
    equal(report['logs'], {'entries': [], 'dropped': 0}, 'Fresh VM has no retained logs')


def validate_program(export, imported, captured, registry, player):
    program = export['program']
    equal({k: program.get(k) for k in ('kind', 'module', 'fingerprint', 'input_sha256',
                                      'registry_sha256', 'translator')}, {
        'kind': 'wasm', 'module': imported['asset']['id'], 'fingerprint': imported['asset']['sha256'],
        'input_sha256': GUEST_SHA256, 'registry_sha256': registry, 'translator': 'wasm2c-1.0.42',
    }, 'Authored module/guest/registry identity')
    equal(program.get('prepared_sha256'), captured['prepared_sha256'], 'Captured metered identity')
    require(captured['input_sha256'] == GUEST_SHA256 and captured['registry_sha256'] == registry,
            'Imported original and ABI identity')
    require(export.get('player_sha256') == digest(player), 'Copied player identity')
    require(export.get('build_storage_removed_before_startup') is True
            and export.get('startup_source_parsing') is False, 'Ordinary source-free exporter')
    startup = export['startup']
    equal({k: startup.get(k) for k in ('tick', 'frames', 'session')}, {
        'tick': 0, 'frames': 800, 'session': {'generation': 2, 'host_tick': 1,
            'simulation_tick': 0, 'restarts': [{'host_step': 1, 'completed_generation': 1,
                'simulation_tick': 1, 'output_frames': '800'}]}}, 'Exporter one-tick cold restart')
    require(startup.get('pcm_sha256') == hashlib.sha256(bytes(800 * 2 * 4)).hexdigest(),
            'Exporter startup PCM')


def validate_refusal(result, diagnostic, outputs):
    require(result.returncode == 1 and result.stdout == '' and result.stderr.strip() == diagnostic,
            'Exact expected refusal, without successful stdout')
    require(all(not p.exists() and not p.is_symlink() for p in outputs), 'Refusal published output')


def validate_deleted(paths):
    require(all(not p.exists() and not p.is_symlink() for p in paths), 'Private inputs remain')
