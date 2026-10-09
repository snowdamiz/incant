<script setup lang="ts">
import WispMark from './WispMark.vue'

const facts = [
  { title: 'Works offline', body: 'The editor needs no account and no connection.' },
  { title: 'Keys stay local', body: 'Model calls go straight from your machine to your provider.' },
  { title: 'No shell access', body: 'The agent acts only through engine commands and sandboxed scripts.' },
] as const
</script>

<template>
  <section aria-labelledby="statement-title" class="bg-wisp text-paper">
    <div class="mx-auto grid max-w-[82rem] gap-12 px-5 py-24 sm:px-8 sm:py-32 lg:grid-cols-12 lg:items-center lg:gap-8 lg:px-12">
      <div class="lg:col-span-4">
        <WispMark class="h-36 w-auto sm:h-52 lg:h-80" body="#f4f1ea" eyes="#4b2bc7" blink />
      </div>
      <div class="lg:col-span-8">
        <h2
          id="statement-title"
          class="font-display-soft text-balance-safe text-[2.5rem] leading-[1.02] font-[400] tracking-[-0.025em] sm:text-[3.6rem] lg:text-[4.4rem]"
        >
          Nothing the agent does is hidden. Nothing it does is <em class="italic">permanent.</em>
        </h2>
        <ul class="mt-14 grid gap-x-8 gap-y-8 sm:grid-cols-3">
          <li v-for="fact in facts" :key="fact.title" class="border-t border-paper/30 pt-5">
            <h3 class="text-[16px] font-semibold">{{ fact.title }}</h3>
            <p class="mt-1.5 text-[15px] leading-relaxed text-paper/80">{{ fact.body }}</p>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>
