<script setup lang="ts">
import WispMark from './WispMark.vue'
import { links, navItems } from '../content'

const year = new Date().getFullYear()
</script>

<template>
  <section aria-labelledby="cta-title" class="relative isolate overflow-hidden px-5 pb-24 sm:px-8 sm:pb-32">
    <div class="relative mx-auto max-w-6xl overflow-hidden rounded-[2rem] border border-wisp-400/25 bg-gradient-to-br from-wisp-700 via-[#2a1580] to-ink-900 px-4 py-16 text-center sm:px-12 sm:py-24">
      <div aria-hidden="true" class="grain absolute inset-0 opacity-70" />
      <div aria-hidden="true" class="absolute -bottom-40 left-1/2 h-80 w-[50rem] -translate-x-1/2 rounded-full bg-[radial-gradient(closest-side,rgba(255,184,107,0.35),transparent)]" />
      <div class="relative">
        <WispMark class="mx-auto h-24 w-auto drop-shadow-[0_14px_40px_rgba(10,5,40,0.7)] motion-safe:animate-float sm:h-28" body="#ffffff" eyes="#3a1fa6" blink />
        <h2 id="cta-title" class="text-balance-safe mx-auto mt-8 max-w-3xl text-4xl font-semibold tracking-[-0.04em] text-white sm:text-6xl">
          Your next world starts with a word.
        </h2>
        <p class="text-balance-safe mx-auto mt-5 max-w-xl text-lg leading-relaxed text-wisp-200">
          Explore how Incant works, from the document model to the agent, in the product guide.
        </p>
        <div class="mt-10 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <a
            :href="links.productGuide"
            class="inline-flex items-center justify-center gap-2 rounded-full bg-white px-5 py-3.5 text-[15px] font-semibold whitespace-nowrap text-ink-950 transition hover:bg-ember-300 sm:px-6"
          >
            Read the product guide <span aria-hidden="true">&nearr;</span>
            <span class="sr-only">(on GitHub, may require access)</span>
          </a>
          <a
            :href="links.repository"
            class="inline-flex items-center justify-center gap-2 rounded-full border border-white/30 px-5 py-3.5 sm:px-6 text-[15px] font-semibold text-white transition hover:bg-white/10"
          >
            Open on GitHub <span aria-hidden="true">&nearr;</span>
            <span class="sr-only">(may require access)</span>
          </a>
        </div>
      </div>
    </div>
  </section>

  <footer class="border-t border-white/[0.06]">
    <div class="mx-auto flex max-w-7xl flex-col gap-8 px-5 py-12 sm:px-8 md:flex-row md:items-center md:justify-between">
      <div class="flex items-center gap-3">
        <WispMark class="h-7 w-auto" body="#6246e5" />
        <div>
          <p class="font-semibold tracking-[-0.02em] text-white">Incant</p>
          <p class="text-sm text-ink-400">The game engine you build with, together.</p>
        </div>
      </div>
      <nav aria-label="Footer">
        <ul class="flex flex-wrap gap-x-6 gap-y-3 text-sm">
          <li v-for="item in navItems" :key="item.href">
            <a :href="item.href" class="text-ink-300 transition-colors hover:text-white">{{ item.label }}</a>
          </li>
          <li><a :href="links.productGuide" class="text-ink-300 transition-colors hover:text-white">Product guide</a></li>
          <li><a :href="links.repository" class="text-ink-300 transition-colors hover:text-white">GitHub</a></li>
          <li><a href="#top" class="text-ink-300 transition-colors hover:text-white">Back to top <span aria-hidden="true">&uarr;</span></a></li>
        </ul>
      </nav>
    </div>
    <div class="mx-auto max-w-7xl px-5 pb-10 text-xs text-ink-400 sm:px-8">&copy; {{ year }} Incant. All rights reserved.</div>
  </footer>
</template>
