<script setup lang="ts">
import SectionHeading from './SectionHeading.vue'
import { exportTargets } from '../content'
import { platformMarks } from '../icons/platforms'

const targets = exportTargets.map((t) => ({ ...t, mark: platformMarks.find((m) => m.id === t.id)! }))
</script>

<template>
  <section id="ship" aria-labelledby="ship-title" class="border-t border-line py-24 sm:py-32">
    <div class="mx-auto max-w-[90rem] px-5 sm:px-8 lg:px-12">
      <SectionHeading id="ship-title" index="05" label="Platforms" title="One project. Six platforms.">
        Export the same project to desktop, mobile and the web, with online multiplayer and cross-play between them. Native
        Xcode and Gradle projects come with it.
      </SectionHeading>

      <ul class="mt-16 grid grid-cols-2 border-t border-l border-line sm:grid-cols-3 lg:grid-cols-6">
        <li v-for="t in targets" :key="t.id" class="flex flex-col items-start border-r border-b border-line px-6 pt-10 pb-7 sm:px-8">
          <span class="flex h-12 items-center" aria-hidden="true">
            <svg :viewBox="t.mark.viewBox" class="w-auto fill-fg" :style="{ height: `${t.mark.optical * 3}rem` }">
              <path :d="t.mark.path" />
            </svg>
          </span>
          <h3 class="mt-12 text-[17px] font-semibold tracking-[-0.015em]">{{ t.mark.name }}</h3>
          <p class="mt-1 font-mono text-[12px] tracking-wide text-muted uppercase">{{ t.output }}</p>
        </li>
      </ul>
    </div>
  </section>
</template>
