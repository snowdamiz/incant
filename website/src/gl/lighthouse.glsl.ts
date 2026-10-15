/*
 * Shared GLSL: hashing, noise, distance primitives and the lighthouse model.
 * The model is built from the same steps as the geometry-graph demo, so the
 * modeling viewport can grow each part (gTaper ... gRocks, 0 to 1) and the
 * coast scene renders the finished tower with every step at 1.
 */
export const common = /* glsl */ `
float hash12(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * .1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
float hash13(vec3 p3) { p3 = fract(p3 * .1031); p3 += dot(p3, p3.zyx + 31.32); return fract((p3.x + p3.y) * p3.z); }

float noise2(vec2 x) {
  vec2 i = floor(x); vec2 f = fract(x); f = f * f * (3. - 2. * f);
  return mix(mix(hash12(i), hash12(i + vec2(1, 0)), f.x), mix(hash12(i + vec2(0, 1)), hash12(i + vec2(1, 1)), f.x), f.y);
}
float noise3(vec3 x) {
  vec3 i = floor(x); vec3 f = fract(x); f = f * f * (3. - 2. * f);
  return mix(mix(mix(hash13(i), hash13(i + vec3(1, 0, 0)), f.x), mix(hash13(i + vec3(0, 1, 0)), hash13(i + vec3(1, 1, 0)), f.x), f.y),
             mix(mix(hash13(i + vec3(0, 0, 1)), hash13(i + vec3(1, 0, 1)), f.x), mix(hash13(i + vec3(0, 1, 1)), hash13(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}
float fbm2(vec2 p) { float a = .5, s = 0.; for (int i = 0; i < 4; i++) { s += a * noise2(p); p = p * 2.07 + vec2(3.1, 1.7); a *= .5; } return s; }

float sdCappedCone(vec3 p, float h, float r1, float r2) {
  vec2 q = vec2(length(p.xz), p.y);
  vec2 k1 = vec2(r2, h);
  vec2 k2 = vec2(r2 - r1, 2. * h);
  vec2 ca = vec2(q.x - min(q.x, (q.y < 0.) ? r1 : r2), abs(q.y) - h);
  vec2 cb = q - k1 + k2 * clamp(dot(k1 - q, k2) / dot(k2, k2), 0., 1.);
  float s = (cb.x < 0. && ca.y < 0.) ? -1. : 1.;
  return s * sqrt(min(dot(ca, ca), dot(cb, cb)));
}
float sdCylinder(vec3 p, float h, float r) { vec2 d = abs(vec2(length(p.xz), p.y)) - vec2(r, h); return min(max(d.x, d.y), 0.) + length(max(d, 0.)); }
float sdBox(vec3 p, vec3 b) { vec3 q = abs(p) - b; return length(max(q, 0.)) + min(max(q.x, max(q.y, q.z)), 0.); }
float sdTorus(vec3 p, vec2 t) { vec2 q = vec2(length(p.xz) - t.x, p.y); return length(q) - t.y; }
float smin(float a, float b, float k) { float h = max(k - abs(a - b), 0.) / k; return min(a, b) - h * h * k * .25; }
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, -s, s, c); }
`

