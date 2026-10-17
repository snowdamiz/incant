"""Bounded retained-data analysis. No GPU, shell child, fitting or admission."""
from pathlib import Path
import collections, hashlib, json, math, statistics, struct, sys, time

START=time.monotonic()
HERE=Path(__file__).resolve().parent
if not __debug__:
    raise RuntimeError("Run without -O: this retained analyzer uses assertions")

def bounded_file(path, cap):
    assert path.is_file() and not path.is_symlink() and path.stat().st_size<=cap
    with path.open('rb') as stream:
        raw=stream.read(cap+1)
    assert len(raw)<=cap
    return raw

# Read-only transport adaptation. No extraction, subprocess, GPU or native build.
import gzip, io, tarfile
PROVENANCE=json.loads(bounded_file(HERE/'provenance.json',256*1024))
ORIGINAL_ROOT=Path(PROVENANCE['original_root'])
compressed=bounded_file(HERE/'records.tar.gz',1024*1024)
assert hashlib.sha256(compressed).hexdigest()==PROVENANCE['archive_sha256']
with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
    expanded=stream.read(8*1024*1024+1)
assert len(expanded)<=8*1024*1024
ENTRIES={}
expected={row['name']:row for row in PROVENANCE['records']}
assert len(expected)==len(PROVENANCE['records'])==704
with tarfile.open(fileobj=io.BytesIO(expanded),mode='r:') as archive:
    for member in archive:
        assert member.isfile() and member.name in expected and member.name not in ENTRIES
        entry=expected[member.name]
        assert member.size==entry['bytes'] and 0<=member.size<=2*1024*1024
        with archive.extractfile(member) as stream:
            raw=stream.read(member.size+1)
        assert len(raw)==member.size and hashlib.sha256(raw).hexdigest()==entry['sha256']
        ENTRIES[member.name]=raw
assert set(ENTRIES)==set(expected) and sum(map(len,ENTRIES.values()))==4551298

def read_raw(path):
    assert path.parent==ORIGINAL_ROOT
    return ENTRIES[path.name]

def load(path, cap=8*1024*1024):
    raw=read_raw(path)
    assert len(raw)<=cap
    return json.loads(raw)
def finite(value):
    assert isinstance(value,(float,int)) and math.isfinite(value) and value>=0
    return value
def stats(values):
    assert values
    return {'n':len(values),'median':statistics.median(values),'min':min(values),'max':max(values),'all':values}
