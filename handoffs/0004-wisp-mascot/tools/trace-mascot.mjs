#!/usr/bin/env node
/**
 * Traces the purple wisp mascot from reference.png into vector paths.
 *
 * 1. Chrome decodes the PNG (decode.mjs); only the mascot crop is read, so the
 *    wordmark and the small example logos are never sampled.
 * 2. A "purpleness" field t = (b - r) is normalised between the white background and
 *    the body color. Dark wordmark pixels have b - r ~ 0, so they cannot leak in.
 * 3. Marching squares extracts the t = 0.5 iso-contours with sub-pixel precision:
 *    one outer silhouette and two eye openings.
 * 4. Corners (the two tail tips) are detected; each smooth run is lightly smoothed
 *    and fitted with cubic Beziers (Schneider's algorithm) to a 0.6 px tolerance.
 *
 * Output: writes editor/ui/brand/wisp-mascot.svg (master in reference pixel units)
 * and prints a JSON summary. Usage (from the worktree root):
 *   node handoffs/0004-wisp-mascot/tools/trace-mascot.mjs
 */
import { writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { decodePng } from './decode.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const CROP = { x: 400, y: 130, width: 460, height: 600 };
const BG_CHROMA = -2; // b - r of the white presentation background
const FG_CHROMA = 123; // b - r of the body (~ rgb 73 42 196)
const TOLERANCE = 0.6; // max fit error, reference pixels

const img = await decodePng(join(here, '..', 'reference.png'), CROP);
const W = img.width;
const H = img.height;

// --- body color: median of solid interior pixels ---------------------------
const solid = [[], [], []];
for (let i = 0; i < W * H; i++) {
  const r = img.data[i * 4], g = img.data[i * 4 + 1], b = img.data[i * 4 + 2];
  if (b - r > 115 && r < 90) { solid[0].push(r); solid[1].push(g); solid[2].push(b); }
}
const median = (a) => a.sort((x, y) => x - y)[a.length >> 1];
const body = solid.map(median);
const hex = `#${body.map((v) => v.toString(16).padStart(2, '0')).join('').toUpperCase()}`;

// --- field -------------------------------------------------------------------
const f = new Float32Array(W * H);
for (let i = 0; i < W * H; i++) {
  const t = (img.data[i * 4 + 2] - img.data[i * 4] - BG_CHROMA) / (FG_CHROMA - BG_CHROMA);
  f[i] = Math.min(1, Math.max(0, t));
}
const at = (x, y) => (x < 0 || y < 0 || x >= W || y >= H ? 0 : f[y * W + x]);

// --- marching squares ----------------------------------------------------------
// Edge ids: horizontal edge (x,y)-(x+1,y) => 'h,x,y'; vertical (x,y)-(x,y+1) => 'v,x,y'.
const ISO = 0.5;
const pointOn = (id) => {
  const [k, xs, ys] = id.split(',');
  const x = +xs, y = +ys;
  if (k === 'h') { const a = at(x, y), b = at(x + 1, y); return [x + (ISO - a) / (b - a), y]; }
  const a = at(x, y), b = at(x, y + 1);
  return [x, y + (ISO - a) / (b - a)];
};
const next = new Map();
for (let y = -1; y < H; y++) {
  for (let x = -1; x < W; x++) {
    const tl = at(x, y) >= ISO, tr = at(x + 1, y) >= ISO, br = at(x + 1, y + 1) >= ISO, bl = at(x, y + 1) >= ISO;
    const code = (tl ? 8 : 0) | (tr ? 4 : 0) | (br ? 2 : 0) | (bl ? 1 : 0);
    if (code === 0 || code === 15) continue;
    const T = `h,${x},${y}`, B = `h,${x},${y + 1}`, L = `v,${x},${y}`, R = `v,${x + 1},${y}`;
    // Segments oriented so the inside (>= ISO) is on the right in screen space.
    const segs = {
      1: [[L, B]], 2: [[B, R]], 3: [[L, R]], 4: [[R, T]], 5: [[L, T], [R, B]], 6: [[B, T]], 7: [[L, T]],
      8: [[T, L]], 9: [[T, B]], 10: [[T, R], [B, L]], 11: [[T, R]], 12: [[R, L]], 13: [[R, B]], 14: [[B, L]],
    }[code];
    for (const [a, b] of segs) next.set(a, b);
  }
}
const loops = [];
const seen = new Set();
for (const start of next.keys()) {
  if (seen.has(start)) continue;
  const ids = [];
  let id = start;
  while (!seen.has(id)) { seen.add(id); ids.push(id); id = next.get(id); }
  if (ids.length < 40) continue; // compression noise
  loops.push(ids.map(pointOn));
}
const area = (pts) => pts.reduce((s, p, i) => { const q = pts[(i + 1) % pts.length]; return s + p[0] * q[1] - q[0] * p[1]; }, 0) / 2;
loops.sort((a, b) => Math.abs(area(b)) - Math.abs(area(a)));
if (loops.length !== 3) throw new Error(`Expected 3 contours (body + 2 eyes), got ${loops.length}`);

// --- geometry helpers --------------------------------------------------------
const sub = (a, b) => [a[0] - b[0], a[1] - b[1]];
const add = (a, b) => [a[0] + b[0], a[1] + b[1]];
const mul = (a, s) => [a[0] * s, a[1] * s];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1];
const len = (a) => Math.hypot(a[0], a[1]);
const norm = (a) => { const l = len(a) || 1; return [a[0] / l, a[1] / l]; };
const dist = (a, b) => len(sub(a, b));