export const lighthouse = /* glsl */ `
float gTaper = 1.;
float gGallery = 1.;
float gWindows = 1.;
float gDoor = 1.;
float gRocks = 1.;

const float LH_H = 9.;
const float LH_R = 1.5;

float lhTopRadius() { return LH_R * (1. - .34 * gTaper); }
float lhRadiusAt(float y) { return mix(LH_R, lhTopRadius(), clamp(y / LH_H, 0., 1.)); }

// Windows: four slots on a spiral, carved by an array step.
float lhWindows(vec3 p) {
  float d = 1e3;
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    float y = 2.3 + fi * 1.8;
    vec3 q = p - vec3(0., y, 0.);
    q.xz = rot(-(fi * 1.5708 + 3.6)) * q.xz;
    float r = lhRadiusAt(y);
    d = min(d, sdBox(q - vec3(0., 0., r), vec3(.17, .36, .32) * vec3(1., gWindows, 1.)));
  }
  return d;
}

float lhDoor(vec3 p) {
  vec3 q = p - vec3(0., .78 * gDoor, 0.);
  q.xz = rot(-3.35) * q.xz;
  return sdBox(q - vec3(0., 0., LH_R), vec3(.38, .78 * gDoor, .42));
}

// Gallery deck, railing, lantern room and roof, grown from the tower top.
float lhTop(vec3 p) {
  if (gGallery < .01) return 1e3;
  float rt = lhTopRadius();
  vec3 q = (p - vec3(0., LH_H, 0.)) / gGallery;
  float deck = sdCylinder(q - vec3(0., .1, 0.), .1, rt + .55);
  float rail = sdTorus(q - vec3(0., .78, 0.), vec2(rt + .5, .035));
  vec3 pp = q;
  float sector = 6.2831 / 18.;
  float a = atan(pp.z, pp.x);
  a = mod(a + sector * .5, sector) - sector * .5;
  pp.xz = length(pp.xz) * vec2(cos(a), sin(a));
  float posts = sdBox(pp - vec3(rt + .5, .48, 0.), vec3(.03, .3, .03));
  float lantern = sdCylinder(q - vec3(0., .86, 0.), .66, rt * .72);
  float roof = sdCappedCone(q - vec3(0., 1.9, 0.), .38, rt * .9, .1);
  float vent = length(q - vec3(0., 2.36, 0.)) - .13;
  float d = min(min(deck, min(rail, posts)), min(lantern, min(roof, vent)));
  return d * gGallery;
}

float lhRocks(vec3 p) {
  if (gRocks < .01) return 1e3;
  float d = 1e3;
  for (int i = 0; i < 9; i++) {
    float fi = float(i);
    float a = fi * 2.399 + .4;
    float rr = 2.25 + hash12(vec2(fi, 3.1)) * 1.3;
    float s = (.32 + hash12(vec2(fi, 7.7)) * .42) * gRocks;
    vec3 c = vec3(cos(a) * rr, s * .35, sin(a) * rr);
    vec3 q = p - c;
    q.xz = rot(fi) * q.xz;
    d = min(d, length(q * vec3(1., 1.35, 1.15)) / 1.15 - s);
  }
  return d;
}

float lhTower(vec3 p) {
  float rt = lhTopRadius();
  float d = sdCappedCone(p - vec3(0., LH_H * .5, 0.), LH_H * .5, LH_R, rt);
  if (gWindows > .01) d = max(d, -lhWindows(p));
  if (gDoor > .01) d = max(d, -lhDoor(p));
  return d;
}

float lighthouseD(vec3 p) {
  return min(min(lhTower(p), lhTop(p)), lhRocks(p));
}

// Material ids: 1 white wall, 2 band, 3 metal, 4 lantern glass, 5 rock, 6 window, 7 door.
// The nearest part decides, so the result is stable at any hit tolerance.
float lighthouseM(vec3 p) {
  float rt = lhTopRadius();
  float dRocks = lhRocks(p);
  float dTop = lhTop(p);
  float dTower = lhTower(p);
  if (dRocks < dTop && dRocks < dTower) return 5.;
  if (dTop < dTower) {
    vec3 q = (p - vec3(0., LH_H, 0.)) / gGallery;
    if (q.y > .22 && q.y < 1.5 && length(q.xz) < rt * .72 + .07) return 4.;
    return 3.;
  }
  if (gWindows > .01 && lhWindows(p) < .06) return 6.;
  if (gDoor > .01 && lhDoor(p) < .06) return 7.;
  float band = step(.5, fract((p.y - .6) / 3.4));
  return band > .5 ? 2. : 1.;
}
`
