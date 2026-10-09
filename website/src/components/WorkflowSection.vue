<script setup lang="ts">
import { workflowSteps } from '../content'
import { pipelineGlyphs } from '../icons/pipeline'
import { platformMarks } from '../icons/platforms'
</script>

<template>
  <section id="workflow" aria-labelledby="workflow-title" class="bg-paper-deep py-28 sm:py-40 lg:py-48">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <div class="max-w-3xl">
        <h2
          id="workflow-title"
          class="font-display-soft text-balance-safe text-[2.75rem] leading-[0.98] font-[420] tracking-[-0.028em] sm:text-[4rem] lg:text-[5.2rem]"
        >
          One project, from shape to <em class="italic">ship.</em>
        </h2>
      </div>

      <!-- Pipeline: product-chrome nodes on one wire. Vertical on phones, horizontal from md. -->
      <div class="relative mt-16 sm:mt-24">
        <span class="absolute top-10 bottom-10 left-8 w-px bg-ink/20 md:hidden" aria-hidden="true" />
        <span class="absolute top-8 right-[calc(100%/12)] left-[calc(100%/12)] hidden h-px overflow-hidden bg-ink/20 md:block" aria-hidden="true">
          <span class="absolute inset-y-0 -left-10 w-10 bg-gradient-to-r from-transparent via-accent to-transparent motion-safe:animate-flow" />
        </span>
        <ol class="relative grid gap-2 md:grid-cols-6 md:gap-0">
          <li v-for="s in workflowSteps" :key="s.id" class="relative flex items-center gap-5 py-2 md:flex-col md:gap-0 md:py-0">
            <span
              class="grid size-16 shrink-0 place-items-center rounded-2xl bg-ed-panel shadow-[0_12px_24px_-16px_rgb(21_22_26/0.6)] ring-1"
              :class="s.id === 'ship' ? 'text-accent-soft ring-accent' : 'text-ed-text ring-ink/10'"
              aria-hidden="true"
            >
              <svg viewBox="0 0 24 24" class="size-7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                <component :is="el.tag" v-for="(el, i) in pipelineGlyphs[s.id]" :key="i" v-bind="el.attrs" />
              </svg>
            </span>
            <span class="text-[16px] font-medium md:mt-5">{{ s.title }}<span class="sr-only">: {{ s.artifact }}</span></span>
          </li>
        </ol>
      </div>

      <!-- Where "Ship" lands: the real platform marks, named for everyone. -->
      <div class="mt-24 border-t border-rule pt-14 sm:mt-32 sm:pt-16">
        <ul class="grid grid-cols-3 gap-y-12 sm:grid-cols-6" aria-label="Export platforms">
          <li v-for="mark in platformMarks" :key="mark.id" class="flex flex-col items-center">
            <svg :viewBox="mark.viewBox" class="h-11 w-auto text-ink sm:h-12 lg:h-14" aria-hidden="true" focusable="false">
              <path fill="currentColor" :d="mark.path" />
            </svg>
            <span class="mt-4 text-[13px] text-muted">{{ mark.name }}</span>
          </li>
        </ul>
        <p class="text-pretty-safe mx-auto mt-14 max-w-md text-center text-[15px] leading-relaxed text-muted">
          Native Xcode and Gradle projects included, along with Steam, crash reporting and dedicated servers.
        </p>
      </div>
    </div>
  </section>
</template>
