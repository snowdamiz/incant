"""Fixed workload and bounded retained-process helpers; no engine execution on import."""
import hashlib
import json
from pathlib import Path
import sys
import time

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import native_process
from native_package import read, require, write_json

COUNT, RATE, TICKS, BATCH = 1024, 60, 3600, 256
ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'


def ident(number):
    require(type(number) is int and 0 <= number < 2**128, 'Invalid identity number')
    return ''.join(ALPHABET[(number >> (5 * index)) & 31] for index in reversed(range(26)))


SCRIPT = ident(1 << 80)


def owner(index):
    require(type(index) is int and 0 <= index < COUNT, 'Invalid workload ordinal')
    return ident(10000 + index)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_hash(path, limit=64 * 1024**2):
    return digest(read(path, limit))


def json_hash(value):
    return digest(json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode())


def entity(index):
    return {'id': owner(index), 'name': f'Matched mover {index}', 'parent': None,
            'provenance': None, 'components': {'Transform': {'translation': [index / 8, 0, 0],
            'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}}}


def recording():
    # Six press/release cycles with long held/idle regions. Both tiers receive the
    # exact same retained input bytes; nothing depends on elapsed wall time.
    return {'format': 'incant-input', 'version': 1, 'tick_rate': RATE, 'start_tick': 0,
            'ticks': TICKS, 'frames': [{'tick': tick, 'events': [
                {'type': 'key', 'code': 'ArrowRight', 'down': down}]}
                for base in range(0, TICKS, 600) for tick, down in ((base + 1, True), (base + 301, False))]}


def position(index, ticks=TICKS):
    owner(index)
    require(type(ticks) is int and 1 <= ticks <= TICKS, 'Invalid oracle tick')
    # Independently integrate actual input boundaries in f64, in operation order.
    x, edges = index / 8, 0
    for tick in range(1, ticks + 1):
        if (tick - 1) % 600 == 0:
            edges += 1
        if (tick - 1) % 600 < 300:
            x += 6 * (1 / RATE)
    return [x, ticks, edges]


def observation(indices, ticks=TICKS, population=False):
    require(len(indices) <= BATCH and len(set(indices)) == len(indices), 'Observation exceeds item bound')
    checks = [{'name': f'row-{n}', 'tick': ticks, 'expect': {'type': 'vector', 'entity': owner(n),
               'property': 'translation', 'value': position(n, ticks), 'tolerance': 1e-9}} for n in indices]
    if population:
        checks.append({'name': 'population', 'tick': ticks, 'expect': {'type': 'entity_count', 'value': COUNT}})
    require(len(checks) <= BATCH, 'Observation checks exceed item bound')
    return {'format': 'incant-native-observation', 'version': 1, 'checks': checks,
            'entities': [owner(n) for n in indices]}


def run(argv, evidence, *, timeout=600, cwd=None, env=None):
    """Managed process and raw files survive refusal/timeout; never orphan compilers."""
    argv = list(map(str, argv))
    receipt = {'argv': argv, 'cwd': str(cwd) if cwd else None, 'started_ns': time.time_ns(), 'exit': None}
    try:
        with evidence.with_suffix('.stdout').open('xb') as out, evidence.with_suffix('.stderr').open('xb') as err:
            result = native_process.run(argv, stdout=out, stderr=err, timeout=timeout, cwd=cwd, env=env)
        receipt['exit'] = result.returncode
        require(result.returncode == 0, 'Command failed; inspect retained stdout/stderr')
        return read(evidence.with_suffix('.stdout'), 16 * 1024**2)
    except BaseException as error:
        receipt['error'] = repr(error)
        raise
    finally:
        receipt['ended_ns'] = time.time_ns()
        for suffix in ('.stdout', '.stderr'):
            if evidence.with_suffix(suffix).exists():
                receipt[suffix + '_sha256'] = file_hash(evidence.with_suffix(suffix), 128 * 1024**2)
        write_json(evidence.with_suffix('.json'), receipt)


def validate_documents(ts, wasm):
    """Verify the exact designed backend delta, not just equal populations."""
    require(ts['id'] == wasm['id'] and len(ts['scenes']) == len(wasm['scenes']) == 1,
            'Project/scene identities differ')
    require(ts['settings']['tick_rate'] == RATE
            and ts['settings'].get('physics_mode', 'deterministic') == 'deterministic'
            and ts['settings']['input_actions'] == {'right': {'kind': 'button',
                'bindings': [{'type': 'key', 'code': 'ArrowRight'}]}}, 'Fixed workload settings differ')
    scene = next(iter(ts['scenes']))
    require(set(wasm['scenes']) == {scene}, 'Scene identity differs')
    for kind, document in [('typescript', ts), ('wasm', wasm)]:
        entities = document['scenes'][scene]['entities']
        require(set(entities) == {owner(n) for n in range(COUNT)}, 'Exact entity membership differs')
        for n in range(COUNT):
            expected = entity(n)
            if kind == 'typescript' and n == 0:
                expected['components']['Script'] = {'source': SCRIPT, 'props': {}}
            actual = dict(entities[owner(n)]); actual['provenance'] = None
            require(actual == expected, 'Entity data or attachment differs')
    require(set(ts['scripts']) == {SCRIPT} and not wasm.get('scripts'), 'Unexpected script registry')
    require(ts['scripts'][SCRIPT] == {'id': SCRIPT, 'name': 'Matched transform system', 'path': 'behavior.ts'},
            'Unexpected TS source registration')
    require(not ts['scenes'][scene].get('program'), 'TS scene unexpectedly selects a module')
    module = wasm['scenes'][scene]['program']
    require(module['kind'] == 'wasm' and set(module) == {'kind', 'module'}, 'Unknown module selector')
    require(not ts.get('assets') and set(wasm['assets']) == {module['module']}
            and wasm['assets'][module['module']]['kind'] == 'wasm', 'Unexpected assets')
    # Erase only the enumerated backend registration delta, then require exact
    # remaining authored fields, including settings/input/provenance, to match.
    import copy
    left, right = copy.deepcopy(ts), copy.deepcopy(wasm)
    left['scripts'] = right['scripts'] = {}
    left['assets'] = right['assets'] = {}
    del left['scenes'][scene]['entities'][owner(0)]['components']['Script']
    # set_component stamps only its owning entity's public transaction provenance.
    left['scenes'][scene]['entities'][owner(0)]['provenance'] = right['scenes'][scene]['entities'][owner(0)]['provenance']
    right['scenes'][scene].pop('program')
    require(left == right, 'Unexplained authored backend delta')
    return scene


def prepared(path):
    receipt = json.loads(read(path / 'prepared.json', 1024**2))
    require(receipt['format'] == 'incant-wasm-ts-workload' and receipt['version'] == 1
            and receipt['entities'] == COUNT and receipt['ticks'] == TICKS, 'Unknown workload')
    expected = {'input.json', 'observe.json', *(f'observe-{i}.json' for i in range(4)),
                'typescript/project.json', 'typescript/behavior.ts', 'wasm/project.json', 'wasm/behavior.wasm'}
    require(set(receipt['files']) == expected, 'Incomplete workload inventory')
    for name, sha in receipt['files'].items():
        require(file_hash(path / name) == sha, 'Prepared input changed: ' + name)
    for name, sha in receipt['source'].items():
        require(name in ('behavior.rs', 'behavior.ts') and file_hash(HERE / name) == sha, 'Guest source changed')
    require(set(receipt['source']) == {'behavior.rs', 'behavior.ts'}
            and receipt['files']['typescript/behavior.ts'] == receipt['source']['behavior.ts'], 'Missing guest source binding')
    require(json.loads(read(path / 'input.json', 1024**2)) == recording(), 'Input workload differs')
    for page in range(4):
        require(json.loads(read(path / f'observe-{page}.json', 1024**2)) ==
                observation(list(range(page * BATCH, (page + 1) * BATCH))), 'All-row oracle differs')
    require(json.loads(read(path / 'observe.json', 1024**2)) ==
            observation([0, COUNT // 2, COUNT - 1], population=True), 'Timed oracle differs')
    ts, wasm = [json.loads(read(path / kind / 'project.json', 8 * 1024**2)) for kind in ('typescript', 'wasm')]
    scene = validate_documents(ts, wasm)
    require(receipt['scene_id'] == scene and receipt['project_id'] == ts['id']
            and ts['settings']['tick_rate'] == RATE, 'Prepared scene/rate differs')
    module = wasm['scenes'][scene]['program']['module']
    require(wasm['assets'][module]['sha256'] == receipt['files']['wasm/behavior.wasm']
            == receipt['guest']['wasm_sha256'], 'Guest module identity differs')
    return receipt
