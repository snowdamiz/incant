<script setup lang="ts">
import ModelingStage from './ModelingStage.vue'
import { toolGroups } from '../content'
</script>

<template>
  <section id="create" aria-labelledby="create-title" class="border-t border-rule py-24 sm:py-32">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <div class="grid gap-8 lg:grid-cols-12 lg:items-end">
        <h2
          id="create-title"
          class="font-display-soft text-balance-safe text-[2.6rem] leading-[1] font-[420] tracking-[-0.025em] sm:text-[3.6rem] lg:col-span-7 lg:text-[4.4rem]"
        >
          Shape it where you <em class="text-accent italic">ship&nbsp;it.</em>
        </h2>
        <p class="text-pretty-safe max-w-md text-[17px] leading-relaxed text-muted lg:col-span-5 lg:pb-2">
          Incant is a modeling tool as much as an engine. Take an asset from its first primitive to a game-ready mesh, material
          and motion without leaving the project. Every graph is text, so the agent can write one and you can review it.
        </p>
      </div>

      <div class="mt-14 sm:mt-20">
        <ModelingStage />
        <p class="mt-4 text-[13px] text-muted">Select a node to see the model at that step of the graph.</p>
      </div>

      <ul class="mt-20 grid gap-x-10 sm:mt-24 sm:grid-cols-2 lg:grid-cols-3">
        <li v-for="group in toolGroups" :key="group.title" class="border-t border-rule pt-6 pb-10 sm:pb-12">
          <h3 class="font-display-soft text-[1.5rem] leading-tight font-[450] tracking-[-0.015em]">{{ group.title }}</h3>
          <p class="text-pretty-safe mt-3 max-w-sm text-[16px] leading-relaxed text-muted">{{ group.body }}</p>
        </li>
      </ul>
    </div>
  </section>
</template>
