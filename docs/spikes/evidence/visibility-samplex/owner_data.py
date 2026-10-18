"""Inert four-arm capture audit. Completion never substitutes for strict acceptance."""
from pathlib import Path
import hashlib,json,math,struct,sys
import verify_visibility as original
ARMS=[('original-forward',False,False),('original-reverse',False,True),('explicit-forward',True,False),('explicit-reverse',True,True)]

def raw(path):
 assert path.is_file()and not path.is_symlink()and path.stat().st_size==original.N*48
 rows=list(struct.iter_unpack('<8f4I',path.read_bytes()))
 assert all(all(math.isfinite(x)for x in r[:8])for r in rows)
 return rows

def normalize(rows):return [r[:9]+(0,0,0)for r in rows]
def packing(rows,explicit):
 bad=[]
 for i,r in enumerate(rows):
  good=r[9]<=65535 and r[10]<=65535 and r[8]==r[9]|r[10]<<16
  good=good and r[11]==(i%4 if explicit and r[8]!=0 else 0)
  if not good:bad.append(i)
 return bad

def inspection(rows,pattern,current,previous,words,valid_previous):
 covered=[0,0];ties=[];errors=[]
 for i,r in enumerate(rows):
  pixel,lane=divmod(i,4);x,y=pixel%64,pixel//64;point=(x+pattern[lane][0],y+pattern[lane][1])
  if r[7]!=lane:errors.append([x,y,lane,'invalid record']);continue
  owner=original.expected(point,current)
  if owner is None:ties.append([x,y,lane]);continue
  if owner==2:
   if r[:7]!=(0.,0.,0.,0.,0.,0.,1.)or r[8]!=0:errors.append([x,y,lane,'invented visible sample'])
   continue
  covered[owner]+=1;z=current[3]if owner==1 else 0.;oldz=previous[3]if owner==1 else 0.
  old=(point[0],point[1]+original.shift(previous,owner)-original.shift(current,owner))
  valid=valid_previous and all(0<=v<64 for v in old)
  motion=(0.,(old[1]-point[1])/64.,(10.-oldz-.1)/49.9,1.)if valid else(0.,)*4
  good=r[8]==words[owner]and abs(r[6]-(10.-z-.1)/49.9)<2e-6
  good=good and all(abs(a-b)<original.EPS for a,b in zip(r[4:6],point))and all(abs(a-b)<2e-6 for a,b in zip(r[:4],motion))
  if not good:errors.append([x,y,lane,'owner/depth/previous correspondence'])
 return dict(covered_samples=covered,tie_samples=ties,mismatches=errors)

def pipeline(record,entries):
 assert record['MSL']['complete']is True and record['MSL']['error']is None
 logs=record['MSL']['actual_HAL_MSL'];assert len(logs)==len(entries)
 for row,(entry,stage)in zip(logs,entries):
  text=row['text'];assert text.startswith(f"Naga generated shader for entry point '{entry}' and stage {stage}\n")
  assert row['bytes']==len(text.encode())and row['sha256']==hashlib.sha256(text.encode()).hexdigest()
 assert record['MSL']['cumulative_MSL_bytes']<=1024*1024
 return [row['sha256']for row in logs]

