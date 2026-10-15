import assert from 'node:assert/strict'
import { test, type TestContext } from 'node:test'
import { createGlView } from '../src/gl/renderer.ts'

type Failure = 'refused' | 'exception' | 'fragment' | 'link' | 'buffer'

function fixture(t: TestContext, failure?: Failure, reducedMotion = false, renderer: string | null = 'Apple GPU') {
  const requested: WebGLContextAttributes[] = []
  const deletedShaders = new Set<object>()
  const deletedPrograms = new Set<object>()
  const deletedBuffers = new Set<object>()
  const canvasEvents = new Map<string, (event: Event) => void>()
  const documentEvents = new Map<string, () => void>()
  const mediaEvents = new Map<string, () => void>()
  const frames = new Map<number, FrameRequestCallback>()
  const observers: { disconnected: boolean; visible: (value: boolean) => void }[] = []
  let frameId = 0
  let draws = 0
  let releases = 0
  let createdShaders = 0
  let fallbacks = 0
  const document = {
    hidden: false,
    addEventListener: (name: string, callback: () => void) => documentEvents.set(name, callback),
    removeEventListener: (name: string) => documentEvents.delete(name),
  }
  const media = {
    matches: reducedMotion,
    addEventListener: (name: string, callback: () => void) => mediaEvents.set(name, callback),
    removeEventListener: (name: string) => mediaEvents.delete(name),
  }
  const gl = {
    VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, COMPILE_STATUS: 3, LINK_STATUS: 4,
    ARRAY_BUFFER: 5, STATIC_DRAW: 6, FLOAT: 7, TRIANGLES: 8, RENDERER: 9,
    createShader: (type: number) => { createdShaders++; return { type } },
    shaderSource() {}, compileShader() {},
    getShaderParameter: (shader: { type: number }) => !(failure === 'fragment' && shader.type === 2),
    isContextLost: () => false,
    getShaderInfoLog: () => 'fixture compile refusal',
    deleteShader: (shader: object) => {
      assert(!deletedShaders.has(shader), 'shader disposed twice')
      deletedShaders.add(shader)
    },
    createProgram: () => ({}), attachShader() {}, bindAttribLocation() {}, linkProgram() {},
    getProgramParameter: () => failure !== 'link',
    getProgramInfoLog: () => 'fixture link refusal',
    deleteProgram: (program: object) => deletedPrograms.add(program),
    createBuffer: () => failure === 'buffer' ? null : ({}),
    bindBuffer() {}, bufferData() {}, enableVertexAttribArray() {}, vertexAttribPointer() {}, useProgram() {},
    deleteBuffer: (buffer: object) => deletedBuffers.add(buffer),
    viewport() {}, getUniformLocation: () => ({}), uniform1f() {}, uniform2f() {}, uniform3f() {}, uniform4f() {},
    drawArrays: () => { draws++ },
    getParameter: () => renderer,
    getExtension: (name: string) => name === 'WEBGL_lose_context'
      ? { loseContext: () => { releases++ } }
      : renderer === null ? null : { UNMASKED_RENDERER_WEBGL: 10 },
  }
  const canvas = {
    width: 300, height: 150,
    getContext: (id: string, attributes: WebGLContextAttributes) => {
      assert.equal(id, 'webgl2')
      requested.push(attributes)
      if (failure === 'exception') throw new Error('context unavailable')
      return failure === 'refused' ? null : gl
    },
    getBoundingClientRect: () => ({ width: 800, height: 500 }),
    addEventListener: (name: string, callback: (event: Event) => void) => canvasEvents.set(name, callback),
    removeEventListener: (name: string) => canvasEvents.delete(name),
  }
  const globals: Record<string, unknown> = {
    window: { devicePixelRatio: 1, matchMedia: () => media },
    document,
    requestAnimationFrame: (callback: FrameRequestCallback) => { frames.set(++frameId, callback); return frameId },
    cancelAnimationFrame: (id: number) => frames.delete(id),
    IntersectionObserver: class {
      constructor(callback: IntersectionObserverCallback) {
        observers.push({ disconnected: false, visible: (value) => callback(
          [{ isIntersecting: value } as IntersectionObserverEntry], this as unknown as IntersectionObserver,
        ) })
      }
      observe() {}
      disconnect() { observers[0]!.disconnected = true }
    },
    ResizeObserver: class { observe() {} disconnect() {} },
  }
  for (const [key, value] of Object.entries(globals)) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, key)
    Object.defineProperty(globalThis, key, { configurable: true, value })
    t.after(() => {
      if (previous) Object.defineProperty(globalThis, key, previous)
      else Reflect.deleteProperty(globalThis, key)
    })
  }
  t.mock.method(console, 'warn', () => {})
  const uniforms: { time: number; dt: number; aspect: number }[] = []
  function create(animate = true) {
    return createGlView(canvas as unknown as HTMLCanvasElement, {
      fragment: 'fixture fragment', animate,
      frame: (time, dt, aspect) => { uniforms.push({ time, dt, aspect }); return {} },
      onFallback: () => { fallbacks++ },
    })
  }
  function flush(now: number) {
    const callbacks = [...frames.values()]
    frames.clear()
    for (const callback of callbacks) callback(now)
  }
  return {
    create, flush, canvas, observers, frames, requested, deletedShaders, deletedPrograms, deletedBuffers,
    canvasEvents, documentEvents, mediaEvents, uniforms, document,
    counts: () => ({ draws, releases, createdShaders, fallbacks }),
  }
}