// Resample to ~1 px spacing so windows are in arc length.
function resample(pts, step = 1) {
  const out = [pts[0]];
  let carry = 0;
  for (let i = 0; i < pts.length; i++) {
    const a = pts[i], b = pts[(i + 1) % pts.length];
    const d = dist(a, b);
    let t = step - carry;
    while (t <= d) { out.push(add(a, mul(sub(b, a), t / d))); t += step; }
    carry = d - (t - step);
  }
  if (dist(out[out.length - 1], out[0]) < step * 0.5) out.pop();
  return out;
}

function findCorners(pts, k = 7, maxAngleDeg = 120) {
  const n = pts.length;
  const ang = pts.map((p, i) => {
    const a = norm(sub(pts[(i - k + n) % n], p)), b = norm(sub(pts[(i + k) % n], p));
    return (Math.acos(Math.max(-1, Math.min(1, dot(a, b)))) * 180) / Math.PI;
  });
  const corners = [];
  for (let i = 0; i < n; i++) {
    if (ang[i] >= maxAngleDeg) continue;
    let isMin = true;
    for (let j = -k; j <= k; j++) if (ang[(i + j + n) % n] < ang[i]) isMin = false;
    if (isMin && !corners.some((c) => Math.min(Math.abs(c - i), n - Math.abs(c - i)) < k)) corners.push(i);
  }
  return { corners, ang };
}

function smoothRun(run, sigma = 1.2) {
  const r = Math.ceil(sigma * 3);
  const w = [];
  for (let j = -r; j <= r; j++) w.push(Math.exp(-(j * j) / (2 * sigma * sigma)));
  return run.map((p, i) => {
    if (i === 0 || i === run.length - 1) return p;
    let sx = 0, sy = 0, sw = 0;
    for (let j = -r; j <= r; j++) {
      const q = run[Math.min(run.length - 1, Math.max(0, i + j))];
      sx += q[0] * w[j + r]; sy += q[1] * w[j + r]; sw += w[j + r];
    }
    return [sx / sw, sy / sw];
  });
}

// --- Schneider cubic fitting ---------------------------------------------------
const bez = (c, t) => {
  const mt = 1 - t;
  return add(add(mul(c[0], mt * mt * mt), mul(c[1], 3 * mt * mt * t)), add(mul(c[2], 3 * mt * t * t), mul(c[3], t * t * t)));
};
const bezD = (c, t) => {
  const mt = 1 - t;
  return add(add(mul(sub(c[1], c[0]), 3 * mt * mt), mul(sub(c[2], c[1]), 6 * mt * t)), mul(sub(c[3], c[2]), 3 * t * t));
};
const bezDD = (c, t) => add(mul(add(sub(c[2], mul(c[1], 2)), c[0]), 6 * (1 - t)), mul(add(sub(c[3], mul(c[2], 2)), c[1]), 6 * t));

function chordParams(pts) {
  const u = [0];
  for (let i = 1; i < pts.length; i++) u.push(u[i - 1] + dist(pts[i], pts[i - 1]));
  return u.map((v) => v / u[u.length - 1]);
}
function generate(pts, u, t1, t2) {
  const p0 = pts[0], p3 = pts[pts.length - 1];
  let c00 = 0, c01 = 0, c11 = 0, x0 = 0, x1 = 0;
  u.forEach((t, i) => {
    const mt = 1 - t;
    const a1 = mul(t1, 3 * mt * mt * t), a2 = mul(t2, 3 * mt * t * t);
    c00 += dot(a1, a1); c01 += dot(a1, a2); c11 += dot(a2, a2);
    const tmp = sub(pts[i], bez([p0, p0, p3, p3], t));
    x0 += dot(a1, tmp); x1 += dot(a2, tmp);
  });
  const det = c00 * c11 - c01 * c01;
  let al = det === 0 ? 0 : (x0 * c11 - x1 * c01) / det;
  let ar = det === 0 ? 0 : (c00 * x1 - c01 * x0) / det;
  const seg = dist(p0, p3);
  if (al < seg * 1e-6 || ar < seg * 1e-6) al = ar = seg / 3;
  return [p0, add(p0, mul(t1, al)), add(p3, mul(t2, ar)), p3];
}
function maxError(pts, c, u) {
  let max = 0, at = pts.length >> 1;
  u.forEach((t, i) => { const d = dist(bez(c, t), pts[i]); if (d > max) { max = d; at = i; } });
  return { max, at };
}
function reparam(pts, c, u) {
  return u.map((t, i) => {
    const d = sub(bez(c, t), pts[i]), d1 = bezD(c, t), d2 = bezDD(c, t);
    const den = dot(d1, d1) + dot(d, d2);
    return den === 0 ? t : Math.min(1, Math.max(0, t - dot(d, d1) / den));
  });
}
function fit(pts, t1, t2, err, out) {
  if (pts.length === 2) {
    const d = dist(pts[0], pts[1]) / 3;
    out.push([pts[0], add(pts[0], mul(t1, d)), add(pts[1], mul(t2, d)), pts[1]]);
    return;
  }
  let u = chordParams(pts);
  let c = generate(pts, u, t1, t2);
  let e = maxError(pts, c, u);
  if (e.max < err) { out.push(c); return; }
  if (e.max < err * 4) {
    for (let k = 0; k < 20; k++) {
      u = reparam(pts, c, u);
      c = generate(pts, u, t1, t2);
      e = maxError(pts, c, u);
      if (e.max < err) { out.push(c); return; }
    }
  }
  const s = Math.min(pts.length - 2, Math.max(1, e.at));
  const tc = norm(sub(pts[s - 1], pts[s + 1]));
  fit(pts.slice(0, s + 1), t1, tc, err, out);
  fit(pts.slice(s), mul(tc, -1), t2, err, out);
}
const endTangent = (run, fromStart, k = 4) =>
  fromStart ? norm(sub(run[Math.min(k, run.length - 1)], run[0])) : norm(sub(run[Math.max(0, run.length - 1 - k)], run[run.length - 1]));

