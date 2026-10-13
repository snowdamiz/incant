#!/usr/bin/env python3
"""Check 0065 adapter reports against the accepted CPU routes (CPU only).

Usage: python3 -B route_check.py ACCEPTED LOCALIZED REPORT.json [...]

ACCEPTED is Astra's retained shipping proof (per-route `<name>.json` reports and
`.pcm`); LOCALIZED is the engine-resolved text table written by verify_bindings.py
prepare. For every report:
  - each generation's game logs equal the accepted fresh route's logs, or are its
    exact prefix through the last simulated tick when the capture stops early;
  - the PCM hash over every host step equals the accepted route PCM prefix;
  - every captured label equals binding_oracle.labels() for the logged state before
    that simulation tick, and won/lost sit in Result only in their own phase;
  - a capture after a restart has the same widgets, layout and drawn text as the
    fresh capture at the same simulation tick (pixels are compared separately).
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'samples/core'))
import binding_oracle as oracle  # noqa: E402
import bindings  # noqa: E402

# Route -> the fresh accepted recordings of each generation, in order.
GENERATIONS = {'gamepad-restart-lose': ['win', 'lose-short'], 'lose-restart-win': ['lose-short', 'win'],
               'win-restart-lose': ['win', 'lose-short'], 'locked': ['locked'], 'lose-short': ['lose-short'],
               'idle': ['idle'], 'win': ['win']}
LOCALE = {'en': 'en', 'ar': 'ar', 'zh-Hans': 'zh-Hans', 'en-XA': 'en-XA'}


def state(entries, before):
    health, count, phase = 3, 0, 'playing'
    for row in entries:
        if row['tick'] >= before:
            break
        message = row['message']
        if message.startswith('core.damage '):
            health = int(message.rsplit('=', 1)[1])
        if message.startswith('core.pickup '):
            count += 1
        if message.startswith('core.won '):
            phase = 'won'
        if message.startswith('core.lost '):
            phase = 'lost'
    return health, count, phase


def fingerprint(capture):
    return {'widgets': capture['widgets'], 'layout': capture['layout'],
            'drawn': [{k: v for k, v in d.items()} for d in capture['drawn']]}


def main():
    accepted, localized = Path(sys.argv[1]), json.loads(Path(sys.argv[2]).read_text(encoding='utf-8'))
    ok = True
    for path in map(Path, sys.argv[3:]):
        report = json.loads(path.read_text(encoding='utf-8'))
        run = path.parent.name
        plan = json.loads((ROOT / 'handoffs/0065-core-live-hud/tools/plans.json').read_text())['runs'][run]
        texts = localized[LOCALE[plan['variant']]]
        names = GENERATIONS[plan['route']]
        failures = []
        logs = report['generation_logs']
        if len(logs) != len(names) and not (len(logs) < len(names)):
            failures.append('unexpected generation count')
        last_ticks = [b['simulation_tick'] for b in report['boundaries']] + [report['final_simulation_tick']]
        for index, (entries, name) in enumerate(zip(logs, names)):
            reference = json.loads((accepted / f'{name}.json').read_text(encoding='utf-8'))['logs']
            if entries['dropped'] or reference['dropped']:
                failures.append(f'generation {index + 1} dropped logs')
            prefix = [row for row in reference['entries'] if row['tick'] <= last_ticks[index]]
            if entries['entries'] != prefix:
                failures.append(f'generation {index + 1} logs differ from accepted {name}')
        pcm = (accepted / f'{plan["route"]}.pcm').read_bytes()
        expected_pcm = hashlib.sha256(pcm[:report['pcm']['frames'] * 8]).hexdigest()
        if report['pcm']['sha256'] != expected_pcm or report['pcm']['frames'] * 8 > len(pcm):
            failures.append('PCM differs from the accepted route prefix')
        fresh = {}
        for capture in report['captures']:
            entries = logs[capture['generation'] - 1]['entries']
            health, count, phase = state(entries, capture['simulation_tick'])
            expected = oracle.labels(texts, health, count)
            widgets = capture['widgets']
            for owner, text in expected.items():
                if widgets[owner]['text'] != text:
                    failures.append(f'{capture["name"]}: {owner} shows {widgets[owner]["text"]!r}, expected {text!r}')
            for role in ('won', 'lost'):
                parent = bindings.HUD['result' if role == phase else 'inactive']
                if widgets[bindings.HUD[role]]['parent'] != parent:
                    failures.append(f'{capture["name"]}: {role} parent {widgets[bindings.HUD[role]]["parent"]}')
            key = capture['simulation_tick']
            if capture['generation'] == 1:
                fresh[key] = capture
            elif key in fresh and fingerprint(fresh[key]) != fingerprint(capture):
                failures.append(f'{capture["name"]}: restarted HUD differs from fresh tick {key}')
        restarts = [c for c in report['captures'] if c['generation'] > 1]
        compared = [c['name'] for c in restarts if c['simulation_tick'] in fresh]
        row = {'run': run, 'variant': plan['variant'], 'route': plan['route'], 'generations': len(logs),
               'boundaries': report['boundaries'], 'pcm_frames': report['pcm']['frames'],
               'pcm_sha256': report['pcm']['sha256'], 'accepted_pcm_prefix_sha256': expected_pcm,
               'restarted_equal_fresh': compared, 'captures': len(report['captures']), 'failures': failures}
        ok &= not failures
        print(json.dumps(row, ensure_ascii=False))
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
