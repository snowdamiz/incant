"""Fixed geometry saved-data replay, no shader/filter math or native execution."""
from pathlib import Path
import hashlib,json,math,struct,sys
N=64*64*4;CHANGED=1<<31;EPS=2e-5

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read_json(p):
 assert p.is_file()and not p.is_symlink()and p.stat().st_size<1024*1024,p
 return json.loads(p.read_text())
def samples(p):
 assert p.is_file()and not p.is_symlink()and p.stat().st_size==N*48,p
 rows=list(struct.iter_unpack('<8f4I',p.read_bytes()))
 assert all(all(math.isfinite(x)for x in r[:8])and r[9:]==(0,0,0)for r in rows)
 return rows
def calibrate(rows):
 assert len(rows)==N
 pattern=[r[4:6]for r in rows[:4]]
 for i,r in enumerate(rows):
  pixel,lane=divmod(i,4);x,y=pixel%64,pixel//64
  assert r[:4]==(0.,)*4 and r[6:9]==(.5,float(lane),lane+1)
  local=(r[4]-x,r[5]-y)
  assert all(0<=v<1 for v in local)
  assert all(abs(a-b)<=EPS for a,b in zip(local,pattern[lane]))
 assert all(math.dist(a,b)>.01 for i,a in enumerate(pattern)for b in pattern[:i])
 return pattern
def inside(x,y,shape):
 # Direct authored half-planes, independent of the native polygon cross-product oracle.
 if shape=='edge':
  slope=11.25/29.5;ds=[x-6.25,y-5.75,35.25-y,(25.375+(y-5.75)*slope-x)/math.hypot(1,slope)]
 else:
  x0,y0,x1,y1=(6.25,44.75,54.25,45.)if shape=='strip'else(4.,42.,57.,48.)
  ds=[x-x0,y-y0,x1-x,y1-y]
 d=min(ds);return None if abs(d)<=1e-5 else d>0
# meshY,cameraY,occluderY,occluderZ,removed
INITIAL=(0.,0.,20.,1.,False)
SCENES={**{f'phase-{i}':(INITIAL,INITIAL,i+1,1,i,i!=0)for i in [0,1,7]},
 'moving-strip':((.125,0.,20.,1.,False),INITIAL,9,1,0,True),
 'moving-camera':((.125,-.125,20.,1.,False),(.125,0.,20.,1.,False),10,1,1,True),
 'occluder':((0.,0.,0.,1.,False),(.125,-.125,20.,1.,False),11,1,2,True),
 'occluder-depth':((0.,0.,0.,2.,False),(0.,0.,0.,1.,False),12,1,3,True),
 'cut':((0.,0.,0.,2.,False),(0.,0.,0.,2.,False),13,2,0,False),
 'removal':((0.,0.,20.,1.,True),INITIAL,1,1,0,False)}
def shift(p,owner):return -8*(p[2 if owner==1 else 0]-p[1])
def expected(point,p):
 x,y=point;o=inside(x,y-shift(p,1),'occluder')
 if o is None:return None
 if o:return 1
 if p[4]:return 2
 y-=shift(p,0);a=inside(x,y,'edge');b=inside(x,y,'strip')
 if a is True or b is True:return 0
 if a is False and b is False:return 2
 return None
def inspect(rows,pattern,current,previous,words,valid_previous):
 covered=[0,0];ties=[]
 for i,r in enumerate(rows):
  pixel,lane=divmod(i,4);x,y=pixel%64,pixel//64;point=(x+pattern[lane][0],y+pattern[lane][1]);owner=expected(point,current)
  assert r[7]==lane
  if owner is None:ties.append([x,y,lane]);continue
  if owner==2:
   assert r[:7]==(0.,0.,0.,0.,0.,0.,1.)and r[8]==0,(x,y,lane,'false coverage')
   continue
  covered[owner]+=1;z=current[3]if owner==1 else 0.;oldz=previous[3]if owner==1 else 0.
  old=(point[0],point[1]+shift(previous,owner)-shift(current,owner))
  valid=valid_previous and all(0<=v<64 for v in old)
  motion=(0.,(old[1]-point[1])/64.,(10.-oldz-.1)/49.9,1.)if valid else(0.,)*4
  assert r[8]==words[owner],(x,y,lane,'owner')
  assert abs(r[6]-(10.-z-.1)/49.9)<2e-6,(x,y,lane,'current depth')
  assert all(abs(a-b)<EPS for a,b in zip(r[4:6],point)),(x,y,lane,'sample position')
  assert all(abs(a-b)<2e-6 for a,b in zip(r[:4],motion)),(x,y,lane,'previous correspondence')
 return {'covered_samples':covered,'tie_samples':ties,'mismatches':[]}