function fitClosed(raw) {
  const pts = resample(raw);
  const n = pts.length;
  const { corners } = findCorners(pts);
  const cuts = corners.length ? [...corners].sort((a, b) => a - b) : [0, n >> 1];
  const smoothJoin = corners.length === 0;
  const curves = [];
  for (let ci = 0; ci < cuts.length; ci++) {
    const a = cuts[ci], b = cuts[(ci + 1) % cuts.length];
    const run = [];
    for (let i = a; ; i = (i + 1) % n) { run.push(pts[i]); if (i === b && run.length > 1) break; }
    const sm = smoothRun(run);
    let t1 = endTangent(sm, true), t2 = endTangent(sm, false);
    if (smoothJoin) {
      // Smooth (corner-free) loops: share one tangent across each join.
      const ta = norm(sub(pts[(a + 4) % n], pts[(a - 4 + n) % n]));
      const tb = norm(sub(pts[(b + 4) % n], pts[(b - 4 + n) % n]));
      t1 = ta; t2 = mul(tb, -1);
    }
    fit(sm, t1, t2, TOLERANCE, curves);
  }
  return { curves, corners: corners.map((i) => pts[i]) };
}

const traced = loops.map(fitClosed);
const ox = CROP.x, oy = CROP.y;
const all = traced.flatMap((t) => t.curves.flat());
const bbox = {
  minX: Math.min(...all.map((p) => p[0])) + ox, maxX: Math.max(...all.map((p) => p[0])) + ox,
  minY: Math.min(...all.map((p) => p[1])) + oy, maxY: Math.max(...all.map((p) => p[1])) + oy,
};
const outerPts = traced[0].curves.flatMap((c) => [c[0], c[3]]);
const tight = {
  minX: Math.min(...outerPts.map((p) => p[0])) + ox, maxX: Math.max(...outerPts.map((p) => p[0])) + ox,
  minY: Math.min(...outerPts.map((p) => p[1])) + oy, maxY: Math.max(...outerPts.map((p) => p[1])) + oy,
};

// Path data in reference pixel coordinates.
const fmt = (v) => (Math.round(v * 100) / 100).toString();
const toD = (curves) => {
  const P = (p) => `${fmt(p[0] + ox)} ${fmt(p[1] + oy)}`;
  return `M${P(curves[0][0])}${curves.map((c) => `C${P(c[1])} ${P(c[2])} ${P(c[3])}`).join('')}Z`;
};
// The outer contour comes out with the inside on the right; eyes are holes, so their
// winding is opposite. The master keeps them as separate shapes (body + white eyes).
const paths = { body: toD(traced[0].curves), eyes: [toD(traced[1].curves), toD(traced[2].curves)] };

const pad = 4;
const vb = [tight.minX - pad, tight.minY - pad, tight.maxX - tight.minX + 2 * pad, tight.maxY - tight.minY + 2 * pad].map((v) => Math.round(v));
const master = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb.join(' ')}">
  <!-- Incant wisp mascot, master trace in reference pixel units. Traced from
       handoffs/0004-wisp-mascot/reference.png by tools/trace-mascot.mjs (see brand/README.md). -->
  <path fill="${hex}" d="${paths.body}"/>
  <path fill="#FFFFFF" d="${paths.eyes.join('')}"/>
</svg>
`;
writeFileSync(join(repoRoot, 'editor/ui/brand/wisp-mascot.svg'), master);
writeFileSync(join(here, 'trace.json'), JSON.stringify({ color: hex, body, crop: CROP, bbox, tight, paths }, null, 1));
console.log(JSON.stringify({
  color: hex, body,
  contours: traced.map((t, i) => ({ i, curves: t.curves.length, corners: t.corners.map((p) => [fmt(p[0] + ox), fmt(p[1] + oy)]) })),
  tight, viewBox: vb,
}, null, 1));
