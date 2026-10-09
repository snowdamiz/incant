<script setup lang="ts">
import { workflowSteps } from '../content'
import { pipelineGlyphs } from '../icons/pipeline'
import { platformMarks } from '../icons/platforms'
</script>

<template>
  <section id="workflow" aria-labelledby="workflow-title" class="bg-paper-deep py-24 sm:py-32">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <div class="grid gap-6 lg:grid-cols-12 lg:items-end lg:gap-8">
        <h2
          id="workflow-title"
          class="font-display-soft text-balance-safe text-[2.75rem] leading-[0.98] font-[420] tracking-[-0.028em] sm:text-[4rem] lg:col-span-7 lg:text-[4.6rem]"
        >
          One project, from shape to <em class="italic">ship.</em>
        </h2>
        <p class="text-pretty-safe max-w-md text-[17px] leading-relaxed text-muted lg:col-span-5 lg:pb-2">
          Six steps in one project and one history. Do any of them by hand, hand them to the agent, or mix both.
        </p>
      </div>

      <!--
        Pipeline: each icon carries its own title and one line.
        Phones: a vertical rail with the text beside each node (smaller tiles below 640 px).
        768 to 1279 px: three columns in two rows, with a connector between neighbours in a row.
        From xl: one horizontal wire across six columns (about 190 px each).
      -->
      <div class="relative mt-14 sm:mt-20">
        <span class="absolute top-8 right-[calc(100%/12)] left-[calc(100%/12)] hidden h-px overflow-hidden bg-ink/20 xl:block" aria-hidden="true">
          <span class="absolute inset-y-0 -left-10 w-10 bg-gradient-to-r from-transparent via-accent to-transparent motion-safe:animate-flow" />
        </span>
        <ol class="relative grid max-w-2xl gap-1 md:max-w-none md:grid-cols-3 md:gap-x-0 md:gap-y-14 xl:grid-cols-6 xl:gap-y-0">
          <li
            v-for="(s, i) in workflowSteps"
            :key="s.id"
            class="relative flex items-start gap-4 py-3 sm:gap-5 md:flex-col md:items-center md:gap-0 md:px-4 md:py-0 md:text-center xl:px-3"
          >
            <!-- Rail segment to the next node: centre to centre, whatever the text height (li py-3 + gap-1). -->
            <span
              v-if="i < workflowSteps.length - 1"
              class="absolute top-9 -bottom-10 left-6 w-px bg-ink/20 sm:top-11 sm:-bottom-12 sm:left-8 md:hidden"
              aria-hidden="true"
            />
            <!-- Row connector (md to xl): from this node's centre to its right-hand neighbour's. -->
            <span
              v-if="i % 3 !== 2"
              class="absolute top-8 left-1/2 hidden h-px w-full bg-ink/20 md:block xl:hidden"
              aria-hidden="true"
            />
            <span
              class="relative grid size-12 shrink-0 place-items-center rounded-xl bg-ed-panel shadow-[0_12px_24px_-16px_rgb(21_22_26/0.6)] ring-1 sm:size-16 sm:rounded-2xl"
              :class="s.id === 'ship' ? 'text-accent-soft ring-accent' : 'text-ed-text ring-ink/10'"
              aria-hidden="true"
            >
              <svg viewBox="0 0 24 24" class="size-6 sm:size-7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                <component :is="el.tag" v-for="(el, j) in pipelineGlyphs[s.id]" :key="j" v-bind="el.attrs" />
              </svg>
            </span>
            <div class="min-w-0 pt-1 sm:pt-2 md:pt-0">
              <h3 class="text-[17px] font-semibold tracking-[-0.01em] md:mt-5">{{ s.title }}</h3>
              <p class="text-pretty-safe mt-1.5 max-w-sm text-[15px] leading-relaxed text-muted md:mx-auto md:max-w-[15rem] xl:max-w-[12.5rem]">
                {{ s.body }}
              </p>
            </div>
          </li>
        </ol>
      </div>

      <!-- Where "Ship" lands: the real platform marks, named for everyone. -->
      <div class="mt-14 border-t border-rule pt-12 sm:mt-16">
        <ul class="grid grid-cols-3 gap-y-10 sm:grid-cols-6" aria-label="Export platforms">
          <li v-for="mark in platformMarks" :key="mark.id" class="flex flex-col items-center">
            <!-- A fixed mark box; each mark is centred on its ink at its optical height. -->
            <span class="flex h-10 items-center sm:h-11 lg:h-12" aria-hidden="true">
              <svg :viewBox="mark.viewBox" :style="{ height: `${mark.optical * 100}%` }" class="w-auto text-ink" focusable="false">
                <path fill="currentColor" :d="mark.path" />
              </svg>
            </span>
            <span class="mt-3 text-[13px] text-muted">{{ mark.name }}</span>
          </li>
        </ul>
        <p class="text-pretty-safe mx-auto mt-10 max-w-md text-center text-[15px] leading-relaxed text-muted">
          Native Xcode and Gradle projects included, along with Steam, crash reporting and dedicated servers.
        </p>
      </div>
    </div>
  </section>
</template>
