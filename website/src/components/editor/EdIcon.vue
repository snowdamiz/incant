<script setup lang="ts">
import { edIconPaths, type EdIconName } from './icons'

defineProps<{ name: EdIconName }>()
const filled: EdIconName[] = ['play', 'stop', 'agent']
</script>

<template>
  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path :d="edIconPaths[name]" :fill="filled.includes(name) ? 'currentColor' : 'none'" />
  </svg>
</template>
