"""Reproduce the numerical fixture with fonttools==4.60.1; not a build dependency.
Usage: python subset_cjk.py UPSTREAM_TTF OUTPUT_TTF
Upstream identity/license are recorded in cjk-provenance.json.
"""
import sys
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
font = TTFont(sys.argv[1], recalcTimestamp=False)
options = subset.Options()
options.notdef_outline = True
options.recalc_timestamp = False
subsetter = subset.Subsetter(options=options)
subsetter.populate(text=' 中文日本語一')
subsetter.subset(font)
font = instantiateVariableFont(font, {'wght': 400}, inplace=True)
# Distinguish a reduced test fixture from the complete upstream family.
for record in font['name'].names:
    if record.nameID in (1, 3, 4, 6, 16):
        record.string = 'IncantCJKFixture'.encode(record.getEncoding())
font.save(sys.argv[2], reorderTables=True)
