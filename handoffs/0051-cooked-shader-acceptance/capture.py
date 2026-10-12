#!/usr/bin/env python3
"""Fixed, root-granted capture commands for Claude's 0051 review only."""
import argparse
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PACKET = Path(__file__).resolve().parent
TOOLS = ROOT / 'artifacts/tools/0051'
OUTPUT = ROOT / 'artifacts/0051-cooked-shader-acceptance'
CASES = {
    'output': ('output-v1', 'incant-render-tests', 'hdr_output::hdr_values_survive_until_tone_mapping_and_transfer_runs_once', 'INCANT_HDR_EVIDENCE', 1),
    'lookdev': ('geometry-v2', 'model_gpu', 'shadows::lookdev::directional_shadow_lookdev_contact_slopes_and_curves', 'INCANT_SHADOW_EVIDENCE', 4),
    'masks': ('geometry-v2', 'model_gpu', 'shadows::alpha_masked_and_reflected_casters_preserve_holes_and_blend_casters_fail_explicitly', 'INCANT_SHADOW_EVIDENCE', 5),
    'diagnostic': ('geometry-v2', 'incant_render', 'diagnostic_output::diagnostic_vertices_draw_reflected_geometry_and_preserve_display_formats', 'INCANT_HDR_EVIDENCE', 2),
}

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case', choices=CASES)
    parser.add_argument('mode', choices=('default', 'cooked'))
    args = parser.parse_args()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with (TOOLS / 'capture.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        gate = TOOLS / 'gpu-review-ready.json'
        released = TOOLS / 'RELEASED.json'
        assert not released.exists(), 'interval released; no further calls authorized'
        grant = json.loads(gate.read_text())
        assert grant['root_grant'] is True
        assert grant['prepared_inputs_sha256'] == digest(PACKET / 'prepared-inputs.json')
        assert grant['capture_helper_sha256'] == digest(Path(__file__))
        ledger_path = OUTPUT / 'children.json'
        ledger = json.loads(ledger_path.read_text()) if ledger_path.exists() else []
        name = f'{args.case}-{args.mode}'
        assert len(ledger) < 8 and all(r['name'] != name for r in ledger)
        assert all(r.get('exit_code') == 0 and r.get('capture_count_matches') for r in ledger), 'earlier child failed or interrupted'
        group, suffix, test, evidence_var, expected = CASES[args.case]
        mode = 'cooked-geometry' if group == 'geometry-v2' and args.mode == 'cooked' else args.mode
        relative = f'{group}/{mode}-{suffix}'
        binary = TOOLS / relative
        staged = json.loads((PACKET / 'staged-inputs.json').read_text())
        assert digest(binary) == staged['binaries'][relative]
        directory = OUTPUT / name
        directory.mkdir(exist_ok=False)
        env = {k: v for k, v in os.environ.items() if not (k.startswith('INCANT_') and k.endswith('_EVIDENCE')) and k != 'INCANT_SHADOW_FIXTURE_COPY'}
        env[evidence_var] = str(directory)
        command = [str(binary), test, '--exact', '--ignored', '--test-threads=1', '--nocapture']
        record = {'name': name, 'command': command, 'binary_sha256': digest(binary), 'started_at': now(), 'expected_capture_count': expected}
        ledger.append(record)
        save(ledger_path, ledger)
        with (directory / 'child.log').open('xb') as log:
            child = subprocess.Popen(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            record['pid'] = child.pid
            save(ledger_path, ledger)
            try:
                record['exit_code'] = child.wait(timeout=60)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                record['exit_code'] = child.returncode
                record['timeout'] = True
        record['ended_at'] = now()
        record['reaped'] = True
        record['files'] = {str(p.relative_to(directory)): digest(p) for p in directory.rglob('*') if p.is_file()}
        record['capture_count_matches'] = len(list(directory.glob('*.png'))) == expected
        save(ledger_path, ledger)
        failed = record['exit_code'] != 0 or not record['capture_count_matches']
        if failed or len(ledger) == 8:
            gate.unlink()
            save(released, {'released_at': now(), 'reason': 'failure' if failed else 'all eight bounded children reaped', 'ledger_sha256': digest(ledger_path), 'pids': [r.get('pid') for r in ledger], 'all_reaped': all(r.get('reaped') for r in ledger)})
        print(json.dumps(record, indent=2))
        if released.exists():
            print('GRAPHICS RELEASED; report this before writing the final report.')
        if failed:
            raise SystemExit(1)

if __name__ == '__main__':
    main()
