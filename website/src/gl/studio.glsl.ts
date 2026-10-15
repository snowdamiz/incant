import { common, lighthouse } from './lighthouse.glsl'

/*
 * Modeling viewport: the lighthouse on a turntable under studio light.
 * Each geometry-graph step drives one g* parameter, so the model rebuilds as
 * nodes are selected. Steps before Output render in clay; Output adds materials.
 */
export const studioFragment = /* glsl */ `#version 300 es
precision highp float;
out vec4 outColor;

uniform vec2 uRes;
uniform float uTime;
uniform float uOrbit;
uniform vec4 uStages;   // taper, gallery, windows, door
uniform vec2 uStages2;  // rocks, final materials
uniform float uFocus;   // part to highlight: 0 none, 1 tower, 2 top, 3 windows, 4 door, 5 rocks

${common}
${lighthouse}

const vec3 TARGET = vec3(0., 5.4, 0.);

float mapD(vec3 p) { return lighthouseD(p); }

vec3 calcNormal(vec3 p) {
  vec2 e = vec2(1., -1.) * .0015;
  return normalize(e.xyy * mapD(p + e.xyy) + e.yyx * mapD(p + e.yyx) + e.yxy * mapD(p + e.yxy) + e.xxx * mapD(p + e.xxx));
}

float softShadow(vec3 ro, vec3 rd) {
  float res = 1., t = .04;
  for (int i = 0; i < 40; i++) {
    float h = mapD(ro + rd * t);
    res = min(res, 9. * h / t);
    t += clamp(h, .03, .5);
    if (res < .002 || t > 14.) break;
  }
  return clamp(res, 0., 1.);
}

float ao(vec3 p, vec3 n) {
  float o = 0., s = 1.;
  for (int i = 1; i <= 4; i++) {
    float h = .06 + .12 * float(i);
    o += (h - mapD(p + n * h)) * s;
    s *= .7;
  }
  return clamp(1. - 1.6 * o, 0., 1.);
}

float part(vec3 p) {
  float dRocks = lhRocks(p), dTop = lhTop(p), dTower = lhTower(p);
  if (dRocks < dTop && dRocks < dTower) return 5.;
  if (dTop < dTower) return 2.;
  if (gWindows > .01 && lhWindows(p) < .08) return 3.;
  if (gDoor > .01 && lhDoor(p) < .08) return 4.;
  return 1.;
}

bool cylHit(vec3 ro, vec3 rd, float r, float y0, float y1, out float tn, out float tf) {
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - r * r;
  float h = b * b - a * c;
  if (h < 0.) return false;
  h = sqrt(h);
  tn = (-b - h) / a; tf = (-b + h) / a;
  // clip against slab
  float s0 = (y0 - ro.y) / rd.y, s1 = (y1 - ro.y) / rd.y;
  tn = max(tn, min(s0, s1)); tf = min(tf, max(s0, s1));
  tn = max(tn, 0.);
  return tf > tn;
}

vec3 background(vec2 q, vec3 rd) {
  vec3 col = mix(vec3(.052, .056, .066), vec3(.085, .09, .105), smoothstep(-.2, .6, rd.y));
  col *= 1. - .35 * length(q - .5);
  return col;
}

void main() {
  vec2 frag = gl_FragCoord.xy;
  vec2 uv = (frag - .5 * uRes) / uRes.y;
  vec2 q = frag / uRes;

  gTaper = uStages.x; gGallery = uStages.y; gWindows = uStages.z; gDoor = uStages.w;
  gRocks = uStages2.x;
  float fin = uStages2.y;

  float a = uOrbit;
  vec3 ro = TARGET + vec3(sin(a) * 25., 4.6, -cos(a) * 25.);
  vec3 fw = normalize(TARGET - ro);
  vec3 rt = normalize(cross(vec3(0., 1., 0.), fw));
  vec3 up = cross(fw, rt);
  vec3 rd = normalize(uv.x * rt + uv.y * up + 1.4 * fw);

  vec3 col = background(q, rd);
  vec3 key = normalize(vec3(-.55, .78, -.35));
  key.xz = rot(a * .35) * key.xz;

  // Floor: a soft studio cyclorama with a faint grid.
  float tFloor = rd.y < 0. ? -ro.y / rd.y : 1e5;
  if (tFloor < 1e4) {
    vec3 p = ro + rd * tFloor;
    float r = length(p.xz);
    vec3 fc = vec3(.075, .08, .092) * (1. - smoothstep(4., 26., r) * .6);
    float foot = tFloor / (uRes.y * 1.4 * max(-rd.y, .02));
    vec2 g = abs(fract(p.xz - .5) - .5) / max(foot, 1e-4);
    float line = 1. - min(min(g.x, g.y), 1.);
    fc += vec3(.12, .13, .16) * line * .25 * (1. - smoothstep(3., 16., r));
    float sh = softShadow(p + vec3(0., .01, 0.), key);
    float contact = 1. - .55 * exp(-max(lighthouseD(p), 0.) * 2.2);
    fc *= mix(.45, 1., sh) * contact;
    col = mix(fc, col, smoothstep(14., 30., r));
  }

  // Model.
  float tn, tf;
  if (cylHit(ro, rd, 4.6, -.1, LH_H + 3.2, tn, tf)) {
    float t = tn;
    bool hit = false;
    for (int i = 0; i < 110; i++) {
      float d = mapD(ro + rd * t);
      if (d < .0012 * t) { hit = true; break; }
      t += d * .9;
      if (t > tf) break;
    }
    if (hit && t < tFloor) {
      vec3 p = ro + rd * t;
      vec3 n = calcNormal(p);
      float pid = part(p);

      vec3 clay = vec3(.62, .62, .64);
      vec3 alb = clay;
      vec3 emit = vec3(0.);
      if (fin > 0.) {
        float m = lighthouseM(p);
        vec3 mat = clay;
        if (m == 1.) mat = vec3(.82, .82, .84);
        else if (m == 2.) mat = vec3(.6, .09, .08);
        else if (m == 3.) mat = vec3(.09, .1, .11);
        else if (m == 4.) {
          float mull = smoothstep(.8, .9, abs(fract(atan(p.z, p.x) * 1.6) - .5) * 2.);
          mat = vec3(.06);
          emit = vec3(1., .72, .38) * 1.5 * fin * (1. - mull * .85);
        }
        else if (m == 5.) mat = mix(vec3(.2, .19, .2), vec3(.36, .32, .3), noise3(p * 3.));
        else if (m == 6.) { mat = vec3(.05); emit = vec3(1., .72, .38) * .8 * fin; }
        else if (m == 7.) mat = vec3(.16, .1, .07);
        alb = mix(clay, mat, fin);
      }

      float occ = ao(p, n);
      float dif = max(dot(n, key), 0.) * softShadow(p + n * .01, key);
      float sky = .5 + .5 * n.y;
      float back = max(dot(n, normalize(vec3(-key.x, .2, -key.z))), 0.);
      vec3 h = normalize(key - rd);
      float spe = pow(max(dot(n, h), 0.), 40.) * dif * (fin > .5 ? .25 : .12);
      float fre = pow(1. - max(dot(n, -rd), 0.), 4.);

      vec3 lit = alb * (vec3(1.05, .98, .9) * dif * 1.5 + vec3(.2, .23, .3) * sky * occ + vec3(.25, .22, .3) * back * .4 * occ);
      lit += vec3(1.) * spe + vec3(.35, .4, .55) * fre * .18 * occ;

      // The part the selected node adds glows in the accent colour.
      if (uFocus > .5 && abs(pid - uFocus) < .5) {
        float pulse = .78 + .22 * sin(uTime * 3.);
        lit = mix(lit, vec3(.55, .6, 1.) * (.45 + dif * .85), .72 * pulse);
        lit += vec3(.55, .6, 1.) * (fre * .7 + .06);
      }
      col = lit + emit;
    }
  }

  col = col / (1. + col * .35);
  col = pow(col, vec3(1. / 2.2));
  col *= .9 + .1 * smoothstep(1.2, .3, length(q - .5));
  col += (hash12(frag + fract(uTime) * 91.) - .5) * .012;
  outColor = vec4(col, 1.);
}
`
