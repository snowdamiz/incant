"""Bounded probe-only skeletal fixture and numeric receipt checks, never execution."""
import base64
import json
import math
import struct

STAGES = ['manual_import_original', 'watched_keys', 'undo_original', 'redo_keys',
          'incompatible_duration_retained_replacement', 'source_recovered_keys']


def gltf(replacement=False, incompatible=False):
    """One skin/clip: only interior keys change; the invalid fixture changes duration."""
    data, views, accessors = bytearray(), [], []
    def add(values, code, component, kind, count):
        while len(data) % 4:
            data.append(0)
        raw = struct.pack('<' + code*len(values), *values)
        index = len(accessors)
        views.append({'buffer':0, 'byteOffset':len(data), 'byteLength':len(raw)})
        accessors.append({'bufferView':index, 'componentType':component, 'type':kind, 'count':count})
        data.extend(raw)
        return index
    position = add([-.5,-.5,0, .5,-.5,0, 0,.5,0], 'f',5126,'VEC3',3)
    accessors[position].update(min=[-.5,-.5,0], max=[.5,.5,0])
    indices = add([0,1,2], 'H',5123,'SCALAR',3)
    joints = add([0]*12, 'H',5123,'VEC4',3)
    weights = add([1,0,0,0]*3, 'f',5126,'VEC4',3)
    inverse = add([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1], 'f',5126,'MAT4',1)
    times = [0,.5,3 if incompatible else 2] if replacement else [0,2]
    values = [0,0,0, .5,0,0, 1,0,0] if replacement else [0,0,0, .5,0,0]
    time = add(times,'f',5126,'SCALAR',len(times))
    accessors[time].update(min=[0],max=[times[-1]])
    output = add(values,'f',5126,'VEC3',len(times))
    document = {
        'asset':{'version':'2.0'}, 'scene':0,'scenes':[{'nodes':[0,1]}],
        'nodes':[{'name':'joint','translation':[0,0,0]},{'mesh':0,'skin':0}],
        'skins':[{'name':'fixed skin','skeleton':0,'joints':[0],'inverseBindMatrices':inverse}],
        'meshes':[{'primitives':[{'attributes':{'POSITION':position,'JOINTS_0':joints,'WEIGHTS_0':weights},
                                'indices':indices,'material':0}]}],
        'materials':[{'pbrMetallicRoughness':{'baseColorFactor':[1,0,0,1],'metallicFactor':0,'roughnessFactor':.8},
                      'emissiveFactor':[1,0,0],'doubleSided':True}],
        'animations':[{'name':'move','samplers':[{'input':time,'output':output,'interpolation':'LINEAR'}],
                       'channels':[{'sampler':0,'target':{'node':0,'path':'translation'}}]}],
        'buffers':[{'byteLength':len(data),'uri':'data:application/octet-stream;base64,'+base64.b64encode(data).decode()}],
        'bufferViews':views,'accessors':accessors,
    }
    return (json.dumps(document,separators=(',',':'))+'\n').encode()


def validate(proof, require):
    require(proof.get('animation_continued') is True and proof.get('committed_incompatible_refused') is True,
            'Actual animated continuation/committed refusal observations required')
    pair = proof.get('animation_step', {})
    def state(value):
        require(isinstance(value,dict) and set(value)=={'playback','completed','transition'}, 'Complete animation observation')
        p = value['playback']
        require(isinstance(p,dict) and set(p)=={'clip','seconds','speed','playing','looping'}, 'Complete playback observation')
        require(type(p['clip']) is int and p['clip']==0 and type(p['seconds']) in (int,float)
                and math.isfinite(p['seconds']) and 0<=p['seconds']<2 and type(p['speed']) in (int,float)
                and p['speed']==1 and p['playing'] is True and p['looping'] is True
                and value['completed'] is False and value['transition'] is None, 'Fixed native animated fixture state')
        return p
    before, after = pair.get('before'), pair.get('after')
    a, b = state(before), state(after)
    require(abs(a['seconds']-(.5+proof['paused_tick']/60)%2)<1e-9,
            'Native clock matches authored start and fixed native tick')
    require(abs(b['seconds']-(a['seconds']+1/60)%2)<1e-9, 'Existing native animation clock must continue')
    stages = proof['stages']
    require(all(s.get('animation')==before for s in stages), 'Every paused stage retains complete animation state')
    revisions = [s['history']['revision'] for s in stages]
    first = revisions[0]
    require(revisions==[first,first+1,first+2,first+3,first+4,first+5], 'Committed incompatible edit and recovery revisions')
    refusal = proof.get('incompatible_refresh_refusal', {})
    event = refusal.get('event', {})
    authored = refusal.get('authored_fingerprint')
    accepted = stages[3]['accepted_fingerprint']
    require(isinstance(authored,str) and len(authored)==64 and all(c in '0123456789abcdef' for c in authored)
            and authored!=accepted and refusal.get('accepted_fingerprint')==accepted
            and refusal.get('committed_revision')==revisions[4], 'Committed authored/live catalog distinction')
    require(event.get('level')=='error' and isinstance(event.get('id'),str) and isinstance(event.get('message'),str)
            and event['message'].startswith('Native Play asset refresh:')
            and 'animation clip bindings or duration table changed' in event['message']
            and refusal.get('native_code_available') is False and 'code' not in refusal,
            'Actual user-visible refresh failure; no invented native error code')
    require(stages[4].get('authored_fingerprint')==authored and stages[4].get('refresh_error')==event,
            'Refused frame retains prior catalog alongside new authored recipe')
