#!/usr/bin/env python3
"""replay_plan.py RECORDING.json OUT_PLAN.json NAME CAPTURE_TICK[:LABEL] ...: turn an input recording
(incant-input frames) into an offscreen-harness case that feeds the same events on the same
ticks and captures frames at the requested ticks. The harness counts its first tick as 1."""
import json
import sys


def main():
    recording = json.load(open(sys.argv[1], encoding='utf-8'))
    out, name = sys.argv[2], sys.argv[3]
    captures = {}
    for item in sys.argv[4:]:
        tick, _, label = item.partition(':')
        captures[int(tick)] = label or f't{tick}'
    events = {f['tick']: f['events'] for f in recording['frames']}
    marks = sorted(set(events) | set(captures) | {recording['ticks']})
    steps, tick = [], 1
    for mark in marks:
        if mark < tick:
            continue
        if mark > tick:
            steps.append({'ticks': mark - tick})
            tick = mark
        step = {'ticks': 1}
        if tick in events:
            step['events'] = events[tick]
        if tick in captures:
            step['capture'] = captures[tick]
        steps.append(step)
        tick += 1
    json.dump({'cases': [{'name': name, 'viewport': [640, 480], 'scale': 2.0, 'steps': steps}]},
              open(out, 'w', encoding='utf-8'), indent=1)
    print(json.dumps({'steps': len(steps), 'last_tick': tick - 1, 'captures': captures}))


if __name__ == '__main__':
    main()
