#!/usr/bin/env python3
"""Run 0069 adapter routes: layout snapshots (no GPU) and single GPU frames. DRAFT.

  capture.py layout                  the 12 layout runs in plans.json (0 frames, no GPU device)
  capture.py RUN_ID [RUN_ID ...]     named review-*/final-* runs from plans.json

Nothing is reused: every planned frame is rendered by its own run, 22 in total
(4 review, 18 final) under the 24-frame appearance ceiling. Before each child,
lease.freeze() must still match the latest freeze generation, the package must
have been built under that same generation with every inventoried file unchanged
(player, cooked data, adapter, producer receipts), and the route must equal
routes.json (binding_oracle recordings, byte-identical to the accepted proofs).
Every adapter child, layout-only or not, is a GPU child for lease.run: it first
takes the global native GPU lock, then the packet lock, refusing at once if either
is busy (root schedules GPU ownership), and holds both through the debit, launch,
kill/reap and accounting. A frame run's planned
frames are debited to the `appearance` ceiling under those locks before the
child starts and are never refunded, so failed or discarded draws count. Each
child runs in a constructed minimal environment and is bounded to 120 s.

Afterwards the report must show: for a layout run, no GPU adapter and zero GPU
frames; for a frame run, exactly the planned frames, one consumer accept for
each frame capture, each frame at most 960x540, and the same adapter/backend as
the first frame run.
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
GPU = lease.EVIDENCE / 'gpu.json'
EXAMPLE = 'zz_0069_result_hud'
MAX_SIZE = (960, 540)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def layout_specs(plans):
    layout = plans['layout']
    return {f'layout-{variant}-{w}x{h}': {'phase': 'layout', 'variant': variant, 'route': layout['route'],
                                          'viewport': [w, h], 'scale': 1.0, 'ticks': layout['ticks'],
                                          'max_frames': 0, 'captures': layout['captures']}
            for variant in layout['variants'] for w, h in layout['viewports']}


def check(report, spec, frames, written):
    """The report's own accounting of GPU work, against the plan."""
    if frames == 0:
        if report['gpu_adapter'] is not None or report['gpu_frames_rendered'] != 0 or written:
            return 'a layout-only run created a GPU device or drew a frame'
        return None
    if report['gpu_frames_rendered'] != frames or report['frames_written'] != frames or len(written) != frames:
        return 'rendered frames differ from the plan'
    rows = [row for row in report['captures'] if 'rgba' in row]
    if len(rows) != frames or any(row['gpu']['consumer_frames'] != 1 for row in rows):
        return 'a frame capture was not exactly one consumer accept'
    if any(row['size'][0] > MAX_SIZE[0] or row['size'][1] > MAX_SIZE[1] for row in rows):
        return 'a frame exceeds 960x540'
    return None


def package_inventory(package):
    return {str(p.relative_to(package)): digest(p) for p in sorted(package.rglob('*'))
            if p.is_file() and p.relative_to(package).parts[0] in ('player', 'adapter', 'producer')}


