<script setup lang="ts">
import GlCanvas from './GlCanvas.vue'
import ArrowIcon from './ArrowIcon.vue'
import { coastFragment } from '../gl/coast.glsl'
import { cameraUniforms, fitToAspect, heroCamera } from '../gl/cameras'
import { links } from '../content'

// The beam turns once every 14 s; a reduced-motion still starts with the beam crossing the bay.
const BEAM_PERIOD = 14
const START_ANGLE = 2.15

function frame(time: number, _dt: number, aspect: number) {
  const sway = Math.sin(time * 0.21) * 0.06
  const camera = fitToAspect(heroCamera, aspect)
  return {
    ...cameraUniforms({ ...camera, pos: [camera.pos[0], camera.pos[1] + sway, camera.pos[2]] }),
    uTime: time,
    uBeamAngle: START_ANGLE + (time / BEAM_PERIOD) * Math.PI * 2,
    uBeam: 1,
    uPreview: 0,
    uLamp: 1,
    uGrid: 0,
    uSelect: 0,
    uGrain: 0.035,
  }
}
</script>

<template>
  <section id="top" aria-labelledby="hero-title" class="relative isolate overflow-hidden bg-night">
    <GlCanvas
      class="absolute inset-0 -z-10"
      :fragment="coastFragment"
      :frame="frame"
      :max-scale="1.25"
      label="A lighthouse on a rocky headland at blue hour. Its beam sweeps across the sea under the first stars. Rendered live in your browser."
    >
      <template #fallback>
        <div class="size-full bg-[radial-gradient(120%_70%_at_30%_62%,#3a2335_0%,#121327_38%,#060709_75%)]" />
      </template>
    </GlCanvas>
    <div
      class="pointer-events-none absolute inset-0 -z-10 bg-[linear-gradient(to_top,var(--color-night)_0%,rgb(6_7_9/0.72)_22%,rgb(6_7_9/0)_55%),linear-gradient(to_right,rgb(6_7_9/0.55)_0%,rgb(6_7_9/0)_55%)]"
      aria-hidden="true"
    />

    <div class="mx-auto flex min-h-[max(40rem,100svh)] max-w-[90rem] flex-col justify-end px-5 pt-36 pb-10 sm:px-8 lg:px-12">
      <p class="eyebrow text-fg-2">Modeling tool · Game engine · Editor</p>
      <h1
        id="hero-title"
        class="display text-balance-safe mt-6 max-w-[14ch] text-[3.1rem] leading-[0.94] min-[400px]:text-[3.5rem] sm:text-[5.4rem] lg:text-[7rem] xl:text-[8rem]"
      >
        The game engine you can talk to.
      </h1>

      <div class="mt-7 grid gap-7 sm:mt-9 sm:gap-8 lg:grid-cols-12 lg:items-end">
        <p class="text-pretty-safe max-w-xl text-[16px] leading-relaxed text-fg-2 sm:text-lg lg:col-span-6">
          Model, script and ship 2D and 3D games in one application. Work by hand, in TypeScript, or by asking the
          agent. Every change lands as a command you can read, test and undo.
        </p>
        <div class="flex flex-wrap items-center gap-3 lg:col-span-6 lg:justify-end">
          <a
            :href="links.repository"
            class="inline-flex h-12 items-center gap-2.5 rounded-full bg-fg px-5 sm:px-6 text-[15px] font-semibold text-night transition-colors hover:bg-white"
          >
            View on GitHub
            <ArrowIcon class="size-3.5" />
          </a>
          <a
            :href="links.productGuide"
            class="inline-flex h-12 items-center gap-1 rounded-full border border-line-strong px-5 sm:px-6 text-[15px] font-medium text-fg backdrop-blur-sm transition-colors hover:border-fg-2 hover:bg-white/5"
          >
            <span class="min-[440px]:hidden">Product plan</span>
            <span class="hidden min-[440px]:inline">Read the product plan</span>
            <span class="sr-only">(on GitHub)</span>
          </a>
        </div>
      </div>

      <div class="mt-10 flex flex-wrap items-center justify-between gap-x-8 gap-y-3 border-t border-line pt-5 text-muted sm:mt-14">
        <p class="eyebrow">Windows · macOS · Linux · iOS · Android · Web</p>
        <p class="eyebrow flex items-center gap-2">
          <span class="size-1.5 rounded-full bg-lamp motion-safe:animate-pulse-soft" aria-hidden="true" />
          Live scene, rendered in your browser
        </p>
      </div>
    </div>
  </section>
</template>
