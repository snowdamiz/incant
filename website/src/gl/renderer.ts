/*
 * A small WebGL2 runner for the site's live scenes: one full-screen fragment shader per canvas.
 * It sizes the drawing buffer to the element, renders only while the canvas is on screen and
 * the tab is visible, lowers its internal resolution when frames run long, and renders a
 * single still frame when the visitor prefers reduced motion.
 */

export type UniformValue = number | readonly number[]

export interface GlViewOptions {
  fragment: string
  /** Re-render every animation frame while visible (false: render on demand only). */
  animate: boolean
  /** Upper bound on drawing-buffer pixels per CSS pixel. */
  maxScale?: number
  /** Lower bound the adaptive resolution may reach. */
  minScale?: number
  /** Called before each frame; return uniforms to upload. `time` is seconds of animated time. */
  frame: (time: number, dt: number, aspect: number) => Record<string, UniformValue>
}

export interface GlView {
  /** Request one frame (useful after a state change when not animating). */
  invalidate(): void
  destroy(): void
}

const VERTEX = `#version 300 es
in vec2 aPos;
void main() { gl_Position = vec4(aPos, 0.0, 1.0); }`

function compile(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type)
  if (!shader) throw new Error('Could not create shader')
  gl.shaderSource(shader, source)
  gl.compileShader(shader)
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS) && !gl.isContextLost()) {
    const log = gl.getShaderInfoLog(shader) ?? 'unknown error'
    gl.deleteShader(shader)
    throw new Error(`Shader compile failed: ${log}`)
  }
  return shader
}

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

function admittedContext(canvas: HTMLCanvasElement): WebGL2RenderingContext | null {
  try {
    return canvas.getContext('webgl2', {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
      powerPreference: 'default',
      // Raymarching is optional presentation. A software/slow context must use
      // the authored fallback before compiling or submitting expensive work.
      failIfMajorPerformanceCaveat: true,
    })
  } catch {
    return null
  }
}

