<script setup lang="ts">
import ModelingStage from './ModelingStage.vue'
import { toolIndex } from '../content'
</script>

<template>
  <section id="create" aria-labelledby="create-title" class="border-t border-rule py-28 sm:py-40 lg:py-48">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <!-- One idea, stated once; the illustration does the explaining. -->
      <div class="max-w-3xl">
        <h2
          id="create-title"
          class="font-display-soft text-balance-safe text-[2.75rem] leading-[0.98] font-[420] tracking-[-0.028em] sm:text-[4rem] lg:text-[5.2rem]"
        >
          Shape it where you <em class="text-accent italic">ship&nbsp;it.</em>
        </h2>
        <p class="text-pretty-safe mt-6 max-w-lg text-[17px] leading-relaxed text-muted sm:mt-8 sm:text-lg">
          Modeling lives inside the engine. Every graph is text the agent can write and you can review.
        </p>
      </div>

      <div class="mt-16 sm:mt-24">
        <ModelingStage />
        <p class="mt-4 text-[13px] text-muted">Select a node to step through the graph.</p>
      </div>

      <!-- The rest of the toolset as a short index, not documentation. -->
      <div class="mt-24 grid gap-10 sm:mt-32 lg:grid-cols-12 lg:gap-8">
        <p
          class="font-display-soft text-balance-safe max-w-xs text-[1.6rem] leading-[1.15] font-[420] tracking-[-0.015em] text-ink lg:col-span-4"
        >
          From first primitive to a game-ready asset.
        </p>
        <dl class="grid gap-x-12 sm:grid-cols-2 lg:col-span-8">
          <div v-for="entry in toolIndex" :key="entry.term" class="border-t border-rule py-5">
            <dt class="font-display-soft text-[1.3rem] leading-snug font-[450] tracking-[-0.01em]">{{ entry.term }}</dt>
            <dd class="mt-1 text-[15px] leading-relaxed text-muted">{{ entry.gloss }}</dd>
          </div>
        </dl>
      </div>
    </div>
  </section>
</template>
