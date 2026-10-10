#!/usr/bin/env python3
"""Require independent hosted evidence before claiming cross-desktop parity."""
import argparse
import json
from pathlib import Path


def compare_hosts(reports):
    if len(reports) != 3 or {r['host']['system'] for r in reports} != {'Darwin', 'Linux', 'Windows'}:
        raise ValueError('expected exactly one passing report from each desktop OS')
    expected = None
    browsers = 0
    for report in reports:
        if report['status'] != 'passed' or report['trials'] < 3:
            raise ValueError('incomplete repeated-process evidence')
        reference = {r['scenario']: r['sha256'] for r in report['reference']}
        if set(reference) != {0, 1} or (expected is not None and reference != expected):
            raise ValueError('cross-host full body-state stream mismatch')
        expected = reference
        modes = [('serial', 0), ('parallel', 1), ('parallel', 4)]
        if report['browser_executed']:
            browsers += 1
            modes.append(('browser-wasm-serial', 0))
            if len(report['browser']) != report['trials']:
                raise ValueError('missing browser process evidence')
        wanted = {(trial, mode, workers, scenario) for trial in range(report['trials'])
                  for mode, workers in modes for scenario in (0, 1)}
        actual = {(r['trial'], r['mode'], r['workers'], r['scenario']) for r in report['records']}
        if actual != wanted or len(report['records']) != len(wanted):
            raise ValueError('missing or duplicated execution mode/trial/scenario')
        if any(r['sha256'] != reference[r['scenario']] for r in report['records']):
            raise ValueError('report contains a nonmatching state stream')
    if browsers < 1:
        raise ValueError('actual browser/WASM execution evidence is required')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    compare_hosts([json.loads(path.read_text()) for path in sorted(args.directory.glob('*/report.json'))])
    print('Public body-state streams match across three hosted desktops and Chromium/WASM.')
    print('Two synthetic workloads; no device, hidden solver-state or performance gate claim.')


if __name__ == '__main__':
    main()
