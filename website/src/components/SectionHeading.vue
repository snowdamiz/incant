<script setup lang="ts">
defineProps<{ id: string; index: string; label: string; title: string }>()
</script>

<template>
  <div class="grid gap-8 lg:grid-cols-12 lg:items-end lg:gap-10">
    <div class="lg:col-span-7">
      <p class="eyebrow flex items-center gap-3 text-muted">
        <span class="text-accent">{{ index }}</span>
        <span class="h-px w-6 bg-line-strong" aria-hidden="true" />
        {{ label }}
      </p>
      <h2 :id="id" class="display text-balance-safe mt-6 text-[2.5rem] leading-[1] min-[400px]:text-[2.8rem] sm:text-[3.75rem] lg:text-[4.5rem]">
        {{ title }}
      </h2>
    </div>
    <p class="text-pretty-safe max-w-xl text-[17px] leading-relaxed text-fg-2 sm:text-lg lg:col-span-5 lg:pb-2">
      <slot />
    </p>
  </div>
</template>
