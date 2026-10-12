"""Pearl Cove HUD: authored UI nodes and the localized string table.

One centred status line carries live progress and the outcome; a second label
is reserved for a restart prompt and stays blank until a real restart exists.
gameplay.ts writes both through its `hud` binding on every progress, phase or
locale change. Engine labels draw light text with a hard one-pixel shadow and
no surface, so the status sits over dark ground: the moss band, or the
letterbox bar when the window is taller than 16:9.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from art import cove  # noqa: E402

FONT_SOURCE = 'third_party/cosmic-text/fonts/Inter-Regular.ttf'  # SIL OFL 1.1
FONT_LICENSE = 'third_party/cosmic-text/fonts/Inter-LICENSE'
ROOT, STATUS, PROMPT = (cove.ulid('HD', n) for n in (1, 2, 3))
TABLE = cove.ulid('ST', 1)
LOCALES = ('en', 'es', 'fr', 'de')
# At 16:9 the alcove cliff faces begin 14 scene pixels (28 logical points at the
# 2x window scale) below the top edge. A 20-point line inset by 6 points keeps
# even the longest localized status on dark moss or the letterbox bar.
INSET, LINE = 6, 20
# Keys and arguments are fixed by gameplay.ts: every key receives `collected`
# and `total`. Plural forms keep the outcome grammatical for any pearl count.
MESSAGES = {
    'hud.collecting': {
        'en': 'Pearls {collected} of {total}',
        'es': 'Perlas: {collected} de {total}',
        'fr': 'Perles : {collected} sur {total}',
        'de': 'Perlen: {collected} von {total}',
    },
    'hud.ready': {
        'en': 'All pearls found. Board the boat.',
        'es': 'Tienes todas las perlas. Sube al bote.',
        'fr': 'Toutes les perles sont là. Montez dans la barque.',
        'de': 'Alle Perlen gefunden. Ab ins Boot!',
    },
    'hud.won': {
        'en': '{total, plural, one {Sailing home with # pearl} other {Sailing home with # pearls}}',
        'es': '{total, plural, one {Rumbo a casa con # perla} other {Rumbo a casa con # perlas}}',
        'fr': '{total, plural, one {Retour au port avec # perle} other {Retour au port avec # perles}}',
        'de': '{total, plural, one {Mit # Perle nach Hause} other {Mit # Perlen nach Hause}}',
    },
    'hud.lost': {
        'en': 'Stung by a sea urchin',
        'es': 'Te picó un erizo de mar',
        'fr': 'Piqué par un oursin',
        'de': 'Von einem Seeigel gestochen',
    },
}


def style(**changes):
    value = {'size': ['auto', 'auto'], 'min_size': [0, 0], 'max_size': [None, None],
             'padding': [0, 0, 0, 0], 'gap': [0, 0], 'direction': 'column', 'wrap': False,
             'align': 'stretch', 'justify': 'start', 'grow': 0, 'shrink': 1, 'basis': 'auto'}
    value.update(changes)
    return value


def label(font, size, line):
    # Authored blank: the controller writes the first localized value at once,
    # so no frame shows a static string that could disagree with the locale.
    return {'role': 'label', 'text': {'content': '', 'fonts': [font], 'font_size': size,
                                       'line_height': line, 'wrap': 'word'}}


def node(scene, ident, name, kind, node_style, parent):
    return {'op': 'create_entity', 'scene_id': scene, 'entity': {
        'id': ident, 'name': name, 'parent': parent, 'provenance': None,
        'components': {'UiNode': {'kind': kind, 'style': node_style, 'order': 0, 'disabled': False}}}}


def commands(scene, font, controller):
    """One atomic transaction: string table, locale, UI nodes and the binding."""
    props = dict(controller['props'])
    props.update({'flipbooks': {'player': cove.PLAYER, 'boat': cove.BOAT},
                  'hud': {'table': TABLE, 'status': STATUS, 'prompt': PROMPT}})
    return [
        {'op': 'upsert_string_table', 'table': {'id': TABLE, 'name': 'Pearl Cove HUD',
                                                'source_locale': 'en', 'messages': MESSAGES}},
        {'op': 'set_locale', 'settings': {'locale': 'en'}},
        node(scene, ROOT, 'HUD', {'role': 'root'},
             style(padding=[24, INSET, 24, INSET], justify='space_between', align='center'), None),
        node(scene, STATUS, 'HUD status', label(font, 16, LINE), style(min_size=[0, LINE]), ROOT),
        node(scene, PROMPT, 'HUD prompt', label(font, 16, LINE), style(min_size=[0, LINE]), ROOT),
        {'op': 'set_component', 'scene_id': scene, 'entity_id': cove.CONTROLLER,
         'component': 'Script', 'value': {'source': controller['source'], 'props': props}},
    ]
