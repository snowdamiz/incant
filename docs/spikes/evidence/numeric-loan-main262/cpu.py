"""Audit retained native CPU commands and immediate binary identities."""
import re


def audit(read, data, rows, git_source):
    results = {}
    for generation in ('cpu001', 'cpu002'):
        source = 'source-001' if generation == 'cpu001' else 'source-002'
        prefix = generation + '/'
        freeze_path = prefix + source + '/freeze.json'
        freeze = data(freeze_path)
        if git_source:
            git_source(freeze['head'], freeze['source_paths'])
        names = sorted(n for n in rows if n.startswith(prefix) and n.count('/') == 1 and n.endswith('.json') and n != prefix+'release.json')
        commands = sorted(((data(n), n) for n in names), key=lambda pair: pair[0]['started_ns'])
        total_bins = 0
        counts = {}
        previous_end = 0
        for command, path in commands:
            name = path.removeprefix(prefix).removesuffix('.json')
            assert command['exit'] == 0 and command['error'] is None
            assert command['source_freeze_sha256'] == rows[freeze_path]['sha256']
            assert previous_end <= command['started_ns'] < command['ended_ns']
            previous_end = command['ended_ns']
            assert command['environment']['CARGO_BUILD_JOBS'] == command['environment']['RAYON_NUM_THREADS'] == '2'
            for stream, digest in command['logs'].items():
                assert rows[prefix+name+'.'+stream]['sha256'] == digest
            for binary in command['binaries']:
                row = rows[prefix+binary['copy']]
                assert row['sha256'] == binary['sha256'] and row['bytes'] == binary['bytes']
                total_bins += 1
            listed = re.findall(r'Running [^\n]*\(([^)]+/deps/[^)]+)\)', read(prefix+name+'.stderr').decode())
            assert listed == [b['original'] for b in command['binaries']]
            if command['argv'][1] == 'test':
                text = read(prefix+name+'.stdout').decode()
                summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
                totals = [sum(int(row[i]) for row in summaries) for i in range(3)]
                assert totals[0] > 0 and totals[1:] == [0, 0]
                assert len(re.findall(r'^test (\S+) \.\.\. ok$', text, re.M)) == totals[0]
                assert len(summaries) == len(listed)
                counts[name] = dict(cases=totals[0],failed=0,ignored=0,harnesses=len(summaries))
            if 'clippy' in name:
                assert '--all-targets' in command['argv'] and command['argv'][-3:] == ['--','-D','warnings']
        release = data(prefix+'release.json')
        assert release['source_freeze_sha256'] == rows[freeze_path]['sha256'] and release['at_ns'] >= previous_end
        if generation == 'cpu002':
            assert len(freeze['source_paths']) == 3874 and len(commands) == 8 and total_bins == 84
            assert counts == {name:dict(cases=466,failed=0,ignored=0,harnesses=42) for name in ('deterministic-tests','fast-tests')}
        results[generation] = dict(source=freeze['head'],commands=len(commands),binary_copies=total_bins,tests=counts)
    return results
