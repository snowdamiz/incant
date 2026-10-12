"""Numerical presentation bindings; no art, visual layout or rendering claims."""
import json
from pathlib import Path
import runpy

ROOT = Path(__file__).resolve().parents[2]
ID = lambda n: f'{n:026d}'
HUD = dict(zip(('table', 'status', 'prompt'), map(ID, range(100, 103))))
RESULTS = dict(zip(('won', 'lost', 'active', 'inactive'), map(ID, range(103, 107))))
UI_ROOT = ID(107)
VIEWPORT = '640x480'
MESSAGES = {
    'hud.collecting': {'en': 'Pearls {collected} of {total}', 'fr': 'Perles {collected} sur {total}'},
    'hud.ready': {'en': 'All pearls found. Board the boat.', 'fr': 'Toutes les perles. Au bateau.'},
    'hud.won': {'en': 'Sailing home with {total} pearls', 'fr': 'Retour avec {total} perles'},
    'hud.lost': {'en': 'Stung by a sea urchin', 'fr': 'Piqué par un oursin'},
}


def observed_source(source):
    # Calls the actual behavior unchanged; reads actual pre-command snapshots.
    return source.replace('export default {', 'const behavior = {', 1) + '''
let observed = '';
export default { ...behavior, update(context: NativeContext) {
  const current = JSON.stringify({
    widgets: context.hud.widgets.filter(w => typeof w.text === 'string').map(w => [w.id, w.text]),
    frames: context.flipbooks.ids.map((id, i) => [id, context.flipbooks.frames[i], context.flipbooks.flags[i]]),
  });
  if (current !== observed) { context.log('fixture.presentation ' + current); observed = current; }
  behavior.update(context);
  if (context.clock.tick === 50) context.commands.setLocale({ locale: 'fr' });
} } satisfies NativeBehavior;
'''


def commands(scene, font, base):
    def entity(owner, kind, parent):
        return {'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': owner, 'name': 'Numerical ' + owner, 'parent': parent, 'provenance': None,
            'components': {'UiNode': {'kind': kind, 'style': style(owner == UI_ROOT), 'order': 0, 'disabled': False}}}}
    style = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['style']
    result = [
        {'op': 'upsert_string_table', 'table': {'id': HUD['table'], 'name': 'Planar numerical HUD',
            'source_locale': 'en', 'messages': MESSAGES}},
        {'op': 'set_locale', 'settings': {'locale': 'en'}},
        entity(UI_ROOT, {'role': 'root'}, None),
    ]
    for role, owner in {**HUD, **RESULTS}.items():
        if role == 'table':
            continue
        kind = {'role': 'container'} if role in ('active', 'inactive') else {'role': 'label', 'text': {
            'content': 'Pending', 'fonts': [font], 'font_size': 16, 'line_height': 24, 'wrap': 'word_or_glyph'}}
        result.append(entity(owner, kind, RESULTS['inactive'] if role in ('won', 'lost') else UI_ROOT))
    for owner, count in [(base['PLAYER'], 3), (base['EXIT'], 2)]:
        result.append({'op': 'set_component', 'scene_id': scene, 'entity_id': owner, 'component': 'SpriteFlipbook',
            'value': {'frames': [{'frame': base['FRAME']}] * count, 'rate': {'numerator': 1, 'denominator': 1},
                      'looping': False, 'playing': False}})
    return result


def props(base):
    return {'flipbooks': {'player': base['PLAYER'], 'boat': base['EXIT']}, 'hud': HUD, 'results': RESULTS}


def extend_observations(plan):
    plan['entities'] += [RESULTS['won'], RESULTS['lost']]
    return plan


