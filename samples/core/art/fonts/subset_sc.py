"""Reproduce CoreSampleSC-Regular.ttf: a Noto Sans SC subset for the HUD's zh-Hans text.

Usage (fonttools==4.60.1, a build-time tool only; never a runtime dependency):
  python subset_sc.py UPSTREAM_NotoSansSC[wght].ttf ../../ui/strings.json OUTPUT.ttf

Upstream identity, licence and digests are recorded in provenance.json. Only the Han
characters and CJK punctuation used by the string table (plus a small punctuation
reserve) are kept; Latin, digits and symbols come from Inter earlier in the chain.
The family is renamed so the reduced subset is never mistaken for the full family.
"""
import json
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

RESERVE = '，。：；！？（）、—…'


def glyph_text(strings_path):
    table = json.load(open(strings_path, encoding='utf-8'))
    chars = set(RESERVE)
    for message in table['messages'].values():
        chars.update(c for c in message.get('zh-Hans', '') if ord(c) > 0x2E7F)
    return ''.join(sorted(chars))


def main(upstream, strings_path, output):
    text = glyph_text(strings_path)
    font = TTFont(upstream, recalcTimestamp=False)
    options = subset.Options()
    options.notdef_outline = True
    options.recalc_timestamp = False
    options.layout_features = ['*']
    subsetter = subset.Subsetter(options=options)
    subsetter.populate(text=text)
    subsetter.subset(font)
    font = instantiateVariableFont(font, {'wght': 400}, inplace=True)
    names = {1: 'Core Sample SC', 3: 'CoreSampleSC-Regular', 4: 'Core Sample SC Regular',
             6: 'CoreSampleSC-Regular', 16: 'Core Sample SC'}
    for record in font['name'].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    font.save(output, reorderTables=True)
    print(json.dumps({'glyph_text': text, 'characters': len(text)}, ensure_ascii=False))


if __name__ == '__main__':
    main(*sys.argv[1:4])
