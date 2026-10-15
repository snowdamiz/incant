/*
 * Camera presets for the coast scene, shared by the shader uniforms and by
 * the editor overlay, which projects world points onto the viewport.
 */
export type Vec3 = readonly [number, number, number]

export interface Camera {
  pos: Vec3
  target: Vec3
  /** Vertical field of view in radians. */
  fov: number
}

/** A scene camera with an optional framing for tall (portrait) viewports. */
export interface FramedCamera extends Camera {
  portrait?: Camera
}

export const LIGHTHOUSE_BASE: Vec3 = [14.5, 3.05, 64]
export const LAMP: Vec3 = [14.5, 3.05 + 9 + 0.86, 64]

export const heroCamera: FramedCamera = {
  pos: [0, 2.1, 0],
  target: [0.2, 6.3, 64],
  fov: 0.62,
  // Tilted down so the tower and beam sit in the sky above the text, which rests on the darker sea.
  portrait: { pos: [0, 2.1, 0], target: [10, -6.5, 64], fov: 0.84 },
}

export const editorCamera: FramedCamera = {
  pos: [-3.5, 7.2, 36],
  target: [13.2, 7.6, 64],
  fov: 0.66,
  portrait: { pos: [-9, 8.5, 18], target: [13.6, 6.6, 64], fov: 0.74 },
}

export const closingCamera: FramedCamera = {
  pos: [-4, 1.5, 22],
  target: [1.5, 7.2, 64],
  fov: 0.72,
  portrait: { pos: [-2, 1.6, 4], target: [10.5, -8.9, 64], fov: 0.9 },
}

/**
 * Keeps a wide composition readable on narrow screens: between 16:10 and roughly 1:2 the
 * camera blends from its landscape framing to its portrait framing.
 */
export function fitToAspect(camera: FramedCamera, aspect: number): Camera {
  const p = camera.portrait
  if (!p) return camera
  const t = Math.min(Math.max((1.6 - aspect) / 1.1, 0), 1)
  const mix = (a: Vec3, b: Vec3): Vec3 => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
  return { pos: mix(camera.pos, p.pos), target: mix(camera.target, p.target), fov: camera.fov + (p.fov - camera.fov) * t }
}

function sub(a: Vec3, b: Vec3): Vec3 {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
function dot(a: Vec3, b: Vec3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
function normalize(a: Vec3): Vec3 {
  const l = Math.hypot(a[0], a[1], a[2]) || 1
  return [a[0] / l, a[1] / l, a[2] / l]
}

/**
 * Projects a world point to fractions of the viewport (0..1 from the left and from the top),
 * matching the shader's camera: uv = (frag - 0.5 * res) / res.y, focal length 0.5 / tan(fov / 2).
 */
export function project(camera: Camera, point: Vec3, aspect: number): { x: number; y: number } {
  const fw = normalize(sub(camera.target, camera.pos))
  const rt = normalize(cross([0, 1, 0], fw))
  const up = cross(fw, rt)
  const d = sub(point, camera.pos)
  const z = dot(d, fw)
  const f = 0.5 / Math.tan(camera.fov / 2)
  const u = (dot(d, rt) / z) * f
  const v = (dot(d, up) / z) * f
  return { x: 0.5 + u / aspect, y: 0.5 - v }
}

export function cameraUniforms(camera: Camera): Record<string, readonly number[] | number> {
  return { uCamPos: camera.pos, uCamTarget: camera.target, uFov: camera.fov }
}