def validate(root,baseline,halo_file):
 names=set(SCENES)|{'rejected','rejected-reference','cut-reference'}
 expected_files={'calibration-visibility.bin','fixture.json','static-area.json','result.json','rejected.txt'}
 expected_files|={n+'-visibility.bin'for n in names}|{n+'-low.f32le'for n in names}
 expected_files|={f'phase-{i}-low.f32le'for i in range(2,7)}|{n+'.json'for n in SCENES}|{f'phase-{i}.json'for i in range(2,7)}
 assert {p.name for p in root.iterdir()}==expected_files
 assert not any(p.is_symlink()or not p.is_file()for p in root.iterdir())
 pattern=calibrate(samples(root/'calibration-visibility.bin'));fixture=read_json(root/'fixture.json')
 assert fixture['measured_sample_offsets']==[list(p)for p in pattern]and fixture['samples']==4 and fixture['input']==[48,48]and fixture['output']==[64,64]
 assert fixture['visibility_only']is True and fixture['old_failed_resolver_unchanged']is True
 ids=[v&~CHANGED for v in read_json(root/'phase-0.json')['owner_words']];assert ids[0]>0 and ids[1]>0 and ids[0]!=ids[1]
 for name,(current,previous,seq,epoch,phase,valid)in SCENES.items():
  stats=read_json(root/(name+'.json'));assert [stats[k]for k in ['sequence','epoch','phase','has_previous']]==[seq,epoch,phase,valid]
  changed=[not valid or current[0]!=previous[0],not valid or current[2:4]!=previous[2:4]]
  words=[v|(CHANGED if c else 0)for v,c in zip(ids,changed)]
  if name=='removal':
   words=stats['owner_words'];assert words[0]==0 and words[1]&CHANGED and words[1]&~CHANGED>0
  else:assert stats['owner_words']==words
  actual=inspect(samples(root/(name+'-visibility.bin')),pattern,current,previous,words,valid)
  assert stats['oracle']==actual,name
 for phase in range(8):
  p=root/f'phase-{phase}-low.f32le';raw=p.read_bytes();assert len(raw)==262144
  assert raw[:49152*4]==(baseline/f'frame-{phase+1:02}.f32le').read_bytes()[:49152*4],('original lowres planes',phase)
  if phase in range(2,7):
   stats=read_json(root/f'phase-{phase}.json');assert stats['sequence']==phase+1 and stats['epoch']==1 and stats['phase']==phase and stats['has_previous']is True and stats['oracle']['mismatches']==[]
 for a,b in [('phase-0','rejected'),('phase-1','rejected-reference'),('cut','cut-reference')]:
  for suffix in ['-low.f32le','-visibility.bin']:assert(root/(a+suffix)).read_bytes()==(root/(b+suffix)).read_bytes()
 assert(root/'phase-1-visibility.bin').read_bytes()==(root/'phase-7-visibility.bin').read_bytes()
 assert(root/'rejected.txt').read_text()=='unsupported display output format: R32Float'
 for p in root.glob('*-low.f32le'):
  assert p.stat().st_size==262144 and all(math.isfinite(v[0])for v in struct.iter_unpack('<f',p.read_bytes()))
 raw=samples(root/'phase-0-visibility.bin');halos=read_json(halo_file)['rows'];assert len(halos)==148
 for h in halos:
  x,y=h['x'],h['y'];assert all(r[8]==0 for r in raw[(y*64+x)*4:(y*64+x+1)*4]),h
 rows=[];missing=[];error=mass=0.
 for x in range(6,55):
  area=(min(x+1,54.25)-max(x,6.25))*.25;n=sum(r[8]&~CHANGED==ids[0]for r in raw[(44*64+x)*4:(44*64+x+1)*4]);coverage=n/4.
  rows.append(dict(x=x,y=44,analytic_area=area,samples=n));mass+=coverage;error+=abs(coverage-area)
  if n==0:missing.append([x,44])
 metrics=read_json(root/'static-area.json')
 assert metrics==dict(halo_rows_checked=148,invented_halo_rows=[],strip_49_pixels=rows,strip_missed_analytic_slivers=missing,strip_absolute_area_error=error,strip_sampled_area_mass=mass,analytic_area=12.,visibility_is_not_radiance=True)
 result=read_json(root/'result.json');assert result==dict(passed=True,calibration_draws=1,accepted_scene_draws=17,rejected_submissions=2,original_static_lowres_phases=8,halo_rows=148,strip_area_pixels=49,missed_slivers_preserved=True,moving_radiance_reconstruction=False,appearance_or_performance_accepted=False)
 records=[dict(path=str(p),bytes=p.stat().st_size,sha256=sha(p))for p in sorted(root.iterdir())]
 return dict(visibility_checks_pass=True,measured_sample_offsets=pattern,analytic_strip_metrics=metrics,unchanged_original_lowres_planes=True,actual_scene_draws=17,calibration_draws=1,records=records,radiance_appearance_or_performance_accepted=False)
if __name__=='__main__':print(json.dumps(validate(*(Path(v)for v in sys.argv[1:]))))
