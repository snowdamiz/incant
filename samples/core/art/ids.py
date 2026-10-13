"""Stable, readable identifiers for every authored Core Sample object.

All IDs are canonical 26-character Crockford base32 text accepted as ULIDs:
'01CS' (Core Sample) + a two-letter kind + a zero-padded decimal index. They never
change between builds, so widget, entity, asset and table references are stable.
"""
KINDS = {'entity': 'EN', 'widget': 'WD', 'part': 'CP', 'script': 'SC', 'table': 'ST', 'asset': 'AS',
         'scene': 'SN', 'link': 'NK'}


def make(kind, index):
    if not 0 < index < 10 ** 20:
        raise ValueError('ID index out of range')
    value = '01CS' + KINDS[kind] + '%020d' % index
    assert len(value) == 26
    return value


def entity(index):
    return make('entity', index)


def widget(index):
    return make('widget', index)


def part(index):
    return make('part', index)


def asset(index):
    return make('asset', index)