for (const renderer of ['ANGLE (Google, Vulkan SwiftShader Device)', 'llvmpipe (LLVM 19)', 'softpipe', 'Microsoft Basic Render Driver']) {
  test(`known software renderer ${renderer} is refused before expensive compilation`, (t) => {
    const f = fixture(t, undefined, false, renderer)
    assert.equal(f.create(), null)
    assert.equal(f.counts().createdShaders, 0)
    assert.equal(f.counts().releases, 1)
    assert.equal(f.frames.size, 0)
  })
}

test('a stalled visible frame retires optional animation once even with private renderer identity', (t) => {
  const f = fixture(t, undefined, false, null)
  const view = f.create()!
  assert(view)
  f.observers[0]!.visible(true)
  f.flush(100)
  f.flush(351)
  assert.equal(f.counts().draws, 1)
  assert.equal(f.counts().fallbacks, 1)
  assert.equal(f.counts().releases, 1)
  view.invalidate()
  f.observers[0]!.visible(true)
  f.flush(600)
  view.destroy()
  assert.equal(f.counts().fallbacks, 1)
  assert.equal(f.frames.size, 0)
})

test('offscreen and hidden intervals do not count as slow visible rendering', (t) => {
  const f = fixture(t)
  const view = f.create()!
  f.observers[0]!.visible(true)
  f.flush(100)
  f.observers[0]!.visible(false)
  f.flush(200)
  f.observers[0]!.visible(true)
  f.flush(2000)
  f.document.hidden = true
  f.documentEvents.get('visibilitychange')!()
  f.flush(2100)
  f.document.hidden = false
  f.documentEvents.get('visibilitychange')!()
  f.flush(4000)
  assert.equal(f.counts().fallbacks, 0)
  assert.equal(f.counts().draws, 3)
  view.destroy()
})

for (const failure of ['refused', 'exception'] as const) {
  test(`major-performance ${failure} reaches fallback before compilation or scheduling`, (t) => {
    const f = fixture(t, failure)
    assert.equal(f.create(), null)
    assert.equal(f.requested.length, 1)
    assert.equal(f.requested[0]!.failIfMajorPerformanceCaveat, true)
    assert.equal(f.counts().createdShaders, 0)
    assert.equal(f.counts().draws, 0)
    assert.equal(f.observers.length, 0)
    assert.equal(f.frames.size, 0)
  })
}

for (const failure of ['fragment', 'link', 'buffer'] as const) {
  test(`${failure} preparation refusal releases partial resources before fallback`, (t) => {
    const f = fixture(t, failure)
    assert.equal(f.create(), null)
    assert.equal(f.deletedShaders.size, 2)
    assert.equal(f.deletedPrograms.size, failure === 'fragment' ? 0 : 1)
    assert.equal(f.counts().releases, 1)
    assert.equal(f.observers.length, 0)
    assert.equal(f.frames.size, 0)
  })
}

test('admitted renderer preserves visible animation and stops after disposal', (t) => {
  const f = fixture(t)
  const view = f.create()!
  assert(view)
  assert.equal(f.frames.size, 0)
  f.observers[0]!.visible(true)
  f.flush(100)
  f.flush(116)
  assert.equal(f.counts().draws, 2)
  assert.equal(f.canvas.width, 600)
  assert.equal(f.canvas.height, 375)
  assert.equal(f.uniforms[1]!.aspect, 1.6)
  assert.equal(f.uniforms[1]!.dt, 0.016)
  assert.equal(f.frames.size, 1)
  view.destroy()
  view.destroy()
  view.invalidate()
  // A callback queued before disconnect must not revive the disposed view.
  f.observers[0]!.visible(true)
  f.flush(132)
  assert.equal(f.counts().draws, 2)
  assert.equal(f.frames.size, 0)
  assert.equal(f.deletedPrograms.size, 1)
  assert.equal(f.deletedBuffers.size, 1)
  assert.equal(f.counts().releases, 1)
  assert(f.observers[0]!.disconnected)
  assert.equal(f.canvasEvents.size + f.documentEvents.size + f.mediaEvents.size, 0)
})

test('reduced motion draws only the requested still and later invalidation', (t) => {
  const f = fixture(t, undefined, true)
  const view = f.create()!
  f.observers[0]!.visible(true)
  f.flush(100)
  assert.equal(f.counts().draws, 1)
  assert.equal(f.frames.size, 0)
  view.invalidate()
  f.flush(116)
  assert.equal(f.counts().draws, 2)
  assert(f.uniforms.every((frame) => frame.time === 0))
  view.destroy()
})
