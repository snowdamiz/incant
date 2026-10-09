<script setup lang="ts">
import { faqItems, links } from '../content'
</script>

<template>
  <section id="faq" aria-labelledby="faq-title" class="py-24 sm:py-32">
    <div class="mx-auto grid max-w-[82rem] gap-12 px-5 sm:px-8 lg:grid-cols-12 lg:gap-8 lg:px-12">
      <div class="lg:col-span-4">
        <h2 id="faq-title" class="font-display-soft text-[2.6rem] leading-[1] font-[420] tracking-[-0.025em] sm:text-[3.6rem]">
          Questions, answered.
        </h2>
        <p class="mt-5 max-w-xs text-[16px] leading-relaxed text-muted">
          The
          <a :href="links.productGuide" class="text-ink underline decoration-rule underline-offset-4 hover:decoration-accent">
            product guide<span class="sr-only"> (on GitHub)</span></a>
          goes deeper, from the document model to the agent.
        </p>
      </div>

      <div class="border-t border-rule lg:col-span-8">
        <details v-for="(item, i) in faqItems" :key="item.question" class="group border-b border-rule" :open="i === 0">
          <summary
            class="flex cursor-pointer list-none items-center justify-between gap-6 py-6 text-left [&::-webkit-details-marker]:hidden"
          >
            <h3 class="text-[18px] font-medium tracking-[-0.01em] sm:text-[19px]">{{ item.question }}</h3>
            <svg viewBox="0 0 20 20" class="size-5 shrink-0 text-muted transition-colors group-hover:text-accent" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
              <path d="M4 10h12" />
              <path d="M10 4v12" class="origin-center transition-transform duration-300 group-open:scale-y-0" />
            </svg>
          </summary>
          <p class="text-pretty-safe max-w-2xl pb-7 text-[16px] leading-relaxed text-muted">{{ item.answer }}</p>
        </details>
      </div>
    </div>
  </section>
</template>
