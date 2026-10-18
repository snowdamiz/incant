"""Saved-data-only MRT discrimination; the original geometric oracle is unchanged."""
from pathlib import Path
import hashlib,json,math,re,struct,sys
import owner_data as prior
original=prior.original
ARMS=['rgba-forward','rgba-sample-x']
POSES=[original.INITIAL]*8+[(.125,0.,20.,1.,False),(.125,-.125,20.,1.,False),(0.,0.,0.,1.,False)]
RECEIPT='exact queue.write_buffer source slice after call; not GPU memory readback'

def slots(arm):
 assert arm in ARMS
 return [0,1,2,3]

def check_routing(record,arm):
 assert record['storage_arm']==arm and record['semantic_readback_slots']==slots(arm)
 assert record['receipt_kind']=='actual wgpu views used by this probe; not native driver state introspection'
 attachments=record['physical_color_attachments'];assert len(attachments)==4
 assert [a['slot']for a in attachments]==[0,1,2,3]
 assert all(a['format']=='Rgba16Uint'and isinstance(a['view'],str)and a['view']for a in attachments)
 assert isinstance(record['depth_view'],str)and record['depth_view']

def check_msl_locations(record,arm,calibration=False):
 text=record['MSL']['actual_HAL_MSL'][1]['text']
 members=re.findall(r'metal::uint4 (\w+) \[\[color\((\d+)\)\]\];',text)
 names=['motion_xy','motion_zw','sample_id'if calibration else'owner','xy'if calibration else'sample_position']
 assert len(members)==4 and dict(members)==dict(zip(names,map(str,slots(arm)))),'actual MSL color routing'

def check_scene(record,baseline,arm):
 expected=baseline['WGSL']
 if arm!=ARMS[0]:
  before='vec4u(in.owner&65535u,in.owner>>16u,0u,0u)'
  after='vec4u(in.owner&65535u,in.owner>>16u,pack(in.sample_position).xy)'
  assert expected.count(before)==1
  expected=expected.replace(before,after)
 assert record['WGSL']==expected,'only spare companion lanes may change'
 for key in ['targets','depth','multisample','primitive','vertex_buffers','layout_groups','cache']:
  assert record[key]==baseline[key],key
 assert record['targets'].count('format: Rgba16Uint')==4
 assert record['targets'].count('blend: None')==4 and record['targets'].count('write_mask: ColorWrites(15)')==4
 check_msl_locations(record,arm)

def field_differences(control,swapped):
 assert len(control)==len(swapped)==original.N
 groups={'motion_xy':(0,2),'motion_zw':(2,4),'position_depth_lane':(4,8),'decoded_owner':(8,9),'raw_motion_zw':(12,16),'raw_owner':(16,20),'raw_owner_xy':(16,18),'companion_zw':(18,20)}
 out={}
 for name,(start,end)in groups.items():
  out[name]=sum((tuple(bits(v)for v in a[start:end])!=tuple(bits(v)for v in b[start:end]))if end<=8 else a[start:end]!=b[start:end]for a,b in zip(control,swapped))
 return out

def storage_raw(path):
 assert path.is_file()and not path.is_symlink()and path.stat().st_size==original.N*80
 rows=list(struct.iter_unpack('<8f12I',path.read_bytes()))
 assert all(all(math.isfinite(x)for x in r[:8])for r in rows)
 return rows

def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def packing(rows,arm):
 bad=[]
 for i,r in enumerate(rows):
  v,o=r[12:16],r[16:20]
  depth=v[0]|((v[1]&32767)<<16)if arm=='redundant-forward'else v[0]|(v[1]<<16)
  valid=(0x3f800000 if v[1]&32768 else 0)if arm=='redundant-forward'else v[2]|(v[3]<<16)
  good=all(x<=65535 for x in v+o)and r[8:12]==(o[0]|(o[1]<<16),o[0],o[1],v[2])
  spare=(o==(0,0,0,0)if r[6]==1. else o[2]|(o[3]<<16)==bits(r[4]))if arm=='rgba-sample-x'else o[2:]==(0,0)
  good=good and [bits(r[2]),bits(r[3])]==[depth,valid]and spare
  if not good:bad.append(i)
 return bad

def duplicate(rows):return [r[:8]+(r[14]|(r[15]<<16),0,0,0)for r in rows]

