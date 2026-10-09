<script setup lang="ts">
/** Title bar shared by the conceptual editor windows, so both read as the same product. */
import WispMark from './WispMark.vue'

defineProps<{ file: string }>()
</script>

<template>
  <div class="flex h-11 items-center gap-3 border-b border-ed-line px-4">
    <div class="flex gap-1.5">
      <span class="size-2.5 rounded-full bg-white/15" />
      <span class="size-2.5 rounded-full bg-white/15" />
      <span class="size-2.5 rounded-full bg-white/15" />
    </div>
    <div class="ml-2 flex min-w-0 items-center gap-2 text-[12.5px]">
      <WispMark class="h-4 w-auto shrink-0" body="#8c95ff" />
      <span class="truncate font-medium">Lantern Reach</span>
      <span class="hidden text-ed-muted sm:inline">/ {{ file }}</span>
    </div>
    <slot />
  </div>
</template>
