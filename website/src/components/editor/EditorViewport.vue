<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import GlCanvas from '../GlCanvas.vue'
import { coastFragment } from '../../gl/coast.glsl'
import { LAMP, LIGHTHOUSE_BASE, cameraUniforms, editorCamera, fitToAspect, project } from '../../gl/cameras'
import type { StageStep } from '../../content'

const props = defineProps<{ step: StageStep }>()

/* Scene state per step; values ease toward their targets in the render loop. */
const targets: Record<StageStep, { beam: number; preview: number; lamp: number; select: number; spin: number }> = {
  ask: { beam: 0, preview: 0, lamp: 0.45, select: 1, spin: 0 },
  review: { beam: 0, preview: 1, lamp: 0.6, select: 0, spin: 0.5 },
  play: { beam: 1, preview: 0, lamp: 1, select: 0, spin: 1 },
  undo: { beam: 0, preview: 0, lamp: 0.45, select: 1, spin: 0 },
}
const state = { beam: 0, preview: 0, lamp: 0.45, select: 1, spin: 0, angle: 2.6 }
const BEAM_PERIOD = 8

const gl = ref<InstanceType<typeof GlCanvas> | null>(null)
watch(
  () => props.step,
  () => gl.value?.invalidate(),
)

function frame(time: number, dt: number, aspect: number) {
  const t = targets[props.step]
  const k = dt === 0 ? 1 : 1 - Math.exp(-dt * 5)
  state.beam += (t.beam - state.beam) * k
  state.preview += (t.preview - state.preview) * k
  state.lamp += (t.lamp - state.lamp) * k
  state.select += (t.select - state.select) * k
  state.spin += (t.spin - state.spin) * k
  state.angle += ((dt * Math.PI * 2) / BEAM_PERIOD) * state.spin
  return {
    ...cameraUniforms(fitToAspect(editorCamera, aspect)),
    uTime: time,
    uBeamAngle: state.angle,
    uBeam: state.beam,
    uPreview: state.preview,
    uLamp: state.lamp,
    uGrid: props.step === 'play' ? 0.0 : 1,
    uSelect: state.select,
    uGrain: 0.015,
  }
}

/* Overlay anchors follow the viewport's aspect ratio. */
const box = ref<HTMLElement | null>(null)
const aspect = ref(16 / 9)
let ro: ResizeObserver | null = null
onMounted(() => {
  if (!box.value) return
  ro = new ResizeObserver(([entry]) => {
    if (entry && entry.contentRect.height > 0) aspect.value = entry.contentRect.width / entry.contentRect.height
  })
  ro.observe(box.value)
})
onBeforeUnmount(() => ro?.disconnect())

const camera = computed(() => fitToAspect(editorCamera, aspect.value))
const lamp = computed(() => project(camera.value, LAMP, aspect.value))
const base = computed(() => project(camera.value, LIGHTHOUSE_BASE, aspect.value))
const pct = (v: number) => `${(v * 100).toFixed(2)}%`
</script>

<template>
  <div ref="box" class="relative size-full overflow-hidden bg-ed-well">
    <GlCanvas
      ref="gl"
      class="absolute inset-0"
      :fragment="coastFragment"
      :frame="frame"
      :max-scale="1.25"
    >
      <template #fallback>
        <div class="size-full bg-[radial-gradient(90%_70%_at_60%_55%,#2a2140_0%,#0e0f12_70%)]" />
      </template>
    </GlCanvas>

    <!-- Selection: transform gizmo on the lantern and its name tag. -->
    <div
      v-if="step === 'ask' || step === 'undo'"
      class="pointer-events-none absolute"
      :style="{ left: pct(lamp.x), top: pct(lamp.y) }"
    >
      <svg viewBox="-60 -60 120 120" class="absolute -top-[60px] -left-[60px] size-[120px] overflow-visible" aria-hidden="true">
        <g stroke-width="2" stroke-linecap="round">
          <line x1="0" y1="0" x2="46" y2="6" stroke="#ff8a7f" />
          <path d="M46 6l-8-6 1 11z" fill="#ff8a7f" stroke="none" transform="translate(4 0)" />
          <line x1="0" y1="0" x2="0" y2="-48" stroke="#7fe0a3" />
          <path d="M0-52l-5 10h10z" fill="#7fe0a3" stroke="none" />
          <line x1="0" y1="0" x2="-30" y2="22" stroke="#85b4ff" />
          <path d="M-34 25l5-11 6 8z" fill="#85b4ff" stroke="none" />
        </g>
        <circle r="4" fill="#ececf1" />
      </svg>
      <span class="absolute top-3 left-7 rounded-[5px] bg-ed-app/85 px-1.5 py-0.5 text-[11px] whitespace-nowrap text-ed-text backdrop-blur">
        Lantern
      </span>
    </div>

    <div
      v-if="step === 'review'"
      class="pointer-events-none absolute"
      :style="{ left: pct(base.x), top: pct(lamp.y) }"
    >
      <span class="absolute -top-9 left-6 rounded-[5px] border border-ed-agent/40 bg-ed-app/85 px-2 py-0.5 text-[11px] whitespace-nowrap text-ed-agent backdrop-blur">
        Beam · proposed
      </span>
    </div>

    <div class="pointer-events-none absolute top-3 left-3 flex gap-2 text-[11px]">
      <span class="rounded-[5px] bg-ed-app/80 px-2 py-1 text-ed-muted backdrop-blur">Perspective</span>
      <span v-if="step === 'review'" class="rounded-[5px] bg-ed-app/80 px-2 py-1 text-ed-agent backdrop-blur">Preview · 4 pending edits</span>
      <span v-if="step === 'play'" class="inline-flex items-center gap-1.5 rounded-[5px] bg-ed-app/80 px-2 py-1 text-ed-text backdrop-blur">
        <span class="size-1.5 rounded-full bg-ed-danger motion-safe:animate-pulse-soft" />Play-test · headless
      </span>
      <span v-if="step === 'undo'" class="rounded-[5px] bg-ed-app/80 px-2 py-1 text-ed-warning backdrop-blur">Undone · revision 41</span>
    </div>
  </div>
</template>
