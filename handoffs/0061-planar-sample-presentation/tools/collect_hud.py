#!/usr/bin/env python3
"""Copy a finished hud_review.py output into screenshots/hud/ and evidence/hud/.

    python3 -B collect_hud.py <hud-review-output> <lease-state-dir>

First-pass screenshots/ and evidence/ files are historical and left untouched.
The status strip crops the top 64 rows (the HUD band) of real captures.
"""
import json
from pathlib import Path
import shutil
import sys

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
sys.path.insert(0, str(HERE))
import pngtool  # noqa: E402

SHOTS = {
    '01-en-start-1280x720.png': 'en/keyboard-win-640x360x2/tick-00002.png',
    '02-en-one-pearl-1280x720.png': 'en/keyboard-win-640x360x2/tick-00150.png',
    '03-en-two-pearls-1280x720.png': 'en/keyboard-win-640x360x2/tick-00450.png',
    '04-en-ready-1280x720.png': 'en/keyboard-win-640x360x2/tick-00560.png',
    '05-en-won-1280x720.png': 'en/keyboard-win-640x360x2/tick-00781.png',
    '06-en-lost-1280x720.png': 'en/urchin-loss-640x360x2/tick-00201.png',
    '07-en-locked-boat-1280x720.png': 'en/locked-boat-640x360x2/tick-00181.png',
    '08-en-window-4x3-start-1280x960.png': 'en/keyboard-win-640x480x2/tick-00002.png',
    '09-en-window-4x3-won-1280x960.png': 'en/keyboard-win-640x480x2/tick-00781.png',
    '10-en-phone-landscape-won-2532x1170.png': 'en/keyboard-win-844x390x3/tick-00781.png',
    '11-es-won-1280x720.png': 'es/keyboard-win-640x360x2/tick-00781.png',
    '12-fr-ready-1280x720.png': 'fr/keyboard-win-640x360x2/tick-00560.png',
    '13-de-lost-1280x720.png': 'de/urchin-loss-640x360x2/tick-00201.png',
    '14-pseudo-ready-1280x720.png': 'pseudo/keyboard-win-640x360x2/tick-00560.png',
}
STRIP = [(locale, run, tick) for locale in ('en', 'es', 'fr', 'de', 'pseudo')
         for run, tick in (('keyboard-win-640x360x2', 2), ('keyboard-win-640x360x2', 560),
                           ('keyboard-win-640x360x2', 781), ('urchin-loss-640x360x2', 201))]


def main():
    review, state = Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]).resolve(strict=True)
    shots, evidence = PACKET / 'screenshots/hud', PACKET / 'evidence/hud'
    for folder in (shots, evidence):
        if folder.exists():
            shutil.rmtree(folder)
    shots.mkdir(parents=True)
    for name, source in SHOTS.items():
        shutil.copyfile(review / source, shots / name)
    rows = []
    for locale, run, tick in STRIP:
        frame = pngtool.read(review / locale / run / f'tick-{tick:05d}.png')
        rows += [row[160:1120] for row in frame[0:64]] + [[(20, 21, 25, 255)] * 960] * 3
    pngtool.write(shots / '15-status-line-all-locales-and-states.png', rows[:-3])
    (evidence / 'scenarios').mkdir(parents=True)
    (evidence / 'frames').mkdir()
    (evidence / 'player-cpu').mkdir()
    shutil.copyfile(review / 'review.json', evidence / 'review.json')
    shutil.copyfile(review / 'build/result.json', evidence / 'scene-result.json')
    for path in sorted((review / 'build').glob('*.json')):
        if path.stem.split('-input')[0].split('-observe')[0] in (
                'keyboard-win', 'arrows-win', 'gamepad-win', 'dpad-win', 'urchin-loss',
                'locked-boat', 'shore-and-cliff'):
            shutil.copyfile(path, evidence / 'scenarios' / path.name)
    for report in sorted(review.glob('*/*/report.json')):
        locale, run = report.parent.parent.name, report.parent.name
        shutil.copyfile(report, evidence / 'frames' / f'{locale}-{run}.json')
    for log in sorted((review / 'player').glob('*.log.json')):
        shutil.copyfile(log, evidence / 'player-cpu' / log.name)
    for name in ('lease.json', 'ledger.jsonl', 'released', 'build-status.txt', 'headless-before.txt',
                 'mutations-hud.json', 'tools-tests.txt', 'rustfmt-check-initial.txt', 'rustfmt-check-final.txt',
                 'review-2-sources.txt', 'review-2-worktree-status.txt'):
        if (state / name).exists():
            shutil.copyfile(state / name, evidence / (name + '.json' if name == 'released' else name))
    for name in ('clippy-hud-frames.txt', 'clippy-hud-frames-nofeatures.txt', 'build-headless.txt'):
        lines = (state / name).read_text().splitlines()
        (evidence / name.replace('.txt', '-tail.txt')).write_text('\n'.join(lines[-3:]) + '\n')
    print(json.dumps({'screenshots': len(list(shots.iterdir())),
                      'evidence_files': sum(1 for p in evidence.rglob('*') if p.is_file())}))


if __name__ == '__main__':
    main()
