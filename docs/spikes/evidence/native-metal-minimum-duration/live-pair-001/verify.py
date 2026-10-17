#!/usr/bin/env python3
"""Replay saved shipping/CPU/native evidence only; no archived program executes."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tempfile

HERE=Path(__file__).resolve().parent
REPO=HERE.parents[4]
SOURCE='380a03a4da2e3882dce8e199e85fcce3d13e4006'


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def file_sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'Nonfinite JSON'))


def read_bounded(path, cap):
    with Path(path).open('rb') as stream:
        raw = stream.read(cap + 1)
    require(len(raw) <= cap, 'Input exceeds byte cap: ' + str(path))
    return raw


def load():
    manifest=decode(read_bounded(HERE/'manifest.json',128*1024))
    packed=read_bounded(HERE/'records.json.gz',8*1024**2)
    require(len(packed)==manifest['archive_bytes'] and sha(packed)==manifest['archive_sha256'],'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as stream:raw=stream.read(32*1024**2+1)
    require(len(raw)==manifest['expanded_bytes']<=32*1024**2,'Expanded archive bound')
    packet=decode(raw);require(packet['format']=='incant-minimum-duration-live-pair-records-v1','Record format')
    records={}
    for row in packet['records']:
        require(row['path'] not in records,'Duplicate record')
        content=row['text'].encode()
        require(len(content)==row['bytes'] and sha(content)==row['sha256'],'Record identity')
        records[row['path']]=content
    require(len(records)==manifest['record_count']==87,'Record inventory')
    for name,digest in manifest['analyzer_dependencies'].items():
        require(file_sha(REPO/name)==digest,'Strict analyzer dependency changed: '+name)
    return manifest,records


def validate(manifest,records):
    base=manifest['original_root']+'/'
    get=lambda name:decode(records[base+name])
    require(manifest['source']==SOURCE and manifest['CPU_qualified_native_source']=='c0ba6ba54ae31aae050cd685f27de7655bf76d16','Producer source')
    build=get('shipping-001/metadata/result.json');report=get('shipping-001/package/diagnostic-build.json')
    calls=get('shipping-001/metadata/managed-calls.json');cpu=get('functional-001/result.json')
    require(build['source']==report['source']==cpu['source']==SOURCE and build['status']==cpu['status']=='complete'
            and build['exit']==cpu['exit']==0 and build['supervisor_terminal'] and cpu['supervisor_terminal']
            and not build['scratch_retained'] and not cpu['scratch_retained'],'Completed build/CPU/barrier')
    require(len(calls)==56 and all(x['exit_code']==0 and x['native_process_returned'] and x['cleanup_confirmed']
            and not x['cleanup_failures'] for x in calls),'Build process cleanup')
    require(report['build_and_input_storage_deleted_before_startup'] and report['no_window_or_GPU_executed']
            and report['startup_source_parsing'] is False and report['diagnostic_only']
            and not report['eligible_for_distribution'] and not report['eligible_for_performance_gate'],'Shipping scope')
    require(report['startup']['tick']==1 and report['startup']['frames']==800
            and report['startup']['render_input']['gpu_submitted'] is False,'CPU1tick/PCMframe distinction')
    require(report['player_sha256']==cpu['player_sha256']==manifest['player_sha256'],'Same player')
    require(report['compatibility']['changed_generated_fields']==['build_id']
            and not report['compatibility']['historical_payload_equality_claim'],'Actual current producer identity')
    require(cpu['exact_entire_logical_report'] and cpu['exact_full_PCM'] and cpu['child_exit']==0
            and cpu['artifacts']['output.pcm']['bytes']==manifest['full_PCM_bytes']==13798400
            and cpu['artifacts']['output.pcm']['sha256']==manifest['full_PCM_sha256'],'Whole CPU/PCM equality')
    actual=get('functional-001/artifacts/stdout.json')
    old=[decode(raw) for name,raw in records.items() if name.endswith('/mac-core-numeric-loan-shipping-001/functional-win/stdout.json')]
    require(len(old)==1 and actual==old[0] and actual['tick']==2156 and len(actual['observations']['checks'])==8
            and len(actual['observations']['entities'])==17,'Exact entire logical report')
    for arm,status in [('ordinary','complete'),('minimum-duration60','failed')]:
        prefix='replay-preparation-001/'+arm+'/'
        plan=get(prefix+'plan.json');grant=get(prefix+'root-grant.json')
        result=get('window-'+arm+'-001/result.json');child=get('window-'+arm+'-001/child-result.json')
        require(grant['plan_sha256']==result['plan_sha256']==sha(records[base+prefix+'plan.json'])
                and grant['runner_sha256']==result['runner_sha256']==sha(records[base+prefix+'run.py']),'Native execution binding')
        require(result['status']==manifest['strict_arm_results'][arm]==status and child['exit_code']==0
                and result['child_cleanup_confirmed'] and child['cleanup_confirmed'] and not child['cleanup_errors'],
                'Original native acceptance/cleanup')
        require(plan['player_sha256']==manifest['player_sha256'] and plan['source']==SOURCE
                and plan['data_files']==get('replay-preparation-001/ordinary/plan.json')['data_files'],
                'Same source/player/data pair')
    require(get('window-minimum-duration60-001/result.json')['analysis']['partial_reasons']
            ==['partial_or_invalid_scheduler_batches'],'Original strict failure retained')
    for name in ['performance_gate_passed','display_gate_passed','policy_adopted','new_default_enabled']:
        require(manifest[name] is False,'Unsupported adoption/gate claim')
    review=decode(records[manifest['independent_review']['path']])
    require(sha(records[manifest['independent_review']['path']])==manifest['independent_review']['sha256']
            and review['adoption_supported'] is False,'Independent review scope')
    for arm in ['ordinary','minimum-duration60']:
        require(review['results'][arm]['result_sha256']==sha(records[base+'window-'+arm+'-001/result.json']),
                'Independent review result binding')
    import analyze
    computed=json.loads(json.dumps(analyze.analyze(records),allow_nan=False))
    require(computed==get('paired-analysis-001/summary.json'),'Independent saved-row arithmetic changed')
    freeze=get('shipping-packet-002/source-manifest.json')
    require(freeze['source']==SOURCE and len(freeze['files'])==4749,'Source inventory')
    return freeze,get('shipping001-artifact-inventory.json')['artifacts'],computed


def verify_git(freeze, revision):
    revision = subprocess.check_output(['git', 'rev-parse', revision], cwd=REPO, text=True).strip()
    require(revision == SOURCE, 'Use exact historical native producer')
    # Stream one blob at a time; neither all source text nor all Git output is loaded together.
    with tempfile.TemporaryFile() as requests:
        requests.write(''.join(revision + ':' + name + '\n' for name in freeze['files']).encode())
        requests.seek(0)
        with subprocess.Popen(['git', 'cat-file', '--batch'], cwd=REPO, stdin=requests,
                              stdout=subprocess.PIPE) as process:
            for name, digest in freeze['files'].items():
                header = process.stdout.readline(1024).split()
                require(len(header) == 3 and header[1] == b'blob', 'Missing Git blob')
                size = int(header[2])
                require(0 <= size <= 32 * 1024**2, 'Source blob size bound')
                raw = process.stdout.read(size)
                require(len(raw) == size and sha(raw) == digest and process.stdout.read(1) == b'\n',
                        'Git source changed: ' + name)
            require(process.stdout.read(1) == b'' and process.wait(timeout=30) == 0, 'Trailing Git output')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals',action='store_true')
    parser.add_argument('--artifacts',action='store_true')
    parser.add_argument('--git-source',metavar='REV')
    parser.add_argument('--source-root',type=Path)
    args=parser.parse_args();manifest,records=load();freeze,artifacts,summary=validate(manifest,records)
    if args.originals:
        for name,raw in records.items():require(read_bounded(name,len(raw))==raw,'Original changed: '+name)
        cargo=decode(records[manifest['original_root']+'/shipping001-cargo-audit.json'])['original']
        require(Path(cargo['path']).stat().st_size==cargo['bytes'] and file_sha(cargo['path'])==cargo['sha256'],'Full retained Cargo stream changed')
    if args.artifacts:
        for row in artifacts:
            require(Path(row['path']).stat().st_size==row['bytes'] and file_sha(row['path'])==row['sha256'],'External artifact changed')
    if args.git_source:verify_git(freeze,args.git_source)
    if args.source_root:
        for name,digest in freeze['files'].items():require(file_sha(args.source_root/name)==digest,'Producing input changed: '+name)
    print('PASS:87 saved records;fresh shipping+exact2156 CPU/PCM;ordinary complete;treatment strict FAILED retained;no adoption')


if __name__=='__main__':main()
