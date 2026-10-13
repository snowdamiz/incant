"""Semantic connections for Claude's existing Core widgets and audio sources.

No scene geometry, widget layout, strings, fonts, mixer settings or gameplay math
are defined here. The IDs are those already authored by scene.py. Commands for
an existing project are submitted by the caller through public incant_cmd RPC.
"""
import copy
import re

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


def roles(prompt=None):
    value = dict(HUD)
    if prompt is not None:
        value['prompt'] = prompt
    return {'hud': validate_hud(value), 'audio': dict(AUDIO)}


def validate_hud(value):
    """The existing sample roles plus one explicitly bound optional label."""
    if not isinstance(value, dict):
        raise ValueError('Core HUD binding must be an object')
    if set(value) - (set(HUD) | {'prompt'}):
        raise ValueError('Core HUD binding has unknown roles')
    for role, expected in HUD.items():
        if role not in value:
            raise ValueError('Core HUD required role is missing: ' + role)
        if value[role] != expected:
            raise ValueError('Core HUD binding identity mismatch: ' + role)
    if 'prompt' in value and (not isinstance(value['prompt'], str) or
            not re.fullmatch(r'[0-7][0-9A-HJKMNP-TV-Z]{25}', value['prompt'])):
        raise ValueError('Core HUD prompt requires a stable entity ID')
    widgets = [owner for role, owner in value.items() if role != 'table']
    if len(set(widgets)) != len(widgets):
        raise ValueError('Core HUD roles must be distinct')
    if 'prompt' in value and value['prompt'] in {ids.widget(i) for i in range(1, 13)}:
        raise ValueError('Core HUD prompt must be an additional label')
    return dict(value)


def widget_ids(hud):
    value = validate_hud(hud)
    return {ids.widget(index) for index in range(1, 13)} | ({value['prompt']} if 'prompt' in value else set())


def authored_hud(document):
    entities = document['scenes'][SCENE]['entities']
    props = entities[CONTROLLER]['components']['Script']['props']
    value = validate_hud(props.get('hud', HUD))
    for role, owner in value.items():
        if role == 'table':
            if owner not in document.get('string_tables', {}):
                raise ValueError('Core HUD string table is missing')
            continue
        if owner not in entities or 'UiNode' not in entities[owner]['components']:
            raise ValueError('Core HUD widget is missing: ' + role)
        expected = 'container' if role in ('result', 'inactive') else 'label'
        if entities[owner]['components']['UiNode']['kind']['role'] != expected:
            raise ValueError('Core HUD role mismatch: ' + role)
    actual = {owner for owner, entity in entities.items() if 'UiNode' in entity['components']}
    if actual != widget_ids(value):
        raise ValueError('Core HUD widget inventory differs from declared roles')
    return value


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
    selected_hud = authored_hud(document)
    for owner in AUDIO.values():
        if 'AudioSource' not in entities[owner]['components']:
            raise ValueError('Core audio role requires an authored AudioSource')
    controller = copy.deepcopy(entities[CONTROLLER]['components']['Script'])
    if controller['source'] != SCRIPT:
        raise ValueError('Core controller source identity mismatch')
    controller['props'].update({'hud': selected_hud, 'audio': dict(AUDIO)})
    return [
        {'op': 'write_script', 'script_id': SCRIPT,
         'expected_source': script['version'], 'source': source},
        {'op': 'set_component', 'scene_id': SCENE, 'entity_id': CONTROLLER,
         'component': 'Script', 'value': controller},
        {'op': 'set_input_actions', 'actions': with_restart(document['settings']['input_actions'])},
    ]
