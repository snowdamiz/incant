"""Semantic connections for Claude's existing Core widgets and audio sources.

No scene geometry, widget layout, strings, fonts, mixer settings or gameplay math
are defined here. The IDs are those already authored by scene.py. Commands for
an existing project are submitted by the caller through public incant_cmd RPC.
"""
import copy

from art import ids

SCENE = ids.make('scene', 1)
SCRIPT = ids.make('script', 1)
CONTROLLER = ids.entity(1)
HUD = {'table': ids.make('table', 1), **{
    role: ids.widget(index) for role, index in (
        ('health', 4), ('collected', 7), ('objective', 5), ('won', 11),
        ('lost', 12), ('result', 9), ('inactive', 10))}}
# Explicit restartAudio reuses a completed voice. The other historically authored
# per-occurrence voices remain unchanged and silent; they are not additional cues.
AUDIO = {role: ids.entity(index) for role, index in (
    ('music', 42), ('pickup', 43), ('damage', 47), ('won', 50), ('lost', 51))}


def roles():
    return {'hud': dict(HUD), 'audio': dict(AUDIO)}


def with_restart(actions):
    result = copy.deepcopy(actions)
    result['restart'] = {'kind': 'button', 'bindings': [
        {'type': 'key', 'code': 'KeyR'},
        {'type': 'gamepad_button', 'id': 0, 'button': 'start'}]}
    return result


def commands(document, script, source):
    """One reversible public transaction connecting an existing authored mesa.

    The source/version pair comes from script.read, never a guessed file write.
    Require the real authored role types before proposing their references.
    """
    entities = document['scenes'][SCENE]['entities']
    for role, owner in HUD.items():
        if role == 'table':
            if owner not in document.get('string_tables', {}):
                raise ValueError('Core HUD string table is missing')
            continue
        expected = 'container' if role in ('result', 'inactive') else 'label'
        if entities[owner]['components']['UiNode']['kind']['role'] != expected:
            raise ValueError('Core HUD role mismatch: ' + role)
    for owner in AUDIO.values():
        if 'AudioSource' not in entities[owner]['components']:
            raise ValueError('Core audio role requires an authored AudioSource')
    controller = copy.deepcopy(entities[CONTROLLER]['components']['Script'])
    if controller['source'] != SCRIPT:
        raise ValueError('Core controller source identity mismatch')
    controller['props'].update(roles())
    return [
        {'op': 'write_script', 'script_id': SCRIPT,
         'expected_source': script['version'], 'source': source},
        {'op': 'set_component', 'scene_id': SCENE, 'entity_id': CONTROLLER,
         'component': 'Script', 'value': controller},
        {'op': 'set_input_actions', 'actions': with_restart(document['settings']['input_actions'])},
    ]
