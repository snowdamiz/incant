"""Summarizes the two probe runs into evidence/webkit-model-field-probe.json.
Usage: python3 -I summarize.py <single.json> <sweep.json>"""
import json
import re
import sys

single = json.load(open(sys.argv[1]))
sweep = json.load(open(sys.argv[2]))
by_slack = {}
for field in sweep["fields"]:
    match = re.match(r"calc\((\d+)ch \+ (\d+)px\)", field["width"])
    if field["id"] is None and match:
        by_slack.setdefault(match.group(2), []).extend([field["n"]] if field["overflows"] else [])
print(json.dumps({
    "what": "Agent composer model field (agent.css .composer__model) reproduced with the bundled JetBrains Mono in an offscreen system WKWebView (never shown). scrollWidth > clientWidth means WebKit ellipsizes the text.",
    "engine": "system WebKit, user agent " + single["userAgent"],
    "font_loaded": single["fontLoaded"],
    "text_width_px_gpt_6_astra": single["textWidth"],
    "single": {f["id"]: {k: f[k] for k in ("width", "clientWidth", "scrollWidth", "overflows")} for f in single["fields"]},
    "sweep_overflowing_lengths_by_slack_px": by_slack,
}, indent=1))
