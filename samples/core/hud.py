"""Localized screen-space HUD for the Core Sample (typed UiNode widgets).

ui/hud.json declares widgets with stable IDs, text as string-table keys, and the
presentation states. ui/strings.json is the typed string table. Display text is
resolved by the engine itself: a disposable probe project (built through the same
public commands) runs one native-play tick that calls localization.localize()
for every key/argument pair in the requested locale, including pseudo-locale.
The builder then writes those exact strings into UiNode text. Runtime binding of
changing values is Astra's integration work; see PRESENTATION.md.
"""
import json
from pathlib import Path

from art import ids

HERE = Path(__file__).resolve().parent
DOCUMENT = json.loads((HERE / 'ui/hud.json').read_text(encoding='utf-8'))
TABLE = json.loads((HERE / 'ui/strings.json').read_text(encoding='utf-8'))
LOCALES = {'en': {'locale': 'en'}, 'en-XA': {'locale': 'en', 'pseudo': True},
           'ar': {'locale': 'ar'}, 'zh-Hans': {'locale': 'zh-Hans'}}
RTL = {'ar'}
PROBE_SCRIPT = ids.make('script', 90)
PROBE_ENTITY = ids.entity(900)
DEFAULT_STYLE = {'size': ['auto', 'auto'], 'min_size': [0, 0], 'max_size': [None, None], 'padding': [0, 0, 0, 0],
                 'gap': [0, 0], 'direction': 'column', 'wrap': False, 'align': 'stretch', 'justify': 'start',
                 'grow': 0, 'shrink': 1, 'basis': 'auto'}


def widget(index):
    return ids.widget(index)


def widgets():
    return {w['index']: w for w in DOCUMENT['widgets']}


def prompt():
    """Optional authored role; this helper never invents its node or wording."""
    selected = [w for w in DOCUMENT['widgets'] if w.get('binding') == 'prompt']
    if len(selected) > 1:
        raise ValueError('Core HUD has duplicate prompt bindings')
    if not selected:
        return None
    node = selected[0]
    if node['role'] != 'label' or node['text']['key'] != 'hud.prompt':
        raise ValueError('Core HUD prompt must be a label using hud.prompt')
    return widget(node['index'])


def requests(total, include_prompt=None):
    """Every (request id, key, arguments) any presentation state can display."""
    out = [('health.%d' % h, 'hud.health', {'health': h}) for h in (3, 2, 1, 0)]
    out += [('samples.%d' % c, 'hud.samples', {'collected': c, 'total': total}) for c in range(total + 1)]
    out += [(k, k, {}) for k in ('hud.objective.collect', 'hud.objective.exit', 'hud.controls', 'hud.won',
                                 'hud.lost')]
    needs_prompt = prompt() is not None if include_prompt is None else include_prompt
    if needs_prompt:
        out.append(('hud.prompt', 'hud.prompt', {}))
    return out


PROBE_SOURCE = """import type { NativeBehavior } from '@incant/runtime/native';
const REQUESTS: [string, string, { [name: string]: number }][] = %s;
/** One-tick localization probe: logs the engine's own text for each request. */
export default {
  update({ localization, log, clock }) {
    if (clock.tick !== 1) return undefined;
    for (const [id, key, args] of REQUESTS) {
      const result = localization.localize({ table_id: '%s', key, arguments: args });
      log(JSON.stringify({ id, text: result.text, resolved: result.resolved_locale, missing: result.missing ?? null }));
    }
    return undefined;
  },
} satisfies NativeBehavior;
"""


