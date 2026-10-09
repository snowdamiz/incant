<script setup lang="ts">
import EditorStage from './EditorStage.vue'
import { links } from '../content'
</script>

<template>
  <section id="top" aria-labelledby="hero-title" class="pt-28 pb-24 sm:pt-36 sm:pb-32 lg:pt-44">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <div class="grid gap-10 lg:grid-cols-12 lg:items-end lg:gap-8">
        <h1
          id="hero-title"
          class="font-display-soft text-balance-safe text-[3.25rem] leading-[0.95] font-[420] tracking-[-0.03em] text-ink min-[400px]:text-[3.6rem] sm:text-[5.2rem] lg:col-span-8 lg:text-[6.6rem] xl:text-[7.4rem]"
        >
          The game engine you can <em class="text-wisp italic">talk&nbsp;to.</em>
        </h1>

        <div class="lg:col-span-4 lg:pb-3">
          <p class="text-pretty-safe max-w-md text-[17px] leading-relaxed text-muted sm:text-lg">
            Incant is a game engine and editor where you and an AI agent build the same world. Drag, script, or just say what
            you want. Every change is readable, and every change can be undone.
          </p>
          <div class="mt-8 flex flex-wrap items-center gap-x-6 gap-y-4">
            <a
              :href="links.productGuide"
              class="inline-flex items-center gap-2 rounded-md bg-ink px-5 py-3 text-[15px] font-medium text-paper transition-colors hover:bg-wisp"
            >
              Read the product guide
              <svg viewBox="0 0 16 16" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M5 11l6-6M6 5h5v5" /></svg>
              <span class="sr-only">(on GitHub, may require access)</span>
            </a>
            <a
              :href="links.repository"
              class="text-[15px] font-medium text-ink underline decoration-rule decoration-1 underline-offset-[6px] transition-colors hover:decoration-wisp"
            >
              View on GitHub
              <span class="sr-only">(may require access)</span>
            </a>
          </div>
        </div>
      </div>

      <div class="mt-16 sm:mt-24">
        <EditorStage />
      </div>
    </div>
  </section>
</template>
