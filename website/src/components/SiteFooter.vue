<script setup lang="ts">
import GlCanvas from './GlCanvas.vue'
import ArrowIcon from './ArrowIcon.vue'
import WispMark from './WispMark.vue'
import { coastFragment } from '../gl/coast.glsl'
import { cameraUniforms, closingCamera, fitToAspect } from '../gl/cameras'
import { links, navItems } from '../content'

const year = new Date().getFullYear()

function frame(time: number, _dt: number, aspect: number) {
  return {
    ...cameraUniforms(fitToAspect(closingCamera, aspect)),
    uTime: time + 40,
    uBeamAngle: 0.9 + (time / 14) * Math.PI * 2,
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
  <section aria-labelledby="closing-title" class="relative isolate overflow-hidden border-t border-line">
    <GlCanvas class="absolute inset-0 -z-10" :fragment="coastFragment" :frame="frame" :max-scale="1.1">
      <template #fallback>
        <div class="size-full bg-[radial-gradient(100%_80%_at_70%_40%,#2b2240_0%,#0d0e18_55%,#060709_85%)]" />
      </template>
    </GlCanvas>
    <div
      class="pointer-events-none absolute inset-0 -z-10 bg-[linear-gradient(to_bottom,var(--color-night)_0%,rgb(6_7_9/0)_30%,rgb(6_7_9/0)_60%,var(--color-night)_100%),linear-gradient(to_right,rgb(6_7_9/0.7),rgb(6_7_9/0)_60%)]"
      aria-hidden="true"
    />
    <div
      class="pointer-events-none absolute inset-0 -z-10 bg-[linear-gradient(to_bottom,rgb(6_7_9/0)_38%,rgb(6_7_9/0.82)_58%,var(--color-night)_100%)] sm:hidden"
      aria-hidden="true"
    />
    <div class="mx-auto flex min-h-[46rem] max-w-[90rem] flex-col justify-end px-5 pt-32 pb-20 sm:min-h-[44rem] sm:justify-center sm:px-8 sm:py-32 lg:min-h-[52rem] lg:px-12">
      <h2 id="closing-title" class="display text-balance-safe max-w-[12ch] text-[3.25rem] leading-[0.95] sm:text-[5.5rem] lg:text-[7rem]">
        Start with a sentence.
      </h2>
      <p class="text-pretty-safe mt-8 max-w-md text-[17px] leading-relaxed text-fg-2 sm:text-lg">
        Everything about how Incant works, from the scene format to the agent, is in the product plan.
      </p>
      <div class="mt-10 flex flex-wrap items-center gap-3">
        <a
          :href="links.repository"
          class="inline-flex h-12 items-center gap-2.5 rounded-full bg-fg px-6 text-[15px] font-semibold text-night transition-colors hover:bg-white"
        >
          View on GitHub
          <ArrowIcon class="size-3.5" />
        </a>
        <a
          :href="links.productGuide"
          class="inline-flex h-12 items-center gap-2.5 rounded-full border border-line-strong px-6 text-[15px] font-medium text-fg backdrop-blur-sm transition-colors hover:border-fg-2 hover:bg-white/5"
        >
          Read the product plan
          <span class="sr-only">(on GitHub)</span>
        </a>
      </div>
    </div>
  </section>

  <footer class="border-t border-line bg-night">
    <div class="mx-auto max-w-[90rem] px-5 pt-16 sm:px-8 lg:px-12">
      <div class="grid gap-12 md:grid-cols-12">
        <div class="md:col-span-5">
          <a href="#top" class="inline-flex items-center gap-2.5">
            <WispMark class="h-6 w-auto" body="#6b52ff" />
            <span class="text-[19px] font-semibold tracking-[-0.03em]">Incant</span>
          </a>
          <p class="mt-4 max-w-xs text-[14.5px] leading-relaxed text-muted">A modeling tool, game engine and editor you can talk to.</p>
        </div>
        <nav aria-label="Footer" class="grid grid-cols-2 gap-8 md:col-span-7 md:grid-cols-3">
          <div>
            <h2 class="eyebrow text-muted">Product</h2>
            <ul class="mt-5 space-y-3 text-[14.5px]">
              <li v-for="item in navItems" :key="item.href"><a :href="item.href" class="text-fg-2 transition-colors hover:text-fg">{{ item.label }}</a></li>
            </ul>
          </div>
          <div>
            <h2 class="eyebrow text-muted">Resources</h2>
            <ul class="mt-5 space-y-3 text-[14.5px]">
              <li><a :href="links.productGuide" class="text-fg-2 transition-colors hover:text-fg">Product plan</a></li>
              <li><a :href="links.repository" class="text-fg-2 transition-colors hover:text-fg">GitHub repository</a></li>
            </ul>
          </div>
          <div class="col-span-2 md:col-span-1">
            <h2 class="eyebrow text-muted">Credits</h2>
            <ul class="mt-5 space-y-3 text-[14.5px]">
              <li><a href="https://fontawesome.com/license/free" class="text-fg-2 transition-colors hover:text-fg">Font Awesome, CC BY 4.0</a></li>
              <li><a href="https://creativecommons.org/licenses/by/3.0/" class="text-fg-2 transition-colors hover:text-fg">Android robot, CC BY 3.0</a></li>
            </ul>
          </div>
        </nav>
      </div>

      <p class="mt-14 text-[12.5px] leading-relaxed text-muted">
        Platform marks from Font Awesome Free; the Android robot is reproduced from work created and shared by Google. Platform
        names and marks belong to their owners. Interactive scenes are rendered live in your browser.
      </p>

      <div class="mt-10 flex flex-wrap items-center justify-between gap-4 border-t border-line py-7 text-[13px] text-muted">
        <span>&copy; {{ year }} Incant</span>
        <a href="#top" class="transition-colors hover:text-fg">Back to top ↑</a>
      </div>
    </div>
    <!-- The oversized wordmark is generated content: decoration only, outside the accessibility tree. -->
    <div
      class="pointer-events-none overflow-hidden text-center text-[22vw] leading-[0.74] font-semibold tracking-[-0.06em] text-white/[0.035] select-none after:content-['Incant']"
      aria-hidden="true"
    />
  </footer>
</template>
