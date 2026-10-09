<script setup lang="ts">
import WispMark from './WispMark.vue'
import { links, navItems } from '../content'

const year = new Date().getFullYear()
</script>

<template>
  <section aria-labelledby="closing-title" class="border-t border-rule bg-paper-deep">
    <div class="mx-auto max-w-[82rem] px-5 py-24 sm:px-8 sm:py-32 lg:px-12">
      <h2
        id="closing-title"
        class="font-display-soft text-balance-safe text-[3.25rem] leading-[0.95] font-[420] tracking-[-0.03em] sm:text-[5.2rem] lg:text-[7rem]"
      >
        Start with a <em class="text-wisp italic">sentence.</em>
      </h2>
      <div class="mt-10 flex flex-col gap-8 sm:flex-row sm:items-end sm:justify-between">
        <p class="max-w-md text-[17px] leading-relaxed text-muted">
          Everything about how Incant works, from the scene format to the agent, is in the product guide.
        </p>
        <div class="flex flex-wrap items-center gap-x-6 gap-y-4">
          <a
            :href="links.productGuide"
            class="inline-flex items-center gap-2 rounded-md bg-ink px-5 py-3 text-[15px] font-medium text-paper transition-colors hover:bg-wisp"
          >
            Read the product guide
            <svg viewBox="0 0 16 16" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M5 11l6-6M6 5h5v5" /></svg>
            <span class="sr-only">(on GitHub, may require access)</span>
          </a>
          <a
            :href="links.repository"
            class="text-[15px] font-medium underline decoration-rule underline-offset-[6px] transition-colors hover:decoration-wisp"
          >
            View on GitHub<span class="sr-only"> (may require access)</span>
          </a>
        </div>
      </div>
    </div>
  </section>

  <footer class="bg-paper-deep">
    <div class="mx-auto flex max-w-[82rem] flex-col gap-6 border-t border-rule px-5 py-10 sm:px-8 md:flex-row md:items-center md:justify-between lg:px-12">
      <div class="flex items-center gap-3">
        <WispMark class="h-6 w-auto" />
        <span class="font-display-soft text-[19px] font-[500]">Incant</span>
        <span class="text-[13px] text-muted">&copy; {{ year }}</span>
      </div>
      <nav aria-label="Footer">
        <ul class="flex flex-wrap gap-x-6 gap-y-2 text-[14px] text-muted">
          <li v-for="item in navItems" :key="item.href"><a :href="item.href" class="hover:text-ink">{{ item.label }}</a></li>
          <li><a :href="links.productGuide" class="hover:text-ink">Product guide</a></li>
          <li><a :href="links.repository" class="hover:text-ink">GitHub</a></li>
        </ul>
      </nav>
    </div>
  </footer>
</template>
