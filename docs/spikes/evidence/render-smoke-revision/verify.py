#!/usr/bin/env python3
"""Read-only replay of revision/renderer receipts, never native execution."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binary', action='store_true', help='rehash retained historical executable')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / 'manifest.json').read_text())
    archive = (root / 'artifacts.json.gz').read_bytes()
    assert hashlib.sha256(archive).hexdigest() == manifest['archive_sha256']
    payloads = {h: base64.b64decode(data, validate=True) for h, data in json.loads(gzip.decompress(archive)).items()}
    for h, data in payloads.items():
        assert hashlib.sha256(data).hexdigest() == h
    for row in manifest['records']:
        data = payloads[row['sha256']]
        assert len(data) == row['bytes']
        if args.originals:
            assert Path(row['original']).read_bytes() == data, row['original']
    def record(suffix):
        row = next(r for r in manifest['records'] if r['original'].endswith(suffix))
        return payloads[row['sha256']]
    actual = json.loads(record('/rpc-001/result.json'))
    assert actual['passed'] and actual['native_processes'] == 4 and actual['edit_revisions'] == [1, 2]
    assert actual['stale_revision_refused'] and actual['after_refusal_revision'] == 2
    assert actual['project_bytes_preserved_on_refusal'] and actual['rpc_error_code_present'] is False
    assert actual['rpc_error']['message'] == 'document revision conflict: expected 0, current 2'
    ledger = json.loads(record('/rpc-001/ledger.json'))
    assert len(ledger) == 4 and all(r['exit_code'] == 0 and r['timeout_seconds'] == 30 for r in ledger)
    assert [r['argv'][1] for r in ledger] == ['init', 'rpc', 'rpc', 'rpc']
    requests = [json.loads(r['stdin'].splitlines()[0]) for r in ledger[1:]]
    assert [r['params']['expected_revision'] for r in requests] == [0, 1, 0]
    assert json.loads(record('/render-smoke-revision-root-001/1.json'))['exit'] == 0
    report = json.loads(record('/render/report.json'))
    assert report['png_transport_validated'] and report['hierarchy_matches_flattened'] and report['hierarchy_differs_from_initial']
    assert report['imported_models']['source_independent'] and report['imported_models']['reimport_changes_pixels']
    assert report['visual_review'] is False and report['native_window_composition_review'] is False
    assert 'Ran 9 tests' in record('/python-tests-001.log').decode() and '\nOK\n' in record('/python-tests-001.log').decode()
    assert manifest['combined_test_breakdown']['producer']['modules'] == {'tools.tests.test_render_geometry': 6, 'tools.tests.test_ci_profile': 3}
    assert manifest['combined_test_breakdown']['root_focused']['total'] == 6
    assert 'Ran 6 tests' in record('/render-smoke-revision-root-001/0.log').decode()
    for row in manifest['combined_test_breakdown']['producer']['bound_profile_inputs']:
        assert hashlib.sha256(payloads[row['sha256']]).hexdigest() == row['sha256']
        assert row['source'] == manifest['source']
    supplement = manifest['source_relevance_supplement']
    assert supplement['original_preflight_preserved'] is True and supplement['additional_change_test_only'] is True
    assert len(supplement['expanded_delta_paths']) == 7 and supplement['additional_path'] == 'crates/incant_cmd/tests/prefabs.rs'
    assert manifest['hosted_linux_repair_pending'] is True and manifest['performance_or_phase_gate'] is False
    if args.binary:
        q = manifest['native_binary_qualification']
        assert hashlib.sha256(Path(q['binary']).read_bytes()).hexdigest() == q['binary_sha256']
    print(json.dumps({'verified': True, 'producer_python_tests': 9, 'producer_geometry_tests': 6, 'producer_profile_tests': 3, 'root_geometry_tests': 6, 'actual_rpc_processes': 4, 'root_historical_mac_renderer_smoke': 'pass', 'hosted_linux_repair': 'pending', 'performance_or_phase_gate': False}))


if __name__ == '__main__':
    main()