def run(run_id, spec):
    frozen = lease.freeze()  # refuses unless the latest freeze generation still holds
    package = lease.EVIDENCE / 'packages' / spec['variant']
    manifest = json.loads((package / 'package.json').read_text(encoding='utf-8'))
    if manifest['freeze'] != {'generation': frozen['generation'], 'sha256': frozen['sha256']}:
        raise lease.Refused(f'{run_id}: package was built under another freeze generation')
    if manifest['adapter_source_sha256'] != frozen['identity']['tools']['result_hud.rs']:
        raise lease.Refused(f'{run_id}: package adapter source differs from the frozen tool')
    if package_inventory(package) != manifest['inventory']:
        raise lease.Refused(f'{run_id}: a packaged file changed since the build')
    adapter = package / 'adapter' / EXAMPLE
    if digest(adapter) != manifest['adapter_sha256']:
        raise lease.Refused(f'{run_id}: adapter differs from its package record')
    routes = json.loads((lease.EVIDENCE / 'routes/routes.json').read_text(encoding='utf-8'))
    route = lease.EVIDENCE / 'routes' / (spec['route'] + '.json')
    if digest(route) != routes[spec['route']]['sha256'] or not routes[spec['route']]['equals_accepted']:
        raise lease.Refused(f'{run_id}: route is not the recorded accepted recording')
    frames = sum(1 for c in spec['captures'] if c['frame'])
    size = [round(v * spec['scale']) for v in spec['viewport']]
    if frames != spec['max_frames'] or size[0] > MAX_SIZE[0] or size[1] > MAX_SIZE[1]:
        raise lease.Refused(f'{run_id}: frame count or size outside the plan bounds')
    out = CAPTURES / run_id
    if out.exists():
        raise lease.Refused(f'{run_id}: output exists')
    plans = lease.EVIDENCE / 'plans'
    plans.mkdir(exist_ok=True)
    plan = plans / f'{run_id}.json'
    plan.write_text(json.dumps({'viewport': spec['viewport'], 'scale': spec['scale'], 'ticks': spec['ticks'],
                                'max_frames': frames, 'captures': spec['captures']}, indent=1) + '\n',
                    encoding='utf-8')
    CAPTURES.mkdir(exist_ok=True)
    scratch = lease.scratch(run_id)
    # The debit happens inside lease.run, under both locks, before the launch; a
    # failed or discarded draw still counts.
    status, stdout, stderr = lease.run(
        [adapter, package / 'player/data', route, plan, out], label=f'adapter-{run_id}', timeout=lease.GPU_TIMEOUT,
        cwd=scratch, env={'PATH': '/usr/bin:/bin', 'HOME': str(scratch), 'TMPDIR': str(scratch)},
        debit=('appearance', run_id, frames) if frames else None, gpu=True)
    if status != 0:
        raise lease.Refused(f'{run_id}: adapter exit {status}: ' + stderr.read_text(encoding='utf-8')[-3000:])
    report = json.loads((out / 'report.json').read_text(encoding='utf-8'))
    written = sorted(p.name for p in out.glob('*.rgba'))
    row = {'run': run_id, 'phase': spec['phase'], 'variant': spec['variant'], 'route': spec['route'],
           'viewport': spec['viewport'], 'adapter_sha256': digest(adapter), 'plan_sha256': digest(plan),
           'report_sha256': digest(out / 'report.json'), 'frames': written,
           'frame_sha256': {name: digest(out / name) for name in written},
           'gpu_frames_rendered': report['gpu_frames_rendered'], 'gpu_adapter': report['gpu_adapter'],
           'elapsed_seconds': report['elapsed_seconds'],
           'freeze': {'generation': frozen['generation'], 'sha256': frozen['sha256']}}
    with (lease.EVIDENCE / 'captures.jsonl').open('a', encoding='utf-8') as stream:
        stream.write(json.dumps(row) + '\n')
    problem = check(report, spec, frames, written)
    if problem:
        raise lease.Refused(f'{run_id}: {problem}: {row}')
    if frames:
        if GPU.exists():
            if json.loads(GPU.read_text(encoding='utf-8')) != report['gpu_adapter']:
                raise lease.Refused(f'{run_id}: GPU adapter/backend differs from the first frame run')
        else:
            GPU.write_text(json.dumps(report['gpu_adapter'], indent=1) + '\n', encoding='utf-8')
    return row


def main():
    plans = json.loads(PLANS.read_text(encoding='utf-8'))
    args = sys.argv[1:]
    try:
        if args == ['layout']:
            selected = layout_specs(plans)
        elif args and not any(a.startswith('-') for a in args):
            selected = {run_id: plans['runs'][run_id] for run_id in args}
        else:
            print(__doc__)
            sys.exit(2)
        for run_id, spec in selected.items():
            print(json.dumps(run(run_id, spec)))
    except lease.Refused as error:
        print(f'refused: {error}', file=sys.stderr)
        sys.exit(3)


if __name__ == '__main__':
    main()
