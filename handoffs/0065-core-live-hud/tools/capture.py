#!/usr/bin/env python3
"""Run one 0065 offscreen route through a frozen per-variant adapter (GPU, serial).

Usage: python3 -B capture.py RUN_ID [RUN_ID ...]

Runs are declared in tools/plans.json. Each run names a package variant, an accepted
route from evidence/routes, a viewport/scale and host-step captures; `frame: false`
entries record the public runtime snapshot only. Before each child this refuses unless
lease.py's guard passes, the adapter matches the hash recorded in its package.json,
the route matches routes.json, and every retained frame so far plus this run's stays
within 24 frames of at most 960x540. The child holds the exclusive GPU lock and is
bounded to 300 s; each capture's recorded duration must stay under 120 s.
"""
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lease  # noqa: E402

PLANS = HERE / 'plans.json'
CAPTURES = lease.EVIDENCE / 'captures'
ROUTE_TIMEOUT = 300
CAPTURE_SECONDS = 120


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def retained():
    return sorted(CAPTURES.rglob('*.rgba')) if CAPTURES.exists() else []


def run(run_id, spec):
    lease.guard()  # refuse before reading or writing anything for this run
    package = lease.EVIDENCE / 'packages' / spec['variant']
    manifest = json.loads((package / 'package.json').read_text(encoding='utf-8'))
    adapter = package / 'adapter' / 'zz_0065_live_hud'
    if digest(adapter) != manifest['adapter_sha256']:
        raise lease.Refused(f'{run_id}: adapter differs from its frozen package record')
    routes = json.loads((lease.EVIDENCE / 'routes/routes.json').read_text(encoding='utf-8'))
    route = lease.EVIDENCE / 'routes' / (spec['route'] + '.json')
    if digest(route) != routes[spec['route']]['sha256'] or not routes[spec['route']]['equals_accepted']:
        raise lease.Refused(f'{run_id}: route is not the recorded accepted recording')
    frames = sum(1 for c in spec['captures'] if c['frame'])
    size = [round(v * spec['scale']) for v in spec['viewport']]
    if size[0] > 960 or size[1] > 540:
        raise lease.Refused(f'{run_id}: {size} exceeds 960x540')
    if len(retained()) + frames > 24:
        raise lease.Refused(f'{run_id}: {len(retained())} retained + {frames} planned exceeds 24 frames')
    CAPTURES.mkdir(exist_ok=True)
    out = CAPTURES / run_id
    if out.exists():
        raise lease.Refused(f'{run_id}: output exists')
    plans = lease.EVIDENCE / 'plans'
    plans.mkdir(exist_ok=True)
    plan = plans / f'{run_id}.json'
    plan.write_text(json.dumps({'viewport': spec['viewport'], 'scale': spec['scale'], 'ticks': spec['ticks'],
                                'max_frames': frames, 'captures': spec['captures']}, indent=1) + '\n',
                    encoding='utf-8')
    scratch = lease.EVIDENCE / 'scratch' / run_id
    scratch.mkdir(parents=True)
    status, stdout, stderr = lease.run(
        [adapter, package / 'player/data', route, plan, out], label=f'gpu-{run_id}', timeout=ROUTE_TIMEOUT,
        gpu=True, cwd=scratch, env={'PATH': '/usr/bin:/bin', 'HOME': str(scratch), 'TMPDIR': str(scratch)})
    if status != 0:
        raise lease.Refused(f'{run_id}: adapter exit {status}: ' + stderr.read_text(encoding='utf-8')[-3000:])
    report = json.loads((out / 'report.json').read_text(encoding='utf-8'))
    slow = [c['name'] for c in report['captures'] if c['capture_seconds'] > CAPTURE_SECONDS]
    written = sorted(p.name for p in out.glob('*.rgba'))
    row = {'run': run_id, 'variant': spec['variant'], 'route': spec['route'], 'adapter_sha256': digest(adapter),
           'plan_sha256': digest(plan), 'report_sha256': digest(out / 'report.json'), 'frames': written,
           'frame_sha256': {name: digest(out / name) for name in written},
           'elapsed_seconds': report['elapsed_seconds'], 'slow_captures': slow,
           'adapter': report['adapter'], 'backend': report['backend']}
    with (lease.EVIDENCE / 'captures.jsonl').open('a', encoding='utf-8') as stream:
        stream.write(json.dumps(row) + '\n')
    if len(written) != frames or report['frames_written'] != frames or slow:
        raise lease.Refused(f'{run_id}: frame count or capture duration outside bounds: {row}')
    return row


def main():
    specs = json.loads(PLANS.read_text(encoding='utf-8'))['runs']
    try:
        for run_id in sys.argv[1:]:
            print(json.dumps(run(run_id, specs[run_id])))
    except lease.Refused as error:
        print(f'refused: {error}', file=sys.stderr)
        sys.exit(3)


if __name__ == '__main__':
    main()
