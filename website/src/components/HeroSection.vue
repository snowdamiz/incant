<script setup lang="ts">
import EditorIllustration from './EditorIllustration.vue'
import WispMark from './WispMark.vue'
import { links } from '../content'
</script>

<template>
  <section id="top" aria-labelledby="hero-title" class="relative isolate overflow-hidden pt-28 pb-16 sm:pt-40 sm:pb-24">
    <!-- Atmosphere -->
    <div aria-hidden="true" class="pointer-events-none absolute inset-0 -z-10">
      <div class="grain absolute inset-0 [mask-image:radial-gradient(ellipse_at_top,black_20%,transparent_70%)]" />
      <div class="absolute top-[-18rem] left-1/2 h-[46rem] w-[70rem] -translate-x-1/2 rounded-full bg-[radial-gradient(closest-side,rgba(98,70,229,0.42),transparent)]" />
      <div class="absolute top-[30rem] left-[8%] h-[24rem] w-[30rem] rounded-full bg-[radial-gradient(closest-side,rgba(255,184,107,0.14),transparent)]" />
      <div class="absolute right-[4%] top-[34rem] h-[22rem] w-[28rem] rounded-full bg-[radial-gradient(closest-side,rgba(94,230,196,0.10),transparent)]" />
    </div>

    <div class="mx-auto max-w-7xl px-5 sm:px-8">
      <div class="mx-auto max-w-4xl text-center">
        <p class="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.04] py-1.5 pr-4 pl-1.5 text-[13px] font-medium text-ink-200 max-[379px]:pr-1.5">
          <span class="rounded-full bg-wisp-500 px-2.5 py-0.5 text-[11px] font-semibold tracking-wide whitespace-nowrap text-white">Engine + Editor + Agent</span>
          <span class="hidden whitespace-nowrap min-[380px]:inline">One scene, built together</span>
        </p>

        <h1
          id="hero-title"
          class="text-balance-safe mt-7 text-[2.6rem] leading-[1.02] font-semibold tracking-[-0.045em] text-white min-[400px]:text-5xl sm:text-6xl lg:text-[5.4rem]"
        >
          Say the word.
          <span class="block bg-gradient-to-r from-wisp-300 via-[#e3b5ff] to-ember-300 bg-clip-text pb-2 text-transparent">
            Build the world.
          </span>
        </h1>

        <p class="text-balance-safe mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-ink-300 sm:text-xl">
          Incant is a game engine and editor where you and an AI agent work on the same scene.
          Drag it, script it, or describe it. Every change is a readable command you can inspect and undo.
        </p>

        <div class="mt-10 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <a
            href="#workflow"
            class="group inline-flex items-center justify-center gap-2 rounded-full bg-white px-6 py-3.5 text-[15px] font-semibold text-ink-950 shadow-[0_10px_40px_-10px_rgba(179,168,255,0.7)] transition hover:bg-wisp-200"
          >
            See how it works
            <span aria-hidden="true" class="transition-transform group-hover:translate-y-0.5">&darr;</span>
          </a>
          <a
            :href="links.productGuide"
            class="inline-flex items-center justify-center gap-2 rounded-full border border-white/15 bg-white/[0.03] px-6 py-3.5 text-[15px] font-semibold text-white transition hover:border-wisp-400/70 hover:bg-wisp-500/15"
          >
            Read the product guide
            <span aria-hidden="true">&nearr;</span>
            <span class="sr-only">(on GitHub, may require access)</span>
          </a>
        </div>
      </div>

      <div class="relative mx-auto mt-24 max-w-6xl sm:mt-32 xl:mt-24">
        <WispMark
          class="absolute -top-[4.75rem] right-3 z-10 h-20 w-auto drop-shadow-[0_10px_30px_rgba(98,70,229,0.75)] motion-safe:animate-float sm:-top-28 sm:right-6 sm:h-28 xl:-top-16 xl:-right-24 xl:h-32"
          body="#5a3ce0"
          blink
        />
        <EditorIllustration />
      </div>
    </div>
  </section>
</template>
