<script setup lang="ts">
defineProps<{ eyebrow: string; title: string }>()
</script>

<template>
  <article
    class="group relative flex flex-col overflow-hidden rounded-3xl border border-white/[0.08] bg-gradient-to-b from-ink-850 to-ink-900 p-6 transition-colors hover:border-white/[0.14] sm:p-8"
  >
    <div class="relative mb-7 flex min-h-0 min-w-0 flex-1 flex-col justify-center">
      <slot name="art" />
    </div>
    <p class="font-mono text-[11px] font-medium tracking-[0.16em] text-wisp-300 uppercase">{{ eyebrow }}</p>
    <h3 class="mt-2 text-xl font-semibold tracking-[-0.02em] text-white sm:text-[1.4rem]">{{ title }}</h3>
    <p class="mt-2.5 max-w-prose text-[15px] leading-relaxed text-ink-300">
      <slot />
    </p>
  </article>
</template>
