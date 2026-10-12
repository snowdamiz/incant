"""check.py <state.json> <width> <min-height> <max-height> <state-word>: refuse a capture unless
the state read shows what that capture is for. Exits 1, naming the first unmet condition:

  - the window is <width> wide and between <min-height> and <max-height> tall (points; macOS
    clamps AX resizes of this window to 874 pt high, 0052);
  - the Viewport header shows <state-word> (Live, Playing, Paused...);
  - the Animation component holds a text field named Clip whose value is 0, the authored value;
  - no node anywhere in the window holds an unsupported-type notice or a raw-value fallback.

Reads one state.py output; posts no input. Every capture of the 0053 recheck counts against the
packet's three, so cap.sh runs only after this passes.
"""

import json
import sys


def main():
    state = json.load(open(sys.argv[1], encoding='utf-8'))
    width, low, high, word = float(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4]), sys.argv[5]
    window = state['window']
    problems = []
    if window['w'] != width or not low <= window['h'] <= high:
        problems.append(f'window is {window["w"]:.0f} x {window["h"]:.0f}')
    if word not in state['words']:
        problems.append(f'header shows {state["words"]}, not {word}')
    clip = [n for n in state['animation'] if n['role'] == 'AXTextField' and 'Clip' in (n['title'], n['label'])]
    if len(clip) != 1 or clip[0]['value'] != '0':
        problems.append(f'Clip text field: {clip}')
    if state['unsupported']:
        problems.append(f'fallback text: {state["unsupported"]}')
    if problems:
        raise SystemExit('no capture: ' + '; '.join(problems))
    print(json.dumps({'checked': state['label'], 'window': [window['w'], window['h']], 'words': state['words'],
                      'clip': clip[0]['box']}))


if __name__ == '__main__':
    main()