def resolve(binary, workdir, locale, total, run, session_type, *, table=None, include_prompt=None):
    """Engine-resolved display strings for `locale` via a disposable probe project."""
    table = TABLE if table is None else table
    workdir.mkdir()
    project = workdir / 'probe.json'
    run([binary, 'init', project, '--entities', '0', '--name', 'HUD localization probe'])
    session = session_type(binary, project, workdir / 'probe.journal.jsonl', [])
    try:
        scene = next(iter(session.call('project.read')['project']['scenes']))
        source = PROBE_SOURCE % (json.dumps(requests(total, include_prompt), ensure_ascii=False), table['id'])
        session.execute('Localization probe for ' + locale, [
            {'op': 'upsert_string_table', 'table': table},
            {'op': 'set_locale', 'settings': LOCALES[locale]},
            {'op': 'write_script', 'script_id': PROBE_SCRIPT, 'expected_source': {'state': 'missing'},
             'source': source},
            {'op': 'upsert_script', 'script': {'id': PROBE_SCRIPT, 'name': 'Localization probe', 'path': 'probe.ts'}},
            {'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': PROBE_ENTITY, 'name': 'Probe', 'parent': None, 'provenance': None, 'components': {
                    'Script': {'source': PROBE_SCRIPT, 'props': {}}}}}])
        session.call('project.save')
    finally:
        session.close()
    report = json.loads(run([binary, 'native-play', project, '--ticks', '1']).stdout)
    (workdir / 'native-play.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n',
                                              encoding='utf-8')
    texts, missing = {}, []
    for entry in report['logs']['entries']:
        value = json.loads(entry['message'])
        texts[value['id']] = value['text']
        if value['missing']:
            missing.append(value)
    expected = {r[0] for r in requests(total, include_prompt)}
    if set(texts) != expected:
        raise RuntimeError(f'localization probe returned {sorted(set(texts) ^ expected)}')
    if missing:
        raise RuntimeError(f'missing {locale} strings: {missing}')
    return texts


def _style(spec, rtl):
    style = dict(DEFAULT_STYLE)
    style.update(spec)
    if rtl:
        if style['direction'] == 'row':
            style['direction'] = 'row_reverse'
        if style['direction'] == 'column' and style['align'] in ('start', 'end'):
            style['align'] = 'end' if style['align'] == 'start' else 'start'
    return style


def _text(spec, texts, state, total):
    key = spec['text']['key']
    if spec['binding'] == 'health':
        return texts['health.%d' % state['health']]
    if spec['binding'] == 'collected':
        return texts['samples.%d' % state['collected']]
    if key == '$objective':
        return '' if state['result'] == 'won' else texts[state['objective']]
    return texts[key]


def _node(spec, texts, state, total, fonts, rtl):
    kind = {'role': spec['role']}
    if spec['role'] == 'label':
        kind['text'] = {'content': _text(spec, texts, state, total), 'fonts': fonts,
                        'font_size': spec['font_size'], 'line_height': spec['line_height'],
                        'wrap': 'word'}
    return {'kind': kind, 'style': _style(spec.get('style', {}), rtl), 'order': spec['index'], 'disabled': False}


def author(scene, texts, fonts, total):
    """create_entity commands for the HUD in its 'playing' state."""
    state = DOCUMENT['states']['playing']
    out = []
    for spec in DOCUMENT['widgets']:
        parent = widget(spec['parent']) if spec['parent'] else None
        out.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': widget(spec['index']), 'name': 'HUD ' + spec['name'] if spec['parent'] else spec['name'],
            'parent': parent, 'provenance': None,
            'components': {'UiNode': _node(spec, texts, state, total, fonts, False)}}})
    return out


def present(scene, texts, fonts, total, locale, state_name):
    """set_component/reparent commands presenting one locale and gameplay state."""
    state = DOCUMENT['states'][state_name]
    rtl = locale in RTL
    out = [{'op': 'set_locale', 'settings': LOCALES[locale]}]
    by_name = {w['name']: w for w in DOCUMENT['widgets']}
    for spec in DOCUMENT['widgets']:
        out.append({'op': 'set_component', 'scene_id': scene, 'entity_id': widget(spec['index']),
                    'component': 'UiNode', 'value': _node(spec, texts, state, total, fonts, rtl)})
    for role, name in (('won', 'Won'), ('lost', 'Lost')):
        out.append({'op': 'reparent_entity', 'scene_id': scene, 'entity_id': widget(by_name[name]['index']),
                    'parent': widget(by_name['Result' if state['result'] == role else 'Inactive results']['index'])})
    if prompt() is not None:
        out.append({'op': 'reparent_entity', 'scene_id': scene, 'entity_id': prompt(),
                    'parent': widget(by_name['Result' if state['result'] else 'Inactive results']['index'])})
    return out


def display_strings(texts):
    return sorted(set(texts.values()))
