"""Actual-process assertion controls added to the existing source-free build.

No compiler invocation, extra shipping binary, screenshot or visual approval.
"""
import copy
import hashlib
import json
from pathlib import Path
import subprocess


def verify(out, command, baseline, entity_count, reject, run, require):
    source = '00000000000000000000000001'
    missing = '00000000000000000000000009'
    def check(name, tick, expect):
        return {'name': name, 'tick': tick, 'expect': expect}
    def vector(name, tick, prop, value):
        return check(name, tick, {'type': 'vector', 'entity': source, 'property': prop,
                                 'value': value, 'tolerance': 0.001})
    plan = {'format': 'incant-native-observation', 'version': 1, 'entities': [source, missing], 'checks': [
        check('startup count', 0, {'type': 'entity_count', 'value': entity_count}),
        vector('startup position', 0, 'translation', [-3, 0, 0]),
        vector('first position precedes scripted velocity', 1, 'translation', [-3, 0, 0]),
        vector('accepted script velocity', 1, 'velocity', [3, 0, 0]),
        check('press edge', 1, {'type': 'action', 'name': 'right', 'pressed': True, 'active': True}),
        check('held without repeated edge', 2, {'type': 'action', 'name': 'right', 'pressed': False, 'active': True}),
        vector('final committed world', 120, 'world_translation', [-3+119*3/60, 0, 0]),
        vector('final solver position', 120, 'physics_translation', [-3+119*3/60, 0, 0]),
        vector('final solver velocity', 120, 'physics_velocity', [3, 0, 0]),
        check('explicit missing entity', 120, {'type': 'exists', 'entity': missing, 'value': False}),
    ]}
    path = out/'observations.json'; path.write_text(json.dumps(plan))
    reports = []
    for repeat in range(2):
        dest = out/f'observed-{repeat}.pcm'
        report = json.loads(run(command(dest)+['--observe', path]))
        observed = report.pop('observations')
        require(report == baseline, 'Observation changed accepted game/render/audio behavior')
        require(observed['passed'] and len(observed['checks']) == 10
                and all(c['passed'] for c in observed['checks']), 'Typed checks did not pass')
        require(observed['tick'] == 120 and observed['tick_rate'] == 60, 'Observation clock mismatch')
        require(observed['entities'][missing] is None and
                observed['entities'][source]['translation'] == baseline['sources'][0]['position'],
                'Selected snapshot is not final committed state')
        reports.append(observed)
    require(reports[0] == reports[1], 'Fresh-process observations changed')
    def completed_failure(label, changed):
        path = out/f'{label}.json'; path.write_text(json.dumps(changed)); dest = out/f'{label}.pcm'
        process = subprocess.run(command(dest)+['--observe', path], capture_output=True, text=True, encoding='utf-8')
        require(process.returncode != 0 and 'assertions failed' in process.stderr and dest.is_file(),
                f'{label} did not distinguish assertion failure from runtime failure')
        report = json.loads(process.stdout)
        require(not report['observations']['passed'] and report['tick'] == 120
                and report['pcm_sha256'] == baseline['pcm_sha256']
                and hashlib.sha256(dest.read_bytes()).hexdigest() == baseline['pcm_sha256'],
                f'{label} changed simulation instead of observing it')
        return report['observations']
    wrong = copy.deepcopy(plan); wrong['checks'][0]['expect']['value'] = entity_count+1
    count_failure = completed_failure('observation-count-false', wrong)
    require(sum(not c['passed'] for c in count_failure['checks']) == 1, 'False assertion affected other checks')
    wrong = copy.deepcopy(plan)
    wrong['checks'] = [check('missing component', 120, {'type':'vector',
        'entity':'00000000000000000000000002', 'property':'physics_velocity', 'value':[0,0,0], 'tolerance':0})]
    missing_failure = completed_failure('observation-missing-component', wrong)
    require(missing_failure['checks'][0]['actual'] is None, 'Missing component became a default zero')
    variants = []
    for label, key, value in [('format','format','unknown'), ('version','version',2), ('extra','extra',0),
                              ('duplicate-entities','entities',[source,source]),
                              ('noncanonical-id','entities',['7zzzzzzzzzzzzzzzzzzzzzzzzz'])]:
        wrong = copy.deepcopy(plan); wrong[key] = value; variants.append((label,wrong))
    wrong = copy.deepcopy(plan); wrong['checks'][0]['tick'] = 121; variants.append(('beyond-end',wrong))
    wrong = copy.deepcopy(plan); wrong['checks'] = wrong['checks']*26; variants.append(('excess-checks',wrong))
    for label, wrong in variants:
        path = out/f'observation-{label}.json'; path.write_text(json.dumps(wrong)); dest = out/f'observation-{label}.pcm'
        reject(f'observation-{label}',command(dest)+['--observe',path],dest)
    path = out/'observation-oversized.json'; path.write_bytes(b' '*(256*1024+1)); dest = out/'observation-oversized.pcm'
    reject('observation-oversized',command(dest)+['--observe',path],dest,'runtime input size limit')
    path = out/'observations.json'; dest = out/'observation-duplicate-option.pcm'
    reject('observation-duplicate-option',command(dest)+['--observe',path,'--observe',path],dest)
    # A passed early assertion must not produce a completed report after a late
    # accepted script failure. Observations cannot make a terminal game usable.
    recording = out/'observation-terminal-input.json'
    recording.write_text(json.dumps({'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':120,
        'frames':[{'tick':2,'events':[{'type':'key','code':'KeyF','down':True}]}]}))
    dest = out/'observation-terminal.pcm'; cmd = command(dest); cmd[4] = recording
    reject('observation-terminal-failure',cmd+['--observe',path],dest,'accepted game input failure')
    return {'passed':True, 'success_processes':2, 'completed_false_processes':2,
            'startup_refusals':len(variants)+2, 'terminal_failure_processes':1,
            'report':reports[0], 'count_failure':count_failure, 'missing_failure':missing_failure}
