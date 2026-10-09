<script setup lang="ts">
import { principles } from '../content'
</script>

<template>
  <section id="principles" aria-labelledby="principles-title" class="py-28 sm:py-40">
    <div class="mx-auto max-w-[82rem] px-5 sm:px-8 lg:px-12">
      <h2
        id="principles-title"
        class="font-display-soft text-balance-safe max-w-2xl text-[2.6rem] leading-[1] font-[420] tracking-[-0.025em] sm:text-[3.6rem]"
      >
        Built for working <em class="italic">together.</em>
      </h2>

      <ol class="mt-14 grid gap-x-12 sm:mt-20 md:grid-cols-3">
        <li v-for="item in principles" :key="item.numeral" class="border-t border-rule pt-6 pb-10 md:pb-0">
          <p class="font-display-soft text-[15px] text-accent italic">{{ item.numeral }}</p>
          <h3 class="font-display-soft mt-3 text-[1.75rem] leading-tight font-[450] tracking-[-0.015em]">{{ item.title }}</h3>
          <p class="text-pretty-safe mt-3 max-w-xs text-[16px] leading-relaxed text-muted">{{ item.body }}</p>
        </li>
      </ol>
    </div>
  </section>
</template>
