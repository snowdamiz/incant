<script setup lang="ts">
import { principles } from '../content'
</script>

<template>
  <section id="principles" aria-labelledby="principles-title" class="border-t border-rule py-24 sm:py-32">
    <div class="mx-auto grid max-w-[82rem] gap-14 px-5 sm:px-8 lg:grid-cols-12 lg:gap-8 lg:px-12">
      <h2
        id="principles-title"
        class="font-display-soft text-balance-safe text-[2.6rem] leading-[1] font-[420] tracking-[-0.025em] sm:text-[3.6rem] lg:col-span-4"
      >
        Built for working <em class="italic">together.</em>
      </h2>

      <ol class="grid gap-x-10 sm:grid-cols-2 lg:col-span-8">
        <li v-for="item in principles" :key="item.numeral" class="border-t border-rule pt-6 pb-12 sm:pb-14">
          <p class="font-display-soft text-[15px] text-accent italic">{{ item.numeral }}</p>
          <h3 class="font-display-soft mt-3 text-[1.75rem] leading-tight font-[450] tracking-[-0.015em]">{{ item.title }}</h3>
          <p class="text-pretty-safe mt-3 max-w-sm text-[16px] leading-relaxed text-muted">{{ item.body }}</p>
        </li>
      </ol>
    </div>
  </section>
</template>
