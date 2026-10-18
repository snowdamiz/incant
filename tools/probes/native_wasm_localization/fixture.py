"""Authored catalog and independent expected coverage; never executes guest code."""
from ..native_wasm_checkpoint.fixture import tone

ENTITY, AUDIO, SCRIPT, TABLE_A, TABLE_B = [f'{n:026}' for n in (1, 2, 3, 4, 5)]
TABLES = (TABLE_A, TABLE_B)
LOCALES = ('en', 'fr-CA', 'de', 'zh-Hant')
TICKS, SPLIT, FRAMES = 12, 5, 800
SCHEMA = bytes([41]) * 32
FAILURES = {f'Digit{i}': name for i, name in enumerate(
    ('offset', 'zero-limit', 'large-limit', 'capacity', 'quota', 'callback'), 1)}


def settings(phase):
    return {'locale': LOCALES[phase], 'fallbacks': ['fr'] if phase == 2 else [],
            'pseudo': phase == 2}


def tables():
    result = []
    for table in TABLES:
        messages = {'empty': {locale: '' for locale in ('en', 'fr', 'fr-CA', 'zh-Hant')}}
        for n in range(65):
            messages[f'k{n:03}'] = {'en': 'English'}
            if n % 2 == 0:
                messages[f'k{n:03}'].update(fr='Francais', **{'zh-Hant': '中文'})
        result.append({'id': table, 'name': 'Coverage ' + table[-1],
                       'source_locale': 'en', 'messages': messages})
    return result


def rows(phase):
    """Resolve from authored values, independently of the guests' ordinal logic."""
    config = settings(phase)
    requested = config['locale']
    candidates = [requested]
    while '-' in candidates[-1]:
        candidates.append(candidates[-1].rsplit('-', 1)[0])
    candidates += config['fallbacks'] + ['en']
    result = []
    for table in sorted(tables(), key=lambda item: item['id']):
        for key, translations in sorted(table['messages'].items()):
            if requested in translations:
                continue  # An intentionally empty direct translation is present.
            resolved = next(locale for locale in candidates if locale in translations)
            result.append({'table_id': table['id'], 'key': key,
                           'requested_locale': requested, 'resolved_locale': resolved,
                           'kind': 'translation'})
    return result


def digest(rows):
    value = 2166136261
    for row in rows:
        for field in ('table_id', 'key', 'requested_locale', 'resolved_locale', 'kind'):
            for byte in row[field].encode() + b'\0':
                value = ((value ^ byte) * 16777619) & 0xffffffff
    return value


def position(tick):
    if tick == 0:
        return [0, 0, 0]
    coverage = rows((tick - 1) % 4)
    return [tick, len(coverage), digest(coverage)]


def observation(start, end):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
            'checks': [{'name': f'coverage-{tick}', 'tick': tick,
                        'expect': {'type': 'vector', 'entity': ENTITY, 'property': 'translation',
                                   'value': position(tick), 'tolerance': 0}}
                       for tick in range(start, end + 1)]}


def recording(ticks, failure=None):
    if failure is not None and failure not in FAILURES:
        raise ValueError('Unknown fixed refusal')
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0,
            'ticks': ticks, 'frames': [] if failure is None else [
                {'tick': 1, 'events': [{'type': 'key', 'code': failure, 'down': True}]}]}
