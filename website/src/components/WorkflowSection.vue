<script setup lang="ts">
import { exportTargets, workflowSteps } from '../content'
</script>

<template>
  <section id="workflow" aria-labelledby="workflow-title" class="border-t border-rule bg-paper-deep py-24 sm:py-32">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <div class="grid gap-8 lg:grid-cols-12 lg:items-end">
        <h2
          id="workflow-title"
          class="font-display-soft text-balance-safe text-[2.6rem] leading-[1] font-[420] tracking-[-0.025em] sm:text-[3.6rem] lg:col-span-7 lg:text-[4.4rem]"
        >
          One project, from shape to <em class="italic">ship.</em>
        </h2>
        <p class="text-pretty-safe max-w-md text-[17px] leading-relaxed text-muted lg:col-span-5 lg:pb-2">
          Model it, surface it, script it, let the agent help, prove it plays, then ship it. Every step lives in the same
          project and the same history, whether you work by hand or with the agent.
        </p>
      </div>

      <ol class="mt-16 grid sm:mt-20 lg:grid-cols-6">
        <li
          v-for="s in workflowSteps"
          :key="s.numeral"
          class="relative border-l border-rule pb-10 pl-7 last:pb-0 lg:border-t lg:border-l-0 lg:pt-8 lg:pr-6 lg:pb-0 lg:pl-0"
        >
          <span class="absolute top-1.5 -left-[5px] size-[9px] rounded-full bg-ink lg:-top-[5px] lg:left-0" aria-hidden="true" />
          <p class="font-display-soft text-[15px] text-accent italic">{{ s.numeral }}</p>
          <h3 class="font-display-soft mt-2 text-[1.6rem] leading-tight font-[450] tracking-[-0.015em]">{{ s.title }}</h3>
          <p class="text-pretty-safe mt-2 max-w-sm text-[15.5px] leading-relaxed text-muted">{{ s.body }}</p>
          <p class="mt-4 font-mono text-[12.5px] text-ink-soft">{{ s.artifact }}</p>
        </li>
      </ol>

      <div class="mt-20 grid gap-8 border-t border-rule pt-10 sm:mt-24 lg:grid-cols-12 lg:gap-8">
        <div class="lg:col-span-4">
          <h3 class="font-display-soft text-[1.6rem] leading-tight font-[450] tracking-[-0.015em]">Every platform, one project</h3>
          <p class="text-pretty-safe mt-3 max-w-sm text-[16px] leading-relaxed text-muted">
            Native Xcode and Gradle projects are generated, not hidden. Steam integration, crash reporting and a dedicated
            server build for multiplayer come from the same pipeline.
          </p>
        </div>
        <ul class="flex flex-wrap items-baseline gap-x-5 gap-y-1 lg:col-span-8 lg:gap-x-7" aria-label="Export targets">
          <li
            v-for="(target, i) in exportTargets"
            :key="target"
            class="font-display-soft text-[2.2rem] leading-[1.15] font-[400] tracking-[-0.02em] sm:text-[3rem] lg:text-[3.6rem]"
          >
            {{ target
            }}<svg
              v-if="i < exportTargets.length - 1"
              viewBox="0 0 10 20"
              class="ml-5 inline-block h-[0.62em] w-[0.31em] text-ink/25 lg:ml-7"
              aria-hidden="true"
              focusable="false"
            ><path d="M9 1 1 19" fill="none" stroke="currentColor" stroke-width="1.2" /></svg>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>
