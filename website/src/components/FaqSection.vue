<script setup lang="ts">
import { faqItems } from '../content'
</script>

<template>
  <section id="faq" aria-labelledby="faq-title" class="relative border-t border-white/[0.06] py-24 sm:py-32">
    <div class="mx-auto grid max-w-7xl gap-12 px-5 sm:px-8 lg:grid-cols-[minmax(0,0.75fr)_minmax(0,1.25fr)] lg:gap-20">
      <div>
        <p class="font-mono text-xs font-medium tracking-[0.2em] text-ember-300 uppercase">FAQ</p>
        <h2 id="faq-title" class="text-balance-safe mt-4 text-4xl font-semibold tracking-[-0.035em] text-white sm:text-5xl">
          Questions, answered plainly.
        </h2>
      </div>

      <div class="divide-y divide-white/[0.08] border-y border-white/[0.08]">
        <details v-for="(item, i) in faqItems" :key="item.question" class="group" :open="i === 0">
          <summary
            class="flex cursor-pointer list-none items-center justify-between gap-6 rounded-lg py-6 text-left text-lg font-semibold tracking-[-0.01em] text-white transition-colors hover:text-wisp-200 [&::-webkit-details-marker]:hidden"
          >
            <h3>{{ item.question }}</h3>
            <span
              aria-hidden="true"
              class="grid size-8 shrink-0 place-items-center rounded-full border border-white/15 text-ink-200 transition-transform duration-300 group-open:rotate-45 group-open:border-wisp-400/60 group-open:text-wisp-200"
            >
              <svg viewBox="0 0 16 16" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <path d="M8 3v10M3 8h10" />
              </svg>
            </span>
          </summary>
          <p class="max-w-2xl pb-7 text-[16px] leading-relaxed text-ink-300">{{ item.answer }}</p>
        </details>
      </div>
    </div>
  </section>
</template>