def check_upload(record,draw,index,previous,words):
 assert record['frame']==index and len(record['uploads'])==2
 for receipt,item in zip(record['uploads'],draw['draw_order']):
  assert receipt['ordinal']==item['ordinal'] and receipt['buffer']==item['previous_instances_handle']
  assert receipt['instances']==item['instances']==1 and receipt['upload_sequence']==index+1
  assert receipt['receipt_kind']==RECEIPT
  b=receipt['CPU_upload_bytes'];assert len(b)==80 and all(type(x)is int and 0<=x<256 for x in b)
  node=item['nodes'];assert node in [[0],[2]];owner=0 if node==[0]else 1
  matrix=list(struct.unpack('<16f',bytes(b[:64])))
  expected=[1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]
  if index:
   expected[13]=previous[0]if owner==0 else previous[2]
   expected[14]=0. if owner==0 else previous[3]
  assert matrix==expected,('previous matrix',index,node,matrix,expected)
  assert bytes(b[64:])==struct.pack('<f3I',float(index!=0),words[owner],0,0),('validity/owner/reserved',index,node)
 return [hashlib.sha256(bytes(r['CPU_upload_bytes'])).hexdigest()for r in record['uploads']]

def validate(root,old,previous_capture):
 assert {p.name for p in root.iterdir()}=={'fixture.json','capture.json','calibration-raw-visibility.bin',*ARMS}
 assert not any(p.is_symlink()for p in root.rglob('*'))
 calibration=prior.raw(root/'calibration-raw-visibility.bin');assert not prior.packing(calibration,False)
 pattern=original.calibrate(prior.normalize(calibration));fixture=original.read_json(root/'fixture.json')
 assert fixture['measured_sample_offsets']==[list(p)for p in pattern]
 assert fixture['geometry_tie_distance_pixels']==1e-5 and fixture['position_epsilon_pixels']==2e-5 and fixture['depth_motion_epsilon']==2e-6
 assert fixture['storage_probe']is True and fixture['slot_probe']is False and fixture['dependency_probe']is True and fixture['capture_completion_is_acceptance']is False and fixture['no_new_color_resolver']is True
 assert fixture['edge']==original.read_json(previous_capture/'fixture.json')['edge']
 assert fixture['strip']==original.read_json(previous_capture/'fixture.json')['strip']
 assert fixture['occluder']==original.read_json(previous_capture/'fixture.json')['occluder']
 final=original.read_json(root/'capture.json')
 assert final['capture_complete']is True and final['accepted_frames']==22 and final['visibility_passes']==23
 assert final['baseline_reproduction_required']is True and final['original_oracle_unchanged']is True and final['appearance_or_performance_accepted']is False
 summaries=[];raws={};pipelines={};uploads={};uniforms={};frames={};differences={}
 common={'attachment-routing.json','static-area.json','frame-10-low.f32le','pipeline-readback.json','pipeline-calibration-2.json'}|{f'pipeline-scene-{i}.json'for i in range(4)}|{f'{prefix}-{i}.json'for prefix in ['draw','upload','frame']for i in range(11)}
 for arm in ARMS:
  path=root/arm;baseline=arm=='rgba-forward';rawname='frame-10-storage.bin'
  assert {p.name for p in path.iterdir()}==common|{rawname}
  pipes=[]
  for i in range(4):
   p=original.read_json(path/f'pipeline-scene-{i}.json');msl=prior.pipeline(p,[('vertex','Vertex'),('fragment','Fragment')]);pipes.append(msl)
   assert p['layout_groups']==['frame uniform vertex+fragment','existing material']and p['cache']is False
   assert '@builtin(sample_index) actual_sample:u32'not in p['WGSL']
   oldp=original.read_json(previous_capture/ARMS[0]/f'pipeline-scene-{i}.json')
   check_scene(p,oldp,arm)
   if baseline:assert msl==prior.pipeline(oldp,[('vertex','Vertex'),('fragment','Fragment')]),'unchanged actual baseline MSL'
  calibration_pipeline=original.read_json(path/'pipeline-calibration-2.json')
  prior.pipeline(calibration_pipeline,[('vertex','Vertex'),('fragment','Fragment')])
  check_msl_locations(calibration_pipeline,arm,True)
  oldcal=original.read_json(previous_capture/ARMS[0]/'pipeline-calibration-2.json')
  expectedcal=oldcal['WGSL']
  assert arm in ARMS  # Calibration source is unchanged in both arms.
  assert calibration_pipeline['WGSL']==expectedcal
  check_routing(original.read_json(path/'attachment-routing.json'),arm)
  rp=original.read_json(path/'pipeline-readback.json');readmsl=prior.pipeline(rp,[('copy','Compute')])
  oldrp=original.read_json(previous_capture/ARMS[0]/'pipeline-readback.json')
  assert rp['WGSL']==oldrp['WGSL']and readmsl==prior.pipeline(oldrp,[('copy','Compute')]),'identical semantic reader shader'
  pipelines[arm]=dict(scene=pipes,readback=readmsl)
  previous=original.INITIAL;ids=None;upload_hashes=[];uniform_rows=[];frame_rows=[]
  for i,current in enumerate(POSES):
   s=original.read_json(path/f'frame-{i}.json');frame_rows.append(s)
   assert [s[k]for k in ['frame_index','sequence','epoch','phase','has_previous','explicit','reverse','storage_arm']]==[i,i+1,1,i%8,i!=0,False,False,arm]
   assert s['current_pose']==list(current[:4])and s['previous_pose']==list(previous[:4])
   if ids is None:
    ids=[word&~original.CHANGED for word in s['owner_words']];assert len(set(ids))==2 and min(ids)>0
   changed=[i==0 or current[0]!=previous[0],i==0 or current[2:4]!=previous[2:4]]
   assert s['owner_words']==[v|(original.CHANGED if c else 0)for v,c in zip(ids,changed)]
   d=original.read_json(path/f'draw-{i}.json')
   assert [d[k]for k in ['accepted_frame_index','reverse','explicit_sample_index','storage_arm','recorded_before_encode']]==[i,False,False,arm,True]
   assert d['depth_clear']==1 and d['depth_compare']=='Less'and d['depth_write']is True and d['resolve']is False and d['viewport']==[0,0,64,64]
   assert d['color_load']=='clear zero'and d['color_store']=='store'
   order=d['draw_order'];assert len(order)==2 and [r['ordinal']for r in order]==[0,1]and [r['original_draw_index']for r in order]==[0,1]
   assert [r['nodes']for r in order]==[[2],[0]]
   assert len(d['uniform_actual_bytes'])==144 and all(type(v)is int and 0<=v<256 for v in d['uniform_actual_bytes'])
   uniform_rows.append(d['uniform_actual_bytes'])
   upload_hashes.append(check_upload(original.read_json(path/f'upload-{i}.json'),d,i,previous,s['owner_words']))
   if i<10:
    assert s['oracle']['mismatches']==[]and s['packing_mismatches']==[]and s['visibility_accepted']is True
    assert s['duplicate_owner_oracle']is None if arm!='redundant-forward'else s['duplicate_owner_oracle']['mismatches']==[]
   previous=current
  rows=storage_raw(path/rawname);raws[arm]=rows
  report=prior.inspection(rows,pattern,POSES[-1],POSES[-2],s['owner_words'],True)
  packed=packing(rows,arm)
  dup=prior.inspection(duplicate(rows),pattern,POSES[-1],POSES[-2],s['owner_words'],True)if arm=='redundant-forward'else None
  assert s['oracle']==report and s['packing_mismatches']==packed and s['duplicate_owner_oracle']==dup
  accepted=not report['mismatches']and not packed and(dup is None or not dup['mismatches']);assert s['visibility_accepted']==accepted
  summaries.append(dict(arm=arm,visibility_accepted=accepted,oracle_mismatches=len(report['mismatches']),packing_mismatches=len(packed),duplicate_owner_oracle=dup))
  if baseline:
   assert [r[:3]for r in report['mismatches']]==[[x,44,3]for x in range(6,54)]
   assert (path/rawname).read_bytes()==(previous_capture/arm/rawname).read_bytes(),'baseline raw bits changed'
  low=(path/'frame-10-low.f32le').read_bytes();assert len(low)==262144 and low==(old/'occluder-low.f32le').read_bytes()
  assert all(math.isfinite(v[0])for v in struct.iter_unpack('<f',low))
  assert original.read_json(path/'static-area.json')==original.read_json(old/'static-area.json')
  uploads[arm]=upload_hashes;uniforms[arm]=uniform_rows;frames[arm]=frame_rows
 assert final['arms']==summaries and final['visibility_accepted']==all(s['visibility_accepted']for s in summaries)
 for arm in ARMS[1:]:
  assert uploads[arm]==uploads[ARMS[0]]and uniforms[arm]==uniforms[ARMS[0]],'same actual CPU upload sources and frame uniform'
  assert [s['owner_words']for s in frames[arm]]==[s['owner_words']for s in frames[ARMS[0]]]
  differences[arm]=field_differences(raws[ARMS[0]],raws[arm])
 records=[dict(path=str(p),bytes=p.stat().st_size,sha256=original.sha(p))for p in sorted(root.rglob('*'))if p.is_file()]
 return dict(capture_data_consistent=True,visibility_accepted=final['visibility_accepted'],arms=summaries,actual_MSL_sha256=pipelines,
  baseline_exact_failure_reproduced=True,field_differences_from_baseline=differences,semantic_readback_slots={arm:slots(arm)for arm in ARMS},CPU_upload_source_hashes=uploads,
  CPU_upload_source_is_GPU_memory_readback=False,records=records,original_oracle_unchanged=True,original_lowres_planes_unchanged=True,
  original_148_halo_49_strip_and_missed_sliver_retained=True,observer_perturbation_possible=True,driver_cause_proven=False,appearance_or_performance_accepted=False)
if __name__=='__main__':print(json.dumps(validate(*(Path(x)for x in sys.argv[1:]))))
