<script setup lang="ts">
import { exportTargets, workflowSteps } from '../content'
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
        <p class="text-pretty-safe mt-6 max-w-lg text-[17px] leading-relaxed text-muted sm:mt-8 sm:text-lg">
          Every step lives in the same project and the same history.
        </p>
      </div>

      <!-- A verb and the artifact it leaves behind: the track reads at a glance. -->
      <ol class="mt-20 grid sm:mt-28 lg:grid-cols-6">
        <li
          v-for="s in workflowSteps"
          :key="s.title"
          class="relative border-l border-rule pb-10 pl-7 last:pb-0 lg:border-t lg:border-l-0 lg:pt-8 lg:pr-4 lg:pb-0 lg:pl-0"
        >
          <span class="absolute top-[0.7rem] -left-[5px] size-[9px] rounded-full bg-ink lg:-top-[5px] lg:left-0" aria-hidden="true" />
          <p class="font-display-soft text-[1.75rem] leading-tight font-[430] tracking-[-0.02em] sm:text-[2rem]">{{ s.title }}</p>
          <p class="mt-2 font-mono text-[12.5px] text-muted">{{ s.artifact }}</p>
        </li>
      </ol>

      <!-- Where "Ship" lands. -->
      <div class="mt-24 sm:mt-32">
        <ul class="flex flex-wrap items-baseline gap-x-5 gap-y-1 lg:gap-x-7" aria-label="Export platforms">
          <li
            v-for="(target, i) in exportTargets"
            :key="target"
            class="font-display-soft text-[2.2rem] leading-[1.15] font-[400] tracking-[-0.02em] sm:text-[3rem] lg:text-[4.2rem]"
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
        <p class="text-pretty-safe mt-8 max-w-lg text-[15px] leading-relaxed text-muted">
          Native Xcode and Gradle projects included, along with Steam, crash reporting and dedicated servers.
        </p>
      </div>
    </div>
  </section>
</template>
