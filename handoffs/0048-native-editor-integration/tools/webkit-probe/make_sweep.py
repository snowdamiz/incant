"""Writes model-field-sweep.html: model-field.html plus one model field per name length 4..32
for each width slack (10, 12, 13, 14 px past the characters). Usage: python3 -I make_sweep.py <out-dir>"""
import os
import sys

here = os.path.dirname(os.path.abspath(__file__))
html = open(os.path.join(here, "model-field.html")).read()
rows = []
for slack in (10, 12, 13, 14):
    for n in range(4, 33):
        value = ("gpt-6-astra-preview-" * 3)[:n]
        rows.append(f'<div class="bar"><input class="model" data-slack="{slack}" value="{value}" style="width: calc({n}ch + {slack}px)"></div>')
html = html.replace('<span class="measure"', "\n".join(rows) + '\n<span class="measure"')
open(os.path.join(sys.argv[1], "model-field-sweep.html"), "w").write(html)