def verify_report(name, report, base):
    entries = report['logs']['entries']
    observed = [(e['tick'], json.loads(e['message'].removeprefix('fixture.presentation ')))
                for e in entries if e['message'].startswith('fixture.presentation ')]
    phase, collected, locale = 'playing', 0, 'en'
    accepted = {'widgets': sorted([[HUD['status'], 'Pending'], [HUD['prompt'], 'Pending'],
                                  [RESULTS['won'], 'Pending'], [RESULTS['lost'], 'Pending']]),
                'frames': sorted([[base['PLAYER'], 0, 0], [base['EXIT'], 0, 0]])}
    expected, last = [], None
    events = {}
    for entry in entries:
        if entry['message'].startswith('planar.'):
            events.setdefault(entry['tick'], []).append(entry['message'])
    # Simulate only the specified display policy. Physics/outcomes are separately
    # checked by the original seven-scenario oracle, not predicted by this code.
    for tick in range(1, report['tick'] + 1):
        if accepted != last:
            expected.append((tick, accepted))
            last = accepted
        for message in events.get(tick, []):
            if message.startswith('planar.pickup'):
                collected = int(message.rsplit(' ', 1)[1].split('/')[0])
            elif message.startswith('planar.won'):
                phase = 'won'
            elif message.startswith('planar.lost'):
                phase = 'lost'
        key = 'hud.' + (phase if phase != 'playing' else 'ready' if collected == 2 else 'collecting')
        status = MESSAGES[key][locale].replace('{collected}', str(collected)).replace('{total}', '2')
        accepted = {'widgets': sorted([[HUD['status'], status], [HUD['prompt'], ''],
                                      [RESULTS['won'], 'Pending'], [RESULTS['lost'], 'Pending']]),
                    'frames': sorted([[base['PLAYER'], {'playing': 0, 'won': 2, 'lost': 1}[phase], 0],
                                      [base['EXIT'], int(collected == 2), 0]])}
        if tick == 50:
            locale = 'fr'  # The callback observes this on tick51, text commits then.
    actual = [(tick, {key: sorted(value) for key, value in state.items()}) for tick, state in observed]
    assert actual == expected, (name, actual, expected)
    for role in ('won', 'lost'):
        assert report['observations']['entities'][RESULTS[role]]['parent'] == RESULTS['active' if phase == role else 'inactive']
    assert not report['logs']['dropped']
    # Prompt deliberately stays blank, including terminal states: no fake restart.
    assert all(dict(row['widgets'])[HUD['prompt']] == '' for _, row in actual[1:])
    return {'changes': len(actual), 'phase': phase, 'collected': collected, 'final_status': dict(actual[-1][1]['widgets'])[HUD['status']]}


def refusal_controls(binary, development, output, base):
    """Actual native runs of invalid authored optional bindings; never mock commands."""
    import copy
    import hashlib
    import shutil
    import subprocess
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    checks = [
        ('unknown-role', 'Planar Sample: unknown flipbooks binding',
         lambda p: p['flipbooks'].update({'typo': base['PLAYER']})),
        ('duplicate-frame-role', 'Planar Sample flipbook roles must be distinct',
         lambda p: p['flipbooks'].update({'boat': base['PLAYER']})),
        ('duplicate-ui-role', 'Planar Sample UI roles must be distinct',
         lambda p: p['hud'].update({'prompt': p['hud']['status']})),
        ('invalid-identity', 'Planar Sample: invalid hud.table identity',
         lambda p: p['hud'].update({'table': 'not-an-id'})),
    ]
    receipts = []
    output.mkdir()
    for name, diagnostic, mutate in checks:
        child = output / name
        shutil.copytree(development / 'authoring', child)
        project = child / 'planar.json'
        # New journal starts from the copied accepted document; prior proof files
        # remain immutable. Every alteration still uses the public command bus.
        rpc = Rpc(binary, project, child / 'refusal.journal.jsonl')
        try:
            before = rpc.call('project.read'); scene = next(iter(before['project']['scenes']))
            component = copy.deepcopy(before['project']['scenes'][scene]['entities'][base['CONTROLLER']]['components']['Script'])
            mutate(component['props'])
            change = rpc.call('command.execute', {'expected_revision': before['revision'], 'description': name,
                'commands': [{'op': 'set_component', 'scene_id': scene, 'entity_id': base['CONTROLLER'],
                              'component': 'Script', 'value': component}]})
            rpc.call('project.save')
        finally:
            rpc.close()
        frozen = {str(p.relative_to(child)): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in child.rglob('*') if p.is_file()}
        pcm = output / (name + '.pcm')
        argv = list(map(str, [binary, 'native-play', project, '--camera', base['CAMERA'], '--ticks', 2,
                             '--ui-viewport', VIEWPORT, '--pcm-output', pcm]))
        run = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=180)
        receipt = {'name': name, 'argv': argv, 'exit': run.returncode, 'stdout': run.stdout,
                   'stderr': run.stderr, 'diagnostic': diagnostic, 'authoring_transaction': change,
                   'authoring_files': frozen}
        (output / (name + '.json')).write_text(json.dumps(receipt, indent=2) + '\n')
        assert run.returncode != 0 and diagnostic in run.stderr and not run.stdout and not pcm.exists(), receipt
        assert frozen == {str(p.relative_to(child)): hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in child.rglob('*') if p.is_file()}
        receipts.append(receipt)
    return receipts
