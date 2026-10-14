#!/usr/bin/env python3
"""Independent source/build/functional/profile audit; no runtime execution."""
import argparse
import base64
from datetime import datetime
import gzip
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(HERE.parent/'mac-core-fast10k'))
from verify import functional, git_source, file_digest
from build import audit as audit_build
from metrics import metrics, compare, controls
H = lambda b: hashlib.sha256(b).hexdigest()
ENGINE = lambda d: {n:s for n,s in d.items() if n.startswith(('crates/', 'sdk/', 'tools/', '.cargo/', 'third_party/')) or n in ('Cargo.toml','Cargo.lock','rust-toolchain.toml','package.json','package-lock.json')}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--verify-only', action='store_true')
    p.add_argument('--external', action='store_true')
    p.add_argument('--git-source', action='store_true')
    args = p.parse_args()
    manifest = json.loads((HERE/'manifest.json').read_text())
    for n,s in manifest['audit_files'].items():
        assert file_digest(ROOT/n) == s, n
    for n,s in manifest['owned'].items():
        assert file_digest(ROOT/n) == s, n
    compressed = (HERE/'artifacts.json.gz').read_bytes()
    assert H(compressed) == manifest['archive_sha256']
    payloads = {k:base64.b64decode(v, validate=True) for k,v in json.loads(gzip.decompress(compressed))['payloads'].items()}
    assert all(H(v) == k for k,v in payloads.items())
    rows = {r['path']:r for r in manifest['records']}
    assert len(rows) == len(manifest['records'])
    assert set(payloads) == {r['sha256'] for r in rows.values() if r['embedded']}
    for r in rows.values():
        if r['embedded']:
            assert len(payloads[r['sha256']]) == r['bytes']
        if args.external:
            original = Path(r['original'])
            assert original.stat().st_size == r['bytes'] and file_digest(original) == r['sha256'], original
    read = lambda n: payloads[rows[n]['sha256']]
    data = lambda n: json.loads(read(n))
    plan, pre = data('pair/plan.json'), data('pair/pre-execution.json')
    assert plan['order'] == ['baseline0','candidate0','candidate1','baseline1','baseline2','candidate2']
    assert plan['ticks'] == 2156 and plan['script_budget_ns'] == 2000000 and plan['regression_percent'] == 5
    assert plan['phase_gate_passed'] is False
    for key,file in [('plan_sha256','plan.json'),('runner_sha256','measure.py'),('quiet_sha256','quiet.json')]:
        assert pre[key] == rows['pair/'+file]['sha256']
    assert pre['compiler_process_names'] == [] and pre['functional_reports_equal'] is True
    quiet = data('pair/quiet.json')
    assert quiet['all_task_builders_terminal'] is quiet['gpu_released'] is True
    pre_ns = int(datetime.fromisoformat(pre['at']).timestamp()*1e9)
    freezes, exports, functionals, source_counts, builds = {}, {}, {}, {}, {}
    for label in ('baseline','candidate'):
        prefix = label+'/'
        f = data(prefix+'freeze.json'); freezes[label] = f
        assert f['source'] == plan[label+'_source'] == manifest['sources'][label]
        assert f['physics_mode'] == 'fast' and f['cpu_jobs'] == 2
        for n,s in f['authoring_files'].items():
            assert rows[prefix+'authoring/'+n]['sha256'] == s
        for key,n in [('project','core.json'),('script','gameplay.ts'),('stress_script','stress.ts')]:
            assert f[key] == rows[prefix+'authoring/'+n]['sha256']
        commands = data(prefix+'commands.json')
        assert len(commands) == 7 and all(c['exit'] == 0 for c in commands)
        for c in commands:
            assert c['started_ns'] <= c['ended_ns']
            for stream in ('stdout','stderr'):
                r = rows[prefix+c[stream]['file']]
                assert (r['sha256'],r['bytes']) == (c[stream]['sha256'],c[stream]['bytes'])
        result = data(prefix+'export-result.json'); post = data(prefix+'post-build-source.json')
        assert result['success'] is True and result['source'] == f['source'] and result['started_ns'] < result['ended_ns']
        assert post['passed'] is True and post['source'] == f['source']
        assert post['freeze_sha256'] == rows[prefix+'freeze.json']['sha256']
        assert post['export_result_sha256'] == rows[prefix+'export-result.json']['sha256']
        assert post['repository_paths_verified'] == len(f['repository_files'])
        assert post['authoring_paths_verified'] == len(f['authoring_files'])
        builds[label] = audit_build(read, data, rows, prefix)
        export = data(prefix+'package/export.json'); exports[label] = export
        assert export['player_sha256'] == result['player_sha256']
        command = data(prefix+'functional-win/command.json'); report = data(prefix+'functional-win/stdout.json')
        assert command['exit'] == 0 and command['source'] == f['source']
        assert command['core_win_verified'] is command['stress_checks_passed'] is True
        assert command['scripted_entities'] == 10000 and command['source_groups'] == 2
        assert result['ended_ns'] <= command['started_ns'] < command['ended_ns'] < pre_ns
        assert command['identity']['player_sha256'] == export['player_sha256']
        assert command['identity']['data'] == export['data']['files']
        for n,s in command['outputs'].items():
            assert rows[prefix+'functional-win/'+n]['sha256'] == s
        assert '--profile' not in command['argv'] and not read(prefix+'functional-win/stderr.log')
        functional(report, data(prefix+'functional-win/observe.json'), data('reference/core-prepared.json')['receipt'])
        assert report['pcm_sha256'] == rows[prefix+'functional-win/output.pcm']['sha256']
        assert rows[prefix+'functional-win/output.pcm']['bytes'] == report['frames']*8
        functionals[label] = report
        source_counts[label] = git_source(f['source'],f['repository_files']) if args.git_source else 0
    assert functionals['baseline'] == functionals['candidate']
    assert exports['baseline']['data'] == exports['candidate']['data']
    a,b = (freezes[k] for k in ('baseline','candidate'))
    assert ENGINE(a['repository_files']) == ENGINE(b['repository_files'])
    assert len(ENGINE(a['repository_files'])) == pre['engine_inputs_equal'] == 3872
    assert a['project'] == b['project'] and a['script'] != b['script'] and a['stress_script'] != b['stress_script']
    # Authoring journals/undo records are expected new data; assets and project remain exact.
    assets = lambda f:{n:s for n,s in f['authoring_files'].items() if n not in ('gameplay.ts','stress.ts') and not n.endswith('.journal.jsonl') and not n.startswith('.incant-source-transactions/')}
    assert assets(a) == assets(b)
    assert b['input_receipt_sha256'] == rows['candidate/prepared.json']['sha256']
    prepared = data('candidate/prepared.json')
    assert prepared['binary_sha256'] == rows['reference/authoring-binary']['sha256']
    assert prepared['public_undo_redo_reopen'] is prepared['original_unchanged'] is True
    assert prepared['only_change'] == ['gameplay.ts','stress.ts'] and len(prepared['transaction']) == 2
    for key in ('public-source','public-reopen'):
        r = data('candidate/'+key+'.json')
        assert r['exit_code'] == 0 and r['failure'] is None and r['owned_process_reaped'] is True
    calls = data('candidate/calls.json'); observed_calls = []; transactions = []; projects = []; script_reads = []
    for key in ('public-source','public-reopen'):
        lines = gzip.decompress(read('candidate/'+key+'.rpc.jsonl.gz')).splitlines()
        assert len(lines) == data('candidate/'+key+'.json')['calls'] * 2
        for request_line,response_line in zip(lines[::2],lines[1::2],strict=True):
            assert request_line.startswith(b'{"request":') and response_line.startswith(b'{"response":')
            raw_request = request_line[len(b'{"request":'):-1]+b'\n'
            raw_response = response_line[len(b'{"response":'):-1]+b'\n'
            req,resp = json.loads(raw_request),json.loads(raw_response)
            assert req['id'] == resp['id'] and 'error' not in resp
            result = resp['result']
            observed_calls.append(dict(method=req['method'],request_sha256=H(raw_request),response_sha256=H(raw_response),revision=result.get('revision') if isinstance(result,dict) else None))
            if req['method'] == 'command.execute':
                operations = req['params']['commands']; assert len(operations) == 1
                op = operations[0]; assert op['op'] == 'write_script'
                assert H(op['source'].encode()) in prepared['script_sha256'].values()
                transactions.append(result['transaction_id'])
            if req['method'] == 'project.read':
                projects.append(result['project'])
            if req['method'] == 'script.read':
                script_reads.append(H(result['untrusted_source_data']['source'].encode()))
    assert observed_calls == calls and transactions == prepared['transaction']
    assert all(project == projects[0] for project in projects)
    assert sorted(script_reads[:2]) == sorted(prepared['old_script_sha256'].values())
    assert sorted(script_reads[2:]) == sorted(prepared['script_sha256'].values())
    assert sum(c['method']=='history.undo' for c in calls) == sum(c['method']=='history.redo' for c in calls) == 2
    ci = data('candidate/npm-test-tools.json')
    assert ci['head'] == manifest['sources']['ci'] and ci['exit'] == 0 and ci['reaped'] is True
    assert ci['source'] == manifest['owned']
    for stream,s in ci['logs'].items():
        assert s == rows['candidate/npm-test-tools.'+stream]['sha256']
    current = data('current/checks.json'); assert len(current) == 3
    for command in current:
        assert command['exit'] == 0 and command['sha256'] == rows['current/'+command['log']]['sha256']
    # Current main composition matches all measured build inputs, except exact CI script wiring.
    for n,s in ENGINE(b['repository_files']).items():
        if n != 'package.json':
            assert file_digest(ROOT/n) == s,n
    previous_end = pre_ns; measured = {}; heaps = []
    inputs = data('reference/shipping-inputs.json')
    for name in plan['order']:
        label,repeat = name[:-1],int(name[-1]); prefix = 'pair/'+name+'/'
        m,r,c = (data(prefix+n) for n in ('measurement.json','stdout.json','command.json'))
        assert c['exit'] == 0 and previous_end <= c['started_ns'] <= c['ended_ns']
        previous_end = c['ended_ns']
        assert '--profile' in c['argv'] and c['argv'][c['argv'].index('--ticks')+1] == '2156'
        assert set(c['environment']) == {'PATH','TMPDIR'}
        for stream in ('stdout.json','stderr.log'):
            assert c[stream+'_sha256'] == rows[prefix+stream]['sha256']
        assert not read(prefix+'stderr.log')
        assert m['profile_report_sha256'] == rows[prefix+'stdout.json']['sha256']
        assert m['identities'] == data(label+'/functional-win/command.json')['identity']
        assert m['metrics'] == metrics(r) and m['comparison'] is None and m['phase_gate_passed'] is False
        assert {k:v for k,v in r.items() if k!='profile'} == functionals[label]
        assert r['pcm_sha256'] == rows[prefix+'output.pcm']['sha256'] == functionals[label]['pcm_sha256']
        assert rows[prefix+'output.pcm']['bytes'] == r['frames']*8
        assert len(r['profile']['scripts']) == 2
        for script in r['profile']['scripts']:
            assert script['totals']['calls'] == 2156 and script['totals']['unwound'] == script['totals']['unavailable_cpu_samples'] == 0
        match,export = m['match'],exports[label]
        assert match['codegen'] == export['codegen'] and match['profiling'] == export['profiling']['compiler']['features']
        assert match['compiler'] == export['compiler'] and match['physics'] == 'fast'
        assert match['ticks'] == 2156 and match['tick_rate'] == 60 and match['project_sha256'] == freezes[label]['project']
        assert match['script_source_sha256'] == data(label+'/package/project-build.json')['compiler']['bundleSha256']
        assert match['data'] == export['data']['files']
        for key in ('input','observe'):
            assert match['inputs'][key] == inputs[key+'_sha256'] == rows[prefix+key+'.json']['sha256']
        measured[name] = m; heaps.append(dict(run=name,quickjs=r['profile']['quickjs_heap']))
    result = data('pair/results.json'); assert result['plan'] == plan and result['phase_gate_passed'] is False
    pairs = []
    for i in range(3):
        old,new = measured['baseline'+str(i)],measured['candidate'+str(i)]
        assert old['match']['script_source_sha256'] != new['match']['script_source_sha256']
        assert {k:v for k,v in old['match'].items() if k!='script_source_sha256'} == {k:v for k,v in new['match'].items() if k!='script_source_sha256'}
        # Difference is authorized only above; generic comparison retains strict identity.
        normalized = dict(new,match=old['match']); comparison = compare(normalized,old)
        assert comparison['script_2ms_p95'] == 'fail' and all(v=='pass' for v in comparison['metrics'].values())
        pair = dict(repeat=i,script_budget=comparison['script_2ms_p95'],metrics={n:dict(baseline=old['metrics'][n],candidate=new['metrics'][n],within_5_percent=s) for n,s in comparison['metrics'].items()})
        pairs.append(pair)
    assert result['pairs'] == pairs
    assert result['records'] == {n:rows['pair/'+n+'/measurement.json']['sha256'] for n in plan['order']}
    assert data('pair/completed.json')['completed'] == plan['order']
    for n in ('failed001','failed002'):
        attempt = data(n+'/attempt.json'); assert attempt['success'] is False and attempt['owned_rpc_closed'] is True
    count = controls(data('pair/baseline0/stdout.json'),measured['baseline0'],dict(measured['candidate0'],match=measured['baseline0']['match']))
    print(json.dumps(dict(passed=True, records=len(rows),payloads=len(payloads),source_counts=source_counts,builds=builds,mutation_controls=count,pairs=pairs,quickjs_heaps=heaps,phase_gate_passed=False),indent=2))


if __name__ == '__main__':
    main()
