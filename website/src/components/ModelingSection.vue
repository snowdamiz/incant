<script setup lang="ts">
import ModelingStage from './ModelingStage.vue'
import ToolVignette from './ToolVignette.vue'
import { toolDetails, toolVignettes } from '../content'
</script>

<template>
  <section id="create" aria-labelledby="create-title" class="border-t border-rule py-28 sm:py-40 lg:py-48">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
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

      <!-- The rest of the toolset, shown rather than listed: one strip of connected viewports. -->
      <ul
        aria-label="More of the toolset"
        class="mt-20 grid grid-cols-2 gap-px overflow-hidden rounded-xl bg-ed-line ring-1 ring-ink/10 sm:mt-28 md:grid-cols-3 xl:grid-cols-6"
      >
        <li v-for="tool in toolVignettes" :key="tool.id" class="bg-ed-panel">
          <div role="img" :aria-label="tool.description" class="flex h-full flex-col px-3 pt-4 pb-3 sm:px-4 sm:pt-5">
            <ToolVignette :id="tool.id" />
            <span class="mt-3 font-mono text-[12px] text-ed-muted">{{ tool.label }}</span>
          </div>
        </li>
      </ul>

      <!-- Full detail stays one click away instead of being printed as a list. -->
      <details class="group mt-6 max-w-4xl">
        <summary class="inline-flex cursor-pointer list-none items-center gap-2 py-2 text-[15px] font-medium text-ink [&::-webkit-details-marker]:hidden">
          The whole toolset
          <svg viewBox="0 0 20 20" class="size-4 text-muted transition-transform duration-200 group-open:rotate-45" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true">
            <path d="M4 10h12M10 4v12" />
          </svg>
        </summary>
        <dl class="mt-4 grid gap-x-12 sm:grid-cols-2">
          <div v-for="entry in toolDetails" :key="entry.term" class="border-t border-rule py-4">
            <dt class="text-[15px] font-medium">{{ entry.term }}</dt>
            <dd class="mt-1 text-[15px] leading-relaxed text-muted">{{ entry.gloss }}</dd>
          </div>
        </dl>
      </details>
    </div>
  </section>
</template>
