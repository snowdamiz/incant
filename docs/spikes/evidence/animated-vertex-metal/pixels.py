"""Bounded data-only RGBA replay; never launches an archived executable."""
import json
from pathlib import Path

SIZE = 160 * 96 * 4


def need(condition, message):
    if not condition:
        raise ValueError(message)


def mask(data):
    return [r > 150 and r-g > 128 and r-b > 128 and a > 200 for r,g,b,a in zip(data[::4],data[1::4],data[2::4],data[3::4])]


def validate(root, kind):
    root = Path(root)
    names = {'initial','replacement','continued','late_refusal_retained'}
    names |= {'invalid_layout_retained'} if kind == 'clip' else {'fresh_reference','stale_candidate_retained'}
    if kind == 'geometry':
        names.add('invalid_topology_retained')
    need({p.stem for p in root.glob('*.rgba')} == names, 'Exact raw capture inventory')
    images = {}
    for name in names:
        path = root / f'{name}.rgba'
        need(path.is_file() and not path.is_symlink() and path.stat().st_size == SIZE, 'Bounded raw capture')
        images[name] = path.read_bytes()
    proof_path = root / 'proof.json'
    need(proof_path.stat().st_size <= 16*1024, 'Bounded proof')
    p = json.loads(proof_path.read_text())
    need((p['width'],p['height'],p['tick'],p['vm_counter_before'],p['vm_counter_after']) == (160,96,2,1,3), 'Dimensions and VM chronology')
    need(abs(p['same_clock_seconds']-.5)<1e-12 and abs(p['continued_clock_seconds']-(.5+1/60))<1e-12, 'Same and continued clocks')
    need(p['world_identity_preserved'] is True and p['fresh_motion_owner'] is True and p['first_replacement_has_previous'] is False, 'Identity and fresh history')
    need(p['catalog_generation'] == 1 and p['frames'] == p['presents'] == (6 if kind == 'tangents' else 7), 'Publication/presentation accounting')
    old, new = images['initial'], images['replacement']
    need(old != new and images['late_refusal_retained'] == old and p['late_refusal_pixels_equal'] is True, 'Changed content and late refusal')
    if kind == 'clip':
        need(images['invalid_layout_retained'] == old and p['invalid_layout_pixels_equal'] is True, 'Invalid clip layout retention')
        a,b = mask(old),mask(new)
        need(sum(a)>0 and p['old_red']==p['replacement_red']==sum(a)==sum(b) and p['shift_pixels']==12, 'Fixed clip coverage')
        need(all(b[y*160+x] == (x>=12 and a[y*160+x-12]) for y in range(96) for x in range(160)), 'Exact clip translation')
    else:
        need(images['fresh_reference']==new==images['stale_candidate_retained'], 'Full fresh/stale RGBA equality')
        need(p['full_frame_matches_fresh'] is True and p['stale_candidate_pixels_equal'] is True and p['source_and_cache_deleted_before_publication'] is True, 'Publication assertions')
        need(p['target_drops']==1 and 'backend: Metal' in p['adapter'], 'Observed adapter and owner teardown')
        need(len(p['original_fingerprint'])==len(p['replacement_fingerprint'])==64 and p['original_fingerprint']!=p['replacement_fingerprint'], 'Distinct content identities')
        oracle=p['pixel_oracle']
        if kind == 'geometry':
            need(images['invalid_topology_retained']==old and p['invalid_topology_pixels_equal'] is True, 'Topology refusal retention')
            a,b=mask(old),mask(new)
            removed=sum(x and not y for x,y in zip(a,b));added=sum(y and not x for x,y in zip(a,b))
            need(0<sum(b)<sum(a) and removed>0 and added==0, 'Changed contained silhouette')
            need(oracle=={'kind':'geometry_silhouette','old_red':sum(a),'new_red':sum(b),'removed':removed,'added':added,'vertex_delta_x':.25}, 'Recomputed silhouette counters')
        else:
            need(kind == 'tangents' and p['invalid_topology_pixels_equal'] is None, 'Scoped tangent case')
            changed=sum(any(abs(old[(y*160+x)*4+c]-new[(y*160+x)*4+c])>8 for c in range(3)) for y in range(48,54) for x in range(72,78))
            need(changed==36 and oracle=={'kind':'tangent_only_shading','interior_changed':36,'material_image_position_normal_uv_identical':True,'old_w':1,'new_w':-1}, 'Interior tangent response and source assertions')
    return {'kind':kind,'captures':len(images),'data_replay':True,'native_reexecuted':False}
