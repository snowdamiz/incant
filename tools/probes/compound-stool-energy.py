#!/usr/bin/env python3
"""Numerical energy trace for Claude's stool fixture; no rendering or project edits.

Reads a validated saved look-dev project and replays prefixes through the public
CLI. Additive primitive masses/inertias are calculated independently in f64;
linear velocity is the world COM velocity, angular velocity is rotated to the
body frame. Only unrotated box/Y-capsule parts are supported by this diagnostic.
This is a fixture-specific investigation, not a general solver correctness gate.
"""
import argparse
import json, math, subprocess, hashlib
from pathlib import Path
root=Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('scene',type=Path,help='existing Claude look-dev output directory')
parser.add_argument('output',type=Path,help='new output directory')
parser.add_argument('--binary',type=Path,default=root/'target/release/incant_headless')
args=parser.parse_args()
scene=args.scene.resolve()
project=scene/'compound.incant.json'
script=scene/'compound_course.js'
binary=args.binary.resolve()
out=args.output.resolve()
out.mkdir(parents=True,exist_ok=False)
p=json.loads(project.read_text())
e=next(e for s in p['scenes'].values() for e in s['entities'].values() if e['name']=='Falling stool')
density=e['components']['Collider']['density']
parts=[]
for part in e['components']['Collider']['shape']['parts']:
 assert part['rotation']==[0,0,0,1]
 s=part['shape']
 if s['type']=='box':
  x,y,z=s['half_extents']; m=8*x*y*z*density
  inertia=[m*(y*y+z*z)/3,m*(x*x+z*z)/3,m*(x*x+y*y)/3]
 else:
  assert s['type']=='capsule'
  r,h=s['radius'],s['half_height']; mc=math.pi*r*r*2*h*density; ms=4*math.pi*r**3/3*density; m=mc+ms
  across=mc*(3*r*r+4*h*h)/12+ms*(.4*r*r+h*h+.75*h*r)
  inertia=[across,.5*mc*r*r+.4*ms*r*r,across]
 parts.append((m,part['translation'],inertia))
mass=sum(t[0] for t in parts)
com=[sum(m*p[i] for m,p,_ in parts)/mass for i in range(3)]
I=[[0.]*3 for _ in range(3)]
for m,p,d in parts:
 r=[p[i]-com[i] for i in range(3)]
 for i in range(3):
  for j in range(3): I[i][j]+=(d[i]+m*sum(v*v for v in r) if i==j else 0)-m*r[i]*r[j]
def cross(a,b): return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def rotate(q,p):
 norm=math.sqrt(sum(v*v for v in q));q=[v/norm for v in q]
 t=[v*2 for v in cross(q[:3],p)];c=cross(q[:3],t)
 return [p[i]+q[3]*t[i]+c[i] for i in range(3)]
def measure(tick,s):
 q=s['rotation'];pos=s['translation'];vel=s['velocity'];av=s['angular_velocity']
 wc=[pos[i]+v for i,v in enumerate(rotate(q,com))]
 w=rotate([-q[0],-q[1],-q[2],q[3]],av)
 kinetic=mass*sum(v*v for v in vel)/2+sum(w[i]*I[i][j]*w[j] for i in range(3) for j in range(3))/2
 return {'tick':tick,'state':s,'center_of_mass':wc,'up_y':rotate(q,[0,1,0])[1],'potential':mass*9.81*wc[1],'kinetic':kinetic,'total':mass*9.81*wc[1]+kinetic}
initial={**e['components']['Transform'],'velocity':[0,0,0],'angular_velocity':[0,0,0]}
trace=[measure(0,initial)]
sourcehash=hashlib.sha256(project.read_bytes()).hexdigest()
# Include all saved authoring files, not generated logs or frame folders.
sourcefiles=[project,script,*scene.glob('*.journal.jsonl')]
sourcehashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sourcefiles}
for tick in range(1,301):
 p=subprocess.run([str(binary),'play',str(project),'--ticks',str(tick),'--compiled-script',str(script)],capture_output=True,text=True,check=True)
 j=json.loads(p.stdout);assert j['completed'];assert j['frames']==[]
 trace.append(measure(tick,j['state']['entities'][e['id']]))
assert sourcehashes=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sourcefiles}
summary={'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'project_sha256':sourcehash,'mass':mass,'local_com':com,'local_inertia':I,'initial_energy':trace[0]['total'],'max_energy':max(t['total'] for t in trace),'final_energy':trace[-1]['total'],'max_positive_step':max(trace[i]['total']-trace[i-1]['total'] for i in range(1,len(trace))),'samples':[{k:v for k,v in trace[i].items() if k!='state'} for i in [0,5,10,13,20,40,57,80,97,150,300]]}
height=com[1]-.01
barrier=math.sqrt(height*height+.11*.11)-height
summary.update(author_files_unchanged=sourcehashes,rendering=False,
               equilibrium_com_height=height,point_foot_barrier_height=barrier,
               point_foot_barrier_energy=mass*9.81*barrier,
               initial_energy_above_upright=trace[0]['total']-mass*9.81*height,
               scope='Approximate point-foot tipping barrier; exact rounded contact dynamics differ. '
                     'No claim of strict per-tick energy monotonicity or universal solver correctness.')
(out/'trace.json').write_text(json.dumps(trace,indent=2)+'\n');(out/'result.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
