#!/usr/bin/env python3
"""Copy a finished review.py output into the handoff's screenshots/ and evidence/.

    python3 -B collect.py <review-output>

Replaces only this handoff's screenshots/*.png and evidence/ tree. The motion
strip crops pearl 1 and urchin 1 from the native runtime sequence at fixed ticks.
"""
import json
from pathlib import Path
import shutil
import sys

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
sys.path.insert(0, str(HERE))
import pngtool  # noqa: E402

SCREENSHOTS = {
    '01-cove-1280x720.png': 'renders/desktop-16x9-1280x720.png',
    '02-cove-logical-320x180.png': 'renders/logical-320x180.png',
    '03-window-4x3-1280x960.png': 'renders/window-4x3-1280x960.png',
    '04-phone-landscape-aspect-1920x887.png': 'renders/phone-landscape-aspect-1920x887.png',
    '05-native-runtime-tick0-960x540.png': 'native/motion/tick-00000.png',
    '06-native-hook-stung-960x540.png': 'native/hooks/tick-00001.png',
    '07-native-hook-cheer-lantern-960x540.png': 'native/hooks/tick-00002.png',
    '08-state-two-pearls-1280x720.png': 'previews/collecting-1280x720.png',
    '09-state-won-with-hook-frames-1280x720.png': 'previews/won-hooks-1280x720.png',
    '10-state-lost-with-hook-frame-1280x720.png': 'previews/lost-hook-1280x720.png',
}
EVIDENCE = {
    'review.json': 'review.json', 'scene-result.json': 'build/result.json',
    'review-package.json': 'package/package.json',
    'native/motion-request.json': 'native/motion-request.json', 'native/motion-report.json': 'native/motion/report.json',
    'native/hooks-request.json': 'native/hooks-request.json', 'native/hooks-report.json': 'native/hooks/report.json',
    'native/collecting-observe.json': 'previews/collecting-observe.json',
    'native/collecting-observation.json': 'previews/collecting-observation.json',
}
SCENARIOS = ('keyboard-win', 'arrows-win', 'gamepad-win', 'dpad-win', 'urchin-loss', 'locked-boat',
             'shore-and-cliff')
STRIP_TICKS = (0, 16, 24, 32, 40, 48, 56, 62)


def main():
    review = Path(sys.argv[1]).resolve(strict=True)
    screenshots, evidence = PACKET / 'screenshots', PACKET / 'evidence'
    for old in screenshots.glob('*.png'):
        old.unlink()
    screenshots.mkdir(exist_ok=True)
    if evidence.exists():
        shutil.rmtree(evidence)
    (evidence / 'scenarios').mkdir(parents=True)
    (evidence / 'native').mkdir()
    for name, source in SCREENSHOTS.items():
        shutil.copyfile(review / source, screenshots / name)
    rows = []
    for tick in STRIP_TICKS:
        frame = pngtool.read(review / f'native/motion/tick-{tick:05d}.png')
        rows += [row[120:300] for row in frame[126:174]] + [[(20, 21, 25, 255)] * 180] * 4
    pngtool.write(screenshots / '11-native-motion-pearl-urchin-ticks.png', pngtool.zoom(rows[:-4], 2))
    art = review / 'build/project/art'
    pngtool.write(screenshots / '12-art-sprites-x6.png', pngtool.zoom(pngtool.flatten(pngtool.read(art / 'cove-sprites.png')), 6))
    pngtool.write(screenshots / '13-art-tiles-x3.png', pngtool.zoom(pngtool.flatten(pngtool.read(art / 'cove-tiles.png')), 3))
    for name, source in EVIDENCE.items():
        shutil.copyfile(review / source, evidence / name)
    for scenario in SCENARIOS:
        for suffix in ('', '-input', '-observe'):
            shutil.copyfile(review / f'build/{scenario}{suffix}.json', evidence / f'scenarios/{scenario}{suffix}.json')
    print(json.dumps({'screenshots': sorted(p.name for p in screenshots.glob('*.png')),
                      'evidence_files': sum(1 for p in evidence.rglob('*') if p.is_file())}, indent=1))


if __name__ == '__main__':
    main()