def analyze(root):
    data=load(root/'result.json'); resources=load(root/'resources.json')
    assert data['completed'] and data['renders']==234 and not data['quality_performance_adoption_accepted']
    assert data['ledger']['maps']==1522 and data['ledger']['submits']==1756
    assert resources['explicit_GPU_bytes']==395032840 and resources['query_slots']==128
    assert resources['marker_texture_logical_bytes']==4 and resources['marker_texture_physical_allocation_unmeasured']
    assert resources['marker_render_passes']==5436
    assert resources['historical_raw_deleted'] and not resources['graph_timing_pool_constructed']
    rows=data['records']; assert len(rows)==234
    expected=[]
    for cycle in range(3):
        for order,routes in enumerate(['OLG','OGL','LOG','LGO','GOL','GLO']):
            expected += [dict(route=r,detailed=True,warmup=cycle==0,overhead=False,cycle=cycle,order=order) for r in routes]
    for route in ['O','G']:
        for cycle in range(3):
            expected += [dict(route=route,detailed=d,warmup=cycle==0,overhead=True,cycle=cycle,order=i) for i,d in enumerate([False,True,True,False])]
    by_pose=collections.defaultdict(list);query_maps=0;data_maps=0;outer_total=0;marker_total=0
    for pose in [32,36,40]:
        current=[r for r in rows if r['pose']==pose]; assert len(current)==78
        for index,(row,schedule) in enumerate(zip(current,expected,strict=True)):
            assert time.monotonic()-START<120
            assert row['trial']==index and row['schedule']==schedule and row['exact_output_passed'] and row['evidence_io_outside_operational_interval']
            prefix=f'pose-{pose}-trial-{index:03}'
            assert row==load(root/f'{prefix}-timing.json')
            parity=load(root/f'{prefix}-parity.json');assert parity['historical_digest_exact'] and parity['historical_raw_unavailable']
            assert parity['fresh_same_pose_baseline_byte_exact']==(index not in [0,1])
            candidate=schedule['route']!='O'; detailed=schedule['detailed']
            count=(38 if detailed else 10) if candidate else (6 if detailed else 2)
            gpu=row['GPU'];assert gpu['marker_render_passes']==count and gpu['timestamp_primitive']=='1x1-clear-render-descriptor-end-fragment'
            assert gpu['instrumented_point_spans_not_hardware_utilization']
            marker_total+=count
            assert gpu['query_count']==count and finite(gpu['period_ns'])>0
            qpath=root/f'{prefix}-query.bin';raw=read_raw(qpath);assert len(raw)==count*8
            ticks=list(struct.unpack('<'+'Q'*count,raw));assert ticks==gpu['raw_ticks']
            query_maps+=1;parts=gpu['intervals'];assert len(parts)==count//2
            outer=[]
            for part in parts:
                a,b=part['start_ticks'],part['end_ticks'];assert 0<=a<=b<2**64-1 and (a,b)!=(0,0)
                period=struct.unpack('<f',struct.pack('<f',finite(gpu['period_ns'])))[0]
                ns=(b-a)*period;assert ns==finite(part['nanoseconds'])
                if part['outer']:outer.append(part)
            assert len(outer)==(5 if candidate else 1)
            assert all(a['end_ticks']<=b['start_ticks'] for a,b in zip(outer,outer[1:]))
            assert sum(x['nanoseconds']for x in outer)==gpu['descriptor_marker_span_sum_ns']
            outer_total+=len(outer)
            operational=finite(row['operational_wall_seconds']);total=finite(row['total_observation_wall_seconds'])
            assert total>=operational>=finite(row['CPU_admission_seconds'])
            assert operational>=finite(row['encoded_queue_dependency_seconds'])>=finite(row['first_submit_to_final_completion_seconds'])
            reads=row['readbacks'];assert len(reads)==(7 if candidate else 3)
            assert sum(x['bytes']for x in reads)==(114048000 if candidate else 39398400)
            assert [x['deferred_operational_capture'] for x in reads]==([True]*4+[False]*3 if candidate else [False]*3)
            for x in reads:
                assert x['mapped_and_unmapped'];assert x['bytes']<=24238336
                assert Path(x['path']).parent==root
                for k in ['wait_seconds','write_hash_seconds','readback_dependency_seconds']:finite(x[k])
            data_maps+=len(reads);by_pose[pose].append(row)
    assert query_maps==234 and data_maps+query_maps+9+1==1522 and outer_total==810 and marker_total==5436
    summaries={};paired={};overhead={}
    for pose,rs in by_pose.items():
        summaries[pose]={}
        for route in ['O','L','G']:
            selected=[r for r in rs if r['schedule']['route']==route and not r['schedule']['warmup'] and not r['schedule']['overhead']]
            assert len(selected)==12
            summaries[pose][route]={k:stats([r['GPU']['descriptor_marker_span_sum_ns'] if k=='GPU_marker_span_sum_ns' else r[k] for r in selected])for k in ['GPU_marker_span_sum_ns','operational_wall_seconds','CPU_admission_seconds','total_observation_wall_seconds']}
        diffs=collections.defaultdict(list)
        for cycle in [1,2]:
            for order in range(6):
                group={r['schedule']['route']:r for r in rs if not r['schedule']['overhead'] and r['schedule']['cycle']==cycle and r['schedule']['order']==order}
                assert set(group)=={'O','L','G'}
                for a,b in [('G','O'),('G','L')]:
                    for k in ['operational_wall_seconds','GPU_marker_span_sum_ns']:
                        value=lambda r:r['GPU']['descriptor_marker_span_sum_ns']if k=='GPU_marker_span_sum_ns'else r[k]
                        diffs[f'{a}_minus_{b}_{k}'].append(value(group[a])-value(group[b]))
        paired[pose]={k:stats(v)for k,v in diffs.items()}
        overhead[pose]={}
        for route in ['O','G']:
            deltas=collections.defaultdict(list)
            for cycle in [1,2]:
                group=[r for r in rs if r['schedule']['overhead'] and r['schedule']['route']==route and r['schedule']['cycle']==cycle]
                assert len(group)==4
                for metric in ['GPU_marker_span_sum_ns','operational_wall_seconds']:
                    costs=[r['GPU']['descriptor_marker_span_sum_ns'] if metric=='GPU_marker_span_sum_ns' else r[metric] for r in group]
                    deltas[metric].append((costs[1]+costs[2]-costs[0]-costs[3])/2)
            overhead[pose][route]={k:stats(v) for k,v in deltas.items()}
    return {'state':'Complete diagnostic observations; no automatic performance or adoption pass','summaries':summaries,'paired_differences':paired,'outer_vs_detailed_overhead':overhead,'all_orders_and_samples_retained':True,'historical_raw_unavailable':True,'current_byte_baselines_checked':True,'no_best_run_selection_or_subtracted_overhead':True,'evidence_IO_excluded_from_operational_wall':True,'GPU_metric':'Observed descriptor-marker span sum; not hardware-wide utilized time or proven serialization of unrelated stages','complete_operational_wall_is_end_to_end_metric':True,'marker_overhead_or_overlap_ambiguity_may_make_sign_inconclusive':True,'shipping_or_quality_adoption':False}
if __name__=='__main__':
    assert len(sys.argv)==1
    result=analyze(ORIGINAL_ROOT)
    published=json.loads(bounded_file(HERE/'analysis.json',128*1024))
    published.pop('seconds')
    assert json.loads(json.dumps(result))==published
    zero=bounded_file(HERE/'E038-zero-query.bin',48)
    assert zero==bytes(48)
    assert hashlib.sha256(zero).hexdigest()=='17b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1'
    print(json.dumps({'verified_records':len(ENTRIES),'renders':234,'maps':1522,'submits':1756,'descriptor_markers':5436,'published_analysis_exact':True,'seconds':time.monotonic()-START,'no_rendered_pixels_or_adoption_verified':True},sort_keys=True))
