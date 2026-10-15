import { common, lighthouse } from './lighthouse.glsl'

/*
 * "Blue hour": a lighthouse on a headland after sunset, rendered live.
 * Sea is an analytic plane with wave normals; the headland and tower are
 * ray-marched only inside their bounding box; the beam is an analytic
 * in-scattering integral, so the whole frame stays cheap enough for laptops.
 */
export const coastFragment = /* glsl */ `#version 300 es
precision highp float;
out vec4 outColor;

uniform vec2 uRes;
uniform float uTime;
uniform vec3 uCamPos;
uniform vec3 uCamTarget;
uniform float uFov;
uniform float uBeamAngle;
uniform float uBeam;      // 0..1 rotating beam
uniform float uPreview;   // 0..1 dashed preview beam (editor review state)
uniform float uLamp;      // 0..1 lantern glow
uniform float uGrid;      // 0..1 editor grid on the water
uniform float uSelect;    // 0..1 selection outline on the tower
uniform float uGrain;     // film grain amount

${common}
${lighthouse}

const vec3 LH_POS = vec3(14.5, 3.05, 64.);
const vec3 SUN_DIR = vec3(-.62, .0, .78);           // set sun, behind the horizon
const vec3 MOON_DIR = vec3(.42, .56, -.71);          // behind the camera, high right
const vec3 LAMP_COL = vec3(1., .72, .38);
float gPixel = .002; // world-per-unit-distance size of one pixel, set in main()

vec3 lampPos() { return LH_POS + vec3(0., LH_H + .86, 0.); }

// ---------- sky ----------
float hills(float az) {
  // Distant coastline silhouettes along the horizon.
  float h = .012 + .022 * fbm2(vec2(az * 9., 1.3)) ;
  h *= smoothstep(.15, -.25, az);                 // only to the left of frame
  float far = .006 + .01 * fbm2(vec2(az * 22., 7.));
  return max(h, far * smoothstep(.6, .1, az));
}

vec3 skyBase(vec3 rd) {
  float y = max(rd.y, 0.);
  vec3 zenith = vec3(.006, .010, .030);
  vec3 upper = vec3(.020, .030, .085);
  vec3 lower = vec3(.085, .075, .165);
  vec3 horizon = vec3(.32, .19, .26);
  vec3 col = mix(horizon, lower, smoothstep(0., .07, y));
  col = mix(col, upper, smoothstep(.04, .26, y));
  col = mix(col, zenith, smoothstep(.2, .75, y));
  // Afterglow where the sun went down.
  vec2 hz = normalize(rd.xz);
  float toward = max(dot(hz, normalize(SUN_DIR.xz)), 0.);
  col += vec3(1., .42, .22) * pow(toward, 5.) * exp(-y * 16.) * .55;
  col += vec3(.9, .5, .45) * pow(toward, 2.) * exp(-y * 6.) * .08;
  return col;
}

vec3 sky(vec3 rd, bool withStars) {
  vec3 col = skyBase(rd);
  float y = rd.y;
  // Thin cloud banks near the horizon, lit from below by the afterglow.
  if (y > 0.) {
    vec2 cp = rd.xz / (y + .04);
    float c = fbm2(cp * vec2(.5, 1.6) + vec2(uTime * .006, 0.));
    c = smoothstep(.52, .86, c) * smoothstep(.0, .05, y) * smoothstep(.32, .08, y);
    vec2 hz = normalize(rd.xz);
    float toward = max(dot(hz, normalize(SUN_DIR.xz)), 0.);
    vec3 cloudCol = mix(vec3(.07, .06, .12), vec3(.55, .26, .24), pow(toward, 3.) * .9);
    col = mix(col, cloudCol, c * .75);
  }
  if (withStars && y > .02) {
    vec2 sp = rd.xz / (y + .3) * 160.;
    vec2 cell = floor(sp);
    float h = hash12(cell);
    if (h > .985) {
      vec2 f = fract(sp) - .5 - (vec2(hash12(cell + 3.1), hash12(cell + 7.7)) - .5) * .6;
      float tw = .65 + .35 * sin(uTime * (1. + h * 3.) + h * 40.);
      float s = smoothstep(.16, 0., length(f)) * (h - .985) * 66. * tw;
      col += vec3(.75, .82, 1.) * s * smoothstep(.02, .25, y);
    }
  }
  // Coastline on the horizon.
  float az = atan(rd.x, rd.z);
  float hh = hills(az);
  if (y < hh && y > -.002) {
    vec3 hillCol = mix(vec3(.035, .035, .07), skyBase(vec3(rd.x, 0., rd.z)) * .55, .55);
    col = mix(col, hillCol, smoothstep(hh, hh - .002, y));
  }
  return col;
}

// ---------- scene ----------
float sdCapsule2(vec2 p, vec2 a, vec2 b, float r) {
  vec2 pa = p - a, ba = b - a;
  float h = clamp(dot(pa, ba) / dot(ba, ba), 0., 1.);
  return length(pa - ba * h) - r;
}

// Coastline in plan view: negative on land.
float landShape(vec2 xz) {
  vec2 q = xz - LH_POS.xz;
  float d = sdCapsule2(q, vec2(-1., 0.), vec2(42., 18.), 6.5);
  d = min(d, length(q - vec2(30., 34.)) - 20.);
  d = min(d, sdCapsule2(q, vec2(-9.5, -3.5), vec2(-8., -2.5), 1.2));   // sea stack
  d += (fbm2(xz * .09) - .5) * 7.;
  return d;
}

float landHeight(vec2 xz) {
  float s = landShape(xz);
  vec2 q = xz - LH_POS.xz;
  float cliff = smoothstep(1.4, -1.8, s);
  float inland = max(-s - 3., 0.);
  float h = -1.2 + cliff * 4.3 + inland * .32 + fbm2(xz * .35) * 1.6 * cliff;
  // Boulders along the waterline.
  h += fbm2(xz * .9) * 1.4 * smoothstep(3.5, 0., abs(s + .4));
  // A level ledge for the tower and the keeper's house.
  float pad = smoothstep(7.5, 4.5, length((q - vec2(2.2, 1.)) * vec2(1., 1.25)));
  return mix(h, LH_POS.y - .05, pad);
}

float headland(vec3 p) {
  return (p.y - landHeight(p.xz)) * .42;
}

float houseD(vec3 p) {
  vec3 q = p - (LH_POS + vec3(4.3, 0., 2.2));
  q.xz = rot(.5) * q.xz;
  float walls = sdBox(q - vec3(0., .85, 0.), vec3(1.55, .85, 1.05));
  vec3 r = q - vec3(0., 1.7, 0.);
  float roof = max(sdBox(r, vec3(1.7, .8, 1.2)), (abs(r.z) * .8 + r.y * .9) - .62);
  return min(walls, roof);
}

float mapScene(vec3 p, out float id) {
  float dh = headland(p);
  vec3 q = p - LH_POS;
  float lb = sdCylinder(q - vec3(0., 6., 0.), 6.6, 4.9);
  float dl = lb > .4 ? lb : lighthouseD(q);
  vec3 hq = p - (LH_POS + vec3(4.3, 1.3, 2.2));
  float hb = length(hq) - 3.;
  float dk = hb > .4 ? hb : houseD(p);
  float d = dh; id = 5.;
  if (dk < d) { d = dk; id = 8.; }
  if (dl < d) { d = dl; id = 0.; }
  return d;
}
float mapD(vec3 p) { float id; return mapScene(p, id); }

vec3 sceneNormal(vec3 p, float t) {
  vec2 e = vec2(1., -1.) * (.002 + t * .0006);
  return normalize(e.xyy * mapD(p + e.xyy) + e.yyx * mapD(p + e.yyx) + e.yxy * mapD(p + e.yxy) + e.xxx * mapD(p + e.xxx));
}

bool boxHit(vec3 ro, vec3 rd, vec3 bmin, vec3 bmax, out float tn, out float tf) {
  vec3 inv = 1. / rd;
  vec3 t0 = (bmin - ro) * inv, t1 = (bmax - ro) * inv;
  vec3 a = min(t0, t1), b = max(t0, t1);
  tn = max(max(a.x, a.y), max(a.z, 0.));
  tf = min(min(b.x, b.y), b.z);
  return tf > tn;
}

// Marches the headland region; returns hit distance or -1.
float march(vec3 ro, vec3 rd, float tmax, int steps, out float id) {
  id = -1.;
  float tn, tf;
  if (!boxHit(ro, rd, LH_POS + vec3(-17., -3.2, -12.), LH_POS + vec3(62., 13.6, 60.), tn, tf)) return -1.;
  float t = tn;
  tf = min(tf, tmax);
  for (int i = 0; i < 140; i++) {
    if (i >= steps || t > tf) break;
    vec3 p = ro + rd * t;
    float d = mapScene(p, id);
    if (d < .0015 * t + .002) return t;
    t += d * .8;
  }
  id = -1.;
  return -1.;
}

// ---------- beam ----------
vec3 beamDir(float s) { return normalize(vec3(cos(uBeamAngle) * s, -.022, sin(uBeamAngle) * s)); }

float beamScatter(vec3 ro, vec3 rd, float tmax, vec3 apex, vec3 dir, float dashed) {
  vec3 w = ro - apex;
  float b = dot(rd, dir);
  float denom = max(1. - b * b, 1e-4);
  float dd = dot(rd, w), e = dot(dir, w);
  float s = clamp((b * e - dd) / denom, 0., tmax);
  vec3 ps = ro + rd * s;
  float u = dot(ps - apex, dir);
  if (u < .2) return 0.;
  float dist = length(ps - apex - dir * u);
  float r = .32 + u * .05;
  float g = exp(-(dist * dist) / (r * r));
  float dens = .25 + .75 * exp(-max(ps.y, 0.) * .07);
  float v = g * dens / (r * max(sqrt(denom), .06)) / (1. + u * .006);
  if (dashed > 0.) v *= .55 + .45 * step(.5, fract(u * .09 - uTime * .4));
  return v * smoothstep(.2, 1.4, u);
}

vec3 beams(vec3 ro, vec3 rd, float tmax) {
  vec3 apex = lampPos();
  vec3 col = vec3(0.);
  if (uBeam > 0.) {
    float b = beamScatter(ro, rd, tmax, apex, beamDir(1.), 0.) + beamScatter(ro, rd, tmax, apex, beamDir(-1.), 0.);
    col += LAMP_COL * b * .16 * uBeam;
  }
  if (uPreview > 0.) {
    float b = beamScatter(ro, rd, tmax, apex, beamDir(1.), 1.) + beamScatter(ro, rd, tmax, apex, beamDir(-1.), 1.);
    col += vec3(.55, .6, 1.) * b * .13 * uPreview;
  }
  return col;
}

// ---------- shading ----------
vec3 shade(vec3 p, vec3 rd, float t, float id) {
  vec3 n = sceneNormal(p, t);
  float matId = id;
  if (id < .5) matId = lighthouseM(p - LH_POS);
  vec3 alb = vec3(.25, .2, .2);
  vec3 emit = vec3(0.);
  if (matId == 1.) alb = vec3(.78, .78, .8);
  else if (matId == 2.) alb = vec3(.46, .07, .07);
  else if (matId == 3.) alb = vec3(.08, .085, .1);
  else if (matId == 4.) {
    vec3 q = p - LH_POS;
    float a = atan(q.z, q.x);
    float mull = smoothstep(.82, .9, abs(fract(a * 1.6) - .5) * 2.);
    alb = vec3(.05);
    emit = LAMP_COL * (2.4 + 1.5 * uBeam) * uLamp * (1. - mull * .8) + LAMP_COL * .05;
  } else if (matId == 5.) {
    float strata = noise3(p * vec3(.5, 3.2, .5));
    alb = mix(vec3(.12, .105, .11), vec3(.25, .21, .2), strata);
    alb *= .65 + .35 * noise3(p * 2.3);
    // Grass on level ground.
    float flat_ = smoothstep(.75, .92, n.y);
    alb = mix(alb, vec3(.07, .09, .06), flat_ * smoothstep(1.8, 2.6, p.y));
  } else if (matId == 6.) { alb = vec3(.05); emit = LAMP_COL * .9; }
  else if (matId == 7.) alb = vec3(.07, .045, .035);
  else if (matId == 8.) {
    vec3 q = p - (LH_POS + vec3(4.3, 0., 2.2));
    alb = q.y > 1.62 ? vec3(.1, .1, .12) : vec3(.55, .54, .52);
    // A lit window on the keeper's house.
    vec3 wq = q; wq.xz = rot(.5) * wq.xz;
    float win = sdBox(wq - vec3(-.55, .95, -1.05), vec3(.22, .26, .08));
    if (win < .02) { alb = vec3(.04); emit = LAMP_COL * 1.1; }
  }

  float occ = clamp(.35 + .65 * (mapD(p + n * .6) / .6), 0., 1.);
  vec3 skyAmb = mix(vec3(.012, .012, .02), vec3(.04, .05, .11), n.y * .5 + .5);
  vec3 col = alb * skyAmb * occ;
  // Moonlight from behind the camera.
  col += alb * vec3(.32, .38, .58) * (matId == 5. ? .6 : .22) * max(dot(n, normalize(MOON_DIR)), 0.);
  // Afterglow rim from the horizon.
  float rim = pow(clamp(1. + dot(rd, n), 0., 1.), 3.) * max(dot(n, normalize(vec3(SUN_DIR.x, .15, SUN_DIR.z))), 0.);
  col += alb * vec3(1., .5, .35) * rim * .5;
  // Warm spill from the lantern.
  vec3 L = lampPos() - p;
  float dl = length(L);
  col += alb * LAMP_COL * max(dot(n, L / dl), 0.) * 2.2 * uLamp / (1. + dl * dl * .35);
  return col + emit;
}

// ---------- sea ----------
vec2 waveGrad(vec2 x, float t) {
  vec2 g = vec2(0.);
  vec2 dirs[5] = vec2[5](vec2(.9, .44), vec2(-.36, .93), vec2(.67, -.74), vec2(.17, .98), vec2(-.8, .6));
  float lens[5] = float[5](9., 5.3, 3.1, 1.9, 1.15);
  for (int i = 0; i < 5; i++) {
    vec2 d = normalize(dirs[i]);
    float k = 6.2831 / lens[i];
    float w = sqrt(9.8 * k);
    float a = .055 * lens[i] / 6.2831;
    g += d * (a * k * cos(dot(d, x) * k - w * t + float(i) * 1.7));
  }
  // Fine chop.
  float e = .08;
  vec2 cx = x * 2.1 + vec2(t * .3, -t * .2);
  float n0 = noise2(cx);
  g += vec2(noise2(cx + vec2(e, 0.)) - n0, noise2(cx + vec2(0., e)) - n0) / e * .045;
  return g;
}

vec3 sea(vec3 ro, vec3 rd, float t) {
  vec3 p = ro + rd * t;
  vec2 g = waveGrad(p.xz, uTime);
  float fade = 1. / (1. + t * .045);
  vec3 n = normalize(vec3(-g.x * fade, 1., -g.y * fade));
  vec3 refl = reflect(rd, n);
  refl.y = abs(refl.y);
  float fres = .02 + .98 * pow(1. - max(dot(n, -rd), 0.), 5.);
  vec3 env = sky(refl, false);

  // Reflected headland and tower.
  float rid;
  float rt = march(p + refl * .05, refl, 120., 56, rid);
  if (rt > 0.) env = shade(p + refl * rt, refl, rt, rid);
  env += beams(p, refl, rt > 0. ? rt : 200.);

  vec3 deep = vec3(.004, .010, .018);
  vec3 col = mix(deep, env, fres);

  // Foam where the swell meets the rocks.
  vec2 lq = p.xz - LH_POS.xz;
  if (lq.x > -24. && lq.x < 70. && lq.y > -16. && lq.y < 64.) {
    float h = landHeight(p.xz);
    float edge = smoothstep(-1.1, 0., h);
    float churn = noise2(p.xz * 1.7 + vec2(uTime * .5, -uTime * .3)) * noise2(p.xz * .6 - uTime * .2);
    col += vec3(.16, .18, .24) * edge * (.25 + 1.5 * churn) * (1. - fres * .5);
  }

  // Glints of the lantern on the swell.
  vec3 toLamp = normalize(lampPos() - p);
  float glint = pow(max(dot(refl, toLamp), 0.), 260.);
  col += LAMP_COL * glint * (2.4 + 3. * uBeam) * uLamp;
  col += LAMP_COL * pow(max(dot(refl, toLamp), 0.), 30.) * .05 * uLamp;

  // The beam grazing the water.
  if (uBeam > 0.) {
    for (int k = 0; k < 2; k++) {
      vec3 bd = beamDir(k == 0 ? 1. : -1.);
      vec3 v = p - lampPos();
      float u = dot(v, bd);
      if (u > 0.) {
        float off = length(v - bd * u) / (.32 + u * .05);
        col += LAMP_COL * exp(-off * off * .5) * .05 * uBeam * (.3 + .7 * min(fres * 4., 1.)) / (1. + u * .02);
      }
    }
  }

  // Editor grid, antialiased by the pixel footprint on the water plane (no derivatives in this branch).
  if (uGrid > 0.) {
    float foot = gPixel * t / max(-rd.y, .02);
    vec2 gp = p.xz / 2.;
    vec2 l = abs(fract(gp - .5) - .5) * 2. / max(foot, 1e-4);
    float line = 1. - min(min(l.x, l.y), 1.);
    float fadeG = exp(-t * .022) * smoothstep(.0, .6, foot < .5 ? 1. : .5 / foot);
    vec2 ax = abs(p.xz - LH_POS.xz) / max(foot, 1e-4);
    col += vec3(.38, .42, .55) * line * .06 * fadeG * uGrid;
    col += vec3(.95, .35, .35) * (1. - min(ax.y * .6, 1.)) * .25 * exp(-t * .015) * uGrid;
    col += vec3(.35, .55, 1.) * (1. - min(ax.x * .6, 1.)) * .25 * exp(-t * .015) * uGrid;
  }
  return col;
}

// ---------- post ----------
vec3 aces(vec3 x) { return clamp((x * (2.51 * x + .03)) / (x * (2.43 * x + .59) + .14), 0., 1.); }

void main() {
  vec2 frag = gl_FragCoord.xy;
  vec2 uv = (frag - .5 * uRes) / uRes.y;

  vec3 ro = uCamPos;
  vec3 fw = normalize(uCamTarget - ro);
  vec3 rt = normalize(cross(vec3(0., 1., 0.), fw));
  vec3 up = cross(fw, rt);
  float f = 1. / tan(uFov * .5);
  gPixel = 2. / (f * uRes.y);
  vec3 rd = normalize(uv.x * rt + uv.y * up + f * .5 * fw);

  float id;
  float tSea = rd.y < 0. ? -ro.y / rd.y : 1e5;
  float t = march(ro, rd, min(tSea, 400.), 120, id);

  vec3 col;
  float tEnd;
  if (t > 0.) {
    vec3 p = ro + rd * t;
    col = shade(p, rd, t, id);
    tEnd = t;
    // Haze toward the horizon.
    col = mix(col, skyBase(vec3(rd.x, 0., rd.z)) * .6, 1. - exp(-t * .004));
    // The selected Lantern (everything above the tower's top) gets an accent rim.
    if (uSelect > 0. && id < .5 && p.y - LH_POS.y > LH_H - .05) {
      vec3 n = sceneNormal(p, t);
      float rim = pow(1. - abs(dot(n, rd)), 2.5);
      col = mix(col, vec3(.55, .6, 1.), (.1 + .75 * rim) * uSelect);
    }
  } else if (rd.y < 0.) {
    col = sea(ro, rd, tSea);
    col = mix(col, skyBase(vec3(rd.x, 0., rd.z)) * .5, 1. - exp(-tSea * .0035));
    tEnd = tSea;
  } else {
    col = sky(rd, true);
    tEnd = 600.;
  }

  // Volumetric beam.
  col += beams(ro, rd, tEnd);

  // Lantern halo and the flash when the beam faces the camera.
  vec3 lp = lampPos();
  vec3 toL = normalize(lp - ro);
  float ang = acos(clamp(dot(rd, toL), -1., 1.));
  float facing = max(max(dot(beamDir(1.), -toL), dot(beamDir(-1.), -toL)), 0.);
  float flash = pow(facing, 48.) * uBeam;
  col += LAMP_COL * (exp(-ang * ang * 9000.) * .7 + exp(-ang * 60.) * .05) * uLamp;
  col += LAMP_COL * (exp(-ang * 14.) * .35 + exp(-ang * ang * 600.) * .9) * flash;
  // Horizontal streak.
  vec3 lc = vec3(dot(toL, rt), dot(toL, up), dot(toL, fw));
  vec2 lUv = lc.xy / lc.z * f * .5;
  vec2 dUv = uv - lUv;
  col += vec3(.95, .75, .55) * exp(-abs(dUv.y) * 260.) * exp(-abs(dUv.x) * 2.6) * (.04 * uLamp + .5 * flash);

  // Grade.
  col *= 1.25;
  col = aces(col);
  col = pow(col, vec3(1. / 2.2));
  col = mix(col, col * vec3(.92, .96, 1.08), .5);
  vec2 q = frag / uRes;
  col *= .55 + .45 * pow(16. * q.x * q.y * (1. - q.x) * (1. - q.y), .18);
  col += (hash12(frag + fract(uTime * 13.7) * 100.) - .5) * uGrain;
  outColor = vec4(col, 1.);
}
`