def validate(root,old):
 assert {p.name for p in root.iterdir()}=={'fixture.json','capture.json','calibration-raw-visibility.bin',*(a[0]for a in ARMS)}
 assert not any(p.is_symlink()for p in root.rglob('*'))
 calibration=raw(root/'calibration-raw-visibility.bin');assert not packing(calibration,False)
 pattern=original.calibrate(normalize(calibration));fixture=original.read_json(root/'fixture.json')
 assert fixture['measured_sample_offsets']==[list(p)for p in pattern]
 assert fixture['geometry_tie_distance_pixels']==1e-5 and fixture['position_epsilon_pixels']==2e-5 and fixture['depth_motion_epsilon']==2e-6
 assert fixture['capture_completion_is_acceptance']is False and fixture['no_new_color_resolver']is True
 final=original.read_json(root/'capture.json');assert final['capture_complete']is True and final['accepted_frames']==44 and final['visibility_passes']==45
 assert final['original_oracle_unchanged']is True and final['appearance_or_performance_accepted']is False
 poses=[original.INITIAL]*8+[(.125,0.,20.,1.,False),(.125,-.125,20.,1.,False),(0.,0.,0.,1.,False)]
 summaries=[];raws={};pipelines={};draws={}
 expected={'static-area.json','frame-10-low.f32le','frame-10-visibility.bin','pipeline-readback.json','pipeline-calibration-2.json'}|{f'pipeline-scene-{i}.json'for i in range(4)}|{f'draw-{i}.json'for i in range(11)}|{f'frame-{i}.json'for i in range(11)}
 for name,explicit,reverse in ARMS:
  path=root/name;assert {p.name for p in path.iterdir()}==expected
  pipes=[]
  for i in range(4):
   p=original.read_json(path/f'pipeline-scene-{i}.json');pipes.append(pipeline(p,[('vertex','Vertex'),('fragment','Fragment')]))
   assert p['layout_groups']==['frame uniform vertex+fragment','existing material']and p['cache']is False
   assert ('@builtin(sample_index) actual_sample:u32'in p['WGSL'])==explicit
  p=original.read_json(path/'pipeline-calibration-2.json');pipeline(p,[('vertex','Vertex'),('fragment','Fragment')]);assert p['layout_groups']==[]
  pipeline(original.read_json(path/'pipeline-readback.json'),[('copy','Compute')]);pipelines[name]=pipes
  previous=original.INITIAL;ids=None
  for i,current in enumerate(poses):
   s=original.read_json(path/f'frame-{i}.json');assert [s[k]for k in ['frame_index','sequence','epoch','phase','has_previous','explicit','reverse']]==[i,i+1,1,i%8,i!=0,explicit,reverse]
   assert s['current_pose']==list(current[:4])and s['previous_pose']==list(previous[:4])
   if ids is None:ids=[word&~original.CHANGED for word in s['owner_words']];assert len(set(ids))==2 and min(ids)>0
   changed=[i==0 or current[0]!=previous[0],i==0 or current[2:4]!=previous[2:4]]
   assert s['owner_words']==[v|(original.CHANGED if c else 0)for v,c in zip(ids,changed)]
   d=original.read_json(path/f'draw-{i}.json');assert d['accepted_frame_index']==i and d['reverse']==reverse and d['explicit_sample_index']==explicit and d['recorded_before_encode']is True
   assert d['depth_clear']==1 and d['depth_compare']=='Less'and d['depth_write']is True and d['resolve']is False and d['viewport']==[0,0,64,64]
   order=d['draw_order'];assert len(order)==2 and [r['ordinal']for r in order]==[0,1]
   assert [r['original_draw_index']for r in order]==([1,0]if reverse else[0,1])
   assert sorted(r['nodes']for r in order)==[[0],[2]]
   assert len(d['uniform_actual_bytes'])==144 and all(type(v)is int and 0<=v<256 for v in d['uniform_actual_bytes'])
   if i<10:assert s['oracle']['mismatches']==[]and s['packing_mismatches']==[]and s['visibility_accepted']is True
   previous=current
  rows=raw(path/'frame-10-visibility.bin');raws[name]=rows
  report=inspection(rows,pattern,poses[-1],poses[-2],s['owner_words'],True);packed=packing(rows,explicit)
  assert s['oracle']==report and s['packing_mismatches']==packed
  accepted=not report['mismatches']and not packed;assert s['visibility_accepted']==accepted
  summary=dict(arm=name,visibility_accepted=accepted,oracle_mismatches=len(report['mismatches']),packing_mismatches=len(packed));summaries.append(summary)
  low=(path/'frame-10-low.f32le').read_bytes();assert len(low)==262144 and all(math.isfinite(v[0])for v in struct.iter_unpack('<f',low))
  assert low==(old/'occluder-low.f32le').read_bytes(),(name,'unchanged original low-res planes')
  metrics=original.read_json(path/'static-area.json');assert metrics==original.read_json(old/'static-area.json'),(name,'original halo/area/sliver oracle')
  draws[name]=[r['nodes']for r in d['draw_order']]
 assert final['arms']==summaries and final['visibility_accepted']==all(s['visibility_accepted']for s in summaries)
 assert draws['original-forward']==draws['explicit-forward']==list(reversed(draws['original-reverse']))==list(reversed(draws['explicit-reverse']))
 old_rows=original.samples(old/'occluder-visibility.bin')
 comparisons={name:sum(a!=b for a,b in zip(normalize(rows),old_rows))for name,rows in raws.items()}
 records=[dict(path=str(p),bytes=p.stat().st_size,sha256=original.sha(p))for p in sorted(root.rglob('*'))if p.is_file()]
 return dict(capture_data_consistent=True,visibility_accepted=final['visibility_accepted'],arms=summaries,actual_MSL_sha256=pipelines,
  normalized_sample_differences_from_retained_failure=comparisons,observed_draw_node_order=draws,records=records,
  original_oracle_unchanged=True,original_lowres_planes_unchanged=True,observer_perturbation_possible=True,driver_cause_proven=False,
  appearance_or_performance_accepted=False)
if __name__=='__main__':print(json.dumps(validate(*(Path(x)for x in sys.argv[1:]))))
