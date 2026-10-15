<script setup lang="ts">
import { faqItems, links } from '../content'
</script>

<template>
  <section id="faq" aria-labelledby="faq-title" class="border-t border-line py-24 sm:py-32">
    <div class="mx-auto grid max-w-[90rem] gap-12 px-5 sm:px-8 lg:grid-cols-12 lg:gap-10 lg:px-12">
      <div class="lg:col-span-5">
        <p class="eyebrow text-muted">FAQ</p>
        <h2 id="faq-title" class="display mt-6 text-[2.5rem] leading-[1] sm:text-[3.5rem]">Questions, answered.</h2>
        <p class="mt-6 max-w-sm text-[16px] leading-relaxed text-muted">
          The
          <a :href="links.productGuide" class="text-fg underline decoration-line-strong underline-offset-4 transition-colors hover:decoration-accent">
            product plan<span class="sr-only"> (on GitHub)</span></a>
          goes deeper, from the document model to the agent.
        </p>
      </div>

      <div class="border-t border-line lg:col-span-7">
        <details v-for="(item, i) in faqItems" :key="item.question" class="group border-b border-line" :open="i === 0">
          <summary class="flex cursor-pointer list-none items-center justify-between gap-6 py-6 text-left [&::-webkit-details-marker]:hidden">
            <h3 class="text-[17px] font-medium tracking-[-0.01em] sm:text-[19px]">{{ item.question }}</h3>
            <svg viewBox="0 0 20 20" class="size-5 shrink-0 text-muted transition-colors group-hover:text-fg" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
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