export function createGlView(canvas: HTMLCanvasElement, options: GlViewOptions): GlView | null {
  const gl = admittedContext(canvas)
  if (!gl) return null

  let program: WebGLProgram | null = null
  let vertexBuffer: WebGLBuffer | null = null
  let locations = new Map<string, WebGLUniformLocation | null>()

  function build(): boolean {
    if (!gl) return false
    let vs: WebGLShader | null = null
    let fs: WebGLShader | null = null
    let candidate: WebGLProgram | null = null
    let buffer: WebGLBuffer | null = null
    try {
      vs = compile(gl, gl.VERTEX_SHADER, VERTEX)
      fs = compile(gl, gl.FRAGMENT_SHADER, options.fragment)
      const p = candidate = gl.createProgram()
      if (!p) throw new Error('Could not create program')
      gl.attachShader(p, vs)
      gl.attachShader(p, fs)
      gl.bindAttribLocation(p, 0, 'aPos')
      gl.linkProgram(p)
      if (!gl.getProgramParameter(p, gl.LINK_STATUS) && !gl.isContextLost()) {
        throw new Error(gl.getProgramInfoLog(p) ?? 'Could not link program')
      }
      buffer = gl.createBuffer()
      if (!buffer) throw new Error('Could not create vertex buffer')
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW)
      gl.enableVertexAttribArray(0)
      gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0)
      gl.useProgram(p)
      if (program) gl.deleteProgram(program)
      if (vertexBuffer) gl.deleteBuffer(vertexBuffer)
      program = p
      vertexBuffer = buffer
      candidate = null
      buffer = null
      locations = new Map()
      return true
    } catch (error) {
      console.warn(error)
      return false
    } finally {
      if (vs) gl.deleteShader(vs)
      if (fs) gl.deleteShader(fs)
      if (candidate) gl.deleteProgram(candidate)
      if (buffer) gl.deleteBuffer(buffer)
    }
  }

  function releaseContext(): void {
    // Releasing an optional context must not prevent the authored fallback.
    try { gl?.getExtension('WEBGL_lose_context')?.loseContext() } catch { /* unavailable */ }
  }

  if (!build()) {
    releaseContext()
    return null
  }

  const maxScale = options.maxScale ?? 1.5
  const minScale = options.minScale ?? 0.35
  let scale = Math.min(window.devicePixelRatio || 1, maxScale) * 0.75
  let visible = false
  let lost = false
  let raf = 0
  let lastNow = 0
  let time = 0
  let slowFrames = 0
  let fastFrames = 0
  let pending = true
  let destroyed = false
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)')

  function resize(): void {
    const rect = canvas.getBoundingClientRect()
    const w = Math.max(1, Math.round(rect.width * scale))
    const h = Math.max(1, Math.round(rect.height * scale))
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w
      canvas.height = h
    }
  }

  function location(name: string): WebGLUniformLocation | null {
    if (!locations.has(name)) locations.set(name, gl!.getUniformLocation(program!, name))
    return locations.get(name) ?? null
  }

  function draw(dt: number): void {
    if (!gl || !program || lost) return
    resize()
    gl.viewport(0, 0, canvas.width, canvas.height)
    const uniforms = options.frame(time, dt, canvas.width / canvas.height)
    uniforms.uRes = [canvas.width, canvas.height]
    for (const [name, value] of Object.entries(uniforms)) {
      const loc = location(name)
      if (!loc) continue
      if (typeof value === 'number') gl.uniform1f(loc, value)
      else if (value.length === 2) gl.uniform2f(loc, value[0]!, value[1]!)
      else if (value.length === 3) gl.uniform3f(loc, value[0]!, value[1]!, value[2]!)
      else if (value.length === 4) gl.uniform4f(loc, value[0]!, value[1]!, value[2]!, value[3]!)
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3)
  }

  function adapt(frameMs: number): void {
    // Hold roughly 50 fps: step the internal resolution down quickly and back up slowly.
    if (frameMs > 24) {
      fastFrames = 0
      if (++slowFrames >= 8 && scale > minScale) {
        scale = Math.max(minScale, scale * 0.85)
        slowFrames = 0
      }
    } else if (frameMs < 13) {
      slowFrames = 0
      const ceiling = Math.min(window.devicePixelRatio || 1, maxScale)
      if (++fastFrames >= 90 && scale < ceiling) {
        scale = Math.min(ceiling, scale * 1.1)
        fastFrames = 0
      }
    }
  }

  function loop(now: number): void {
    raf = 0
    if (destroyed || !visible || lost) return
    if (document.hidden) {
      // Hidden tabs get at most the one requested still frame.
      if (pending) {
        pending = false
        draw(0)
      }
      return
    }
    const dt = lastNow ? Math.min((now - lastNow) / 1000, 0.1) : 0
    if (lastNow) adapt(now - lastNow)
    lastNow = now
    const animating = options.animate && !reduced.matches
    if (animating) time += dt
    if (animating || pending) {
      pending = false
      draw(dt)
    }
    if (animating) raf = requestAnimationFrame(loop)
  }

  function kick(): void {
    if (!destroyed && !raf && visible && !lost && (pending || !document.hidden)) {
      lastNow = 0
      raf = requestAnimationFrame(loop)
    }
  }

  const io = new IntersectionObserver(
    (entries) => {
      visible = entries.some((e) => e.isIntersecting)
      if (visible) {
        pending = true
        kick()
      }
    },
    { rootMargin: '120px 0px' },
  )
  io.observe(canvas)

  const ro = new ResizeObserver(() => {
    pending = true
    kick()
  })
  ro.observe(canvas)

  const onVisibility = (): void => kick()
  document.addEventListener('visibilitychange', onVisibility)
  const onMotion = (): void => {
    pending = true
    kick()
  }
  reduced.addEventListener('change', onMotion)

  const onLost = (event: Event): void => {
    event.preventDefault()
    lost = true
    if (raf) cancelAnimationFrame(raf)
    raf = 0
  }
  const onRestored = (): void => {
    lost = false
    if (build()) {
      pending = true
      kick()
    }
  }
  canvas.addEventListener('webglcontextlost', onLost)
  canvas.addEventListener('webglcontextrestored', onRestored)

  return {
    invalidate() {
      if (destroyed) return
      pending = true
      kick()
    },
    destroy() {
      if (destroyed) return
      destroyed = true
      if (raf) cancelAnimationFrame(raf)
      io.disconnect()
      ro.disconnect()
      document.removeEventListener('visibilitychange', onVisibility)
      reduced.removeEventListener('change', onMotion)
      canvas.removeEventListener('webglcontextlost', onLost)
      canvas.removeEventListener('webglcontextrestored', onRestored)
      if (program) gl.deleteProgram(program)
      if (vertexBuffer) gl.deleteBuffer(vertexBuffer)
      program = null
      vertexBuffer = null
      releaseContext()
    },
  }
}
