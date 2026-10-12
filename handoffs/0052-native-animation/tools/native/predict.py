"""predict.py <blob.json> <vx> <vy> <vw> <vh> <tick|authored>: compare a measured triangle with
the fixture's analytic pose. Read-only arithmetic; no capture, no input.

Fixture facts this model uses (all from the fixture's public files):
- rig.gltf/rig.bin: one triangle (0,0,0), (1,0,0), (0,1,0), normals +Z, every vertex weighted 1.0
  to joint 0, identity inverse bind matrix, joint rest pose identity. Clip 0 "move" interpolates
  the joint's translation (0,0,0) -> (2,0,0) over 0..2 s; clip 1 "rotate" slerps its rotation
  (0,0,0,1) -> (0,0,1,0), i.e. 0 -> 180 degrees about +Z. Each sample starts from the rest pose
  (AnimationPlayer::sample), so the rotate clip pivots on the world origin.
- animation.ts: phase = (tick - 1) % 240; phase 0 selects clip 0 at 0 s and phase 120 selects
  clip 1 at 0 s, both looping at speed 1, so the committed time at tick N is phase % 120 / 60 s
  (matching the fixture manifest's logs, e.g. tick 30 -> clip 0 at 0.483333 s).
- project.json: entity Transform identity; camera at (0.5, 0.5, 8), identity rotation (looking
  -Z), perspective, fov_degrees 60, which this model takes as the vertical field of view.

<vx> <vy> <vw> <vh> is the viewport rectangle in the capture's pixels. The blob is imgtool's
output for that rectangle.
"""

import json
import math
import sys

HALF_HEIGHT = 8 * math.tan(math.radians(30))


def pose(tick):
    if tick == 'authored':
        return 'authored (clip 0 at 0 s)', (0.0, 0.0), 0.0
    phase = (int(tick) - 1) % 240
    seconds = (phase % 120) / 60
    if phase < 120:
        return f'move at {seconds:.6f} s', (seconds, 0.0), 0.0
    return f'rotate at {seconds:.6f} s', (0.0, 0.0), math.pi * seconds / 2


def main():
    blob = json.load(open(sys.argv[1]) if sys.argv[1] != '-' else sys.stdin)
    vx, vy, vw, vh = map(float, sys.argv[2:6])
    what, (ox, oy), theta = pose(sys.argv[6])
    unit = (vh / 2) / HALF_HEIGHT
    cx, cy = vx + vw / 2, vy + vh / 2

    def screen(x, y):
        return (cx + (x - 0.5) * unit, cy - (y - 0.5) * unit)

    c, s = math.cos(theta), math.sin(theta)
    world = [(ox, oy), (ox + c, oy + s), (ox - s, oy + c)]
    predicted = [screen(*p) for p in world]
    centroid = screen(ox + (c - s) / 3, oy + (s + c) / 3)
    area = unit * unit / 2
    out = {
        'tick': sys.argv[6], 'pose': what, 'theta_deg': round(math.degrees(theta), 3),
        'unit_px': round(unit, 3),
        'predicted': {'corner': [round(v, 2) for v in predicted[0]],
                      'x_end': [round(v, 2) for v in predicted[1]],
                      'y_end': [round(v, 2) for v in predicted[2]],
                      'centroid': [round(v, 2) for v in centroid], 'area_px': round(area, 1)},
    }
    if blob.get('count'):
        measured = blob['centroid']
        out['measured'] = {'centroid': measured, 'count': blob['count'], 'vertices': blob['vertices'], 'mean': blob['mean']}
        out['centroid_error_px'] = round(math.dist(measured, centroid), 2)
        out['area_ratio'] = round(blob['count'] / area, 4)
        # Each measured extreme point against its nearest predicted corner.
        out['vertex_errors_px'] = [round(min(math.dist(v, p) for p in predicted), 2) for v in blob['vertices']]
        # Rotation about the corner from the centroid direction: the centroid sits at 45 degrees
        # from the triangle's +X leg, sqrt(2)/3 units from the corner.
        corner = predicted[0]
        angle = math.degrees(math.atan2(-(measured[1] - corner[1]), measured[0] - corner[0])) - 45
        out['measured_theta_deg_about_predicted_corner'] = round(angle, 2)
        out['measured_radius_units'] = round(math.dist(measured, corner) / unit, 4)
    print(json.dumps(out))


if __name__ == '__main__':
    main()
