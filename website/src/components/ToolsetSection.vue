<script setup lang="ts">
import SectionHeading from './SectionHeading.vue'
import { tools } from '../content'
import { toolGlyphs } from '../icons/tools'
</script>

<template>
  <section id="toolset" aria-labelledby="toolset-title" class="border-t border-line py-24 sm:py-32">
    <div class="mx-auto max-w-[90rem] px-5 sm:px-8 lg:px-12">
      <SectionHeading id="toolset-title" index="03" label="Toolset" title="Everything between the first cube and the store page.">
        Modeling, materials, animation, physics, networking and UI live in one application, so nothing leaves the project to
        be finished somewhere else.
      </SectionHeading>

      <ul class="mt-16 grid grid-cols-1 border-t border-l border-line sm:grid-cols-2 lg:grid-cols-4">
        <li
          v-for="tool in tools"
          :key="tool.title"
          class="flex gap-5 border-r border-b border-line p-5 transition-colors hover:bg-white/[0.02] sm:block sm:p-8"
        >
          <svg viewBox="0 0 24 24" class="size-8 shrink-0 text-accent sm:size-9" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path :d="toolGlyphs[tool.icon]" />
          </svg>
          <div>
            <h3 class="text-[17px] font-semibold tracking-[-0.015em] sm:mt-12">{{ tool.title }}</h3>
            <p class="text-pretty-safe mt-1 text-[15px] leading-relaxed text-muted sm:mt-1.5">{{ tool.body }}</p>
          </div>
        </li>
      </ul>
    </div>
  </section>
</template>
