<script setup lang="ts">
/** Small ink-on-paper diagram for each principle. Decorative; the text beside it carries meaning. */
import type { Principle } from '../content'

defineProps<{ id: Principle['id'] }>()

const INK = '#15161a'
const ACCENT = '#4650c8'

/* One history: your edits (ink) and the agent's (accent) on one line; the last is undone. */
const edits = [
  { x: 20, agent: false },
  { x: 46, agent: false },
  { x: 72, agent: true },
  { x: 98, agent: false },
  { x: 124, agent: true, undone: true },
] as const

/* Plain text: a document diff with one added line. */
const lines = [
  { y: 12, w: 92, added: false },
  { y: 26, w: 112, added: true },
  { y: 40, w: 70, added: false },
] as const

/* Evidence: three play-test frames tracing a jump, then a pass. */
const frames = [
  { x: 4, dot: [12, 30] },
  { x: 46, dot: [20, 12] },
  { x: 88, dot: [28, 22] },
] as const
</script>

<template>
  <svg viewBox="0 0 160 56" aria-hidden="true" focusable="false" class="block h-14 w-auto">
    <g v-if="id === 'history'">
      <line x1="8" y1="34" x2="152" y2="34" :stroke="INK" stroke-opacity="0.25" stroke-width="1.5" />
      <circle
        v-for="e in edits"
        :key="e.x"
        :cx="e.x"
        cy="34"
        r="5"
        :fill="'undone' in e ? '#f4f3ef' : e.agent ? ACCENT : INK"
        :stroke="e.agent ? ACCENT : INK"
        stroke-width="1.5"
      />
      <path d="M122 24C116 12 104 12 100 22" fill="none" :stroke="INK" stroke-width="1.5" stroke-linecap="round" />
      <path d="M96 18l4 4.5 4.5-3.5" fill="none" :stroke="INK" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
    </g>

    <g v-else-if="id === 'text'">
      <g v-for="l in lines" :key="l.y">
        <rect x="22" :y="l.y - 3" :width="l.w" height="6" rx="3" :fill="l.added ? ACCENT : INK" :fill-opacity="l.added ? 0.55 : 0.18" />
        <path v-if="l.added" :d="`M6 ${l.y}h8M10 ${l.y - 4}v8`" :stroke="ACCENT" stroke-width="1.75" stroke-linecap="round" />
      </g>
    </g>

    <g v-else>
      <g v-for="f in frames" :key="f.x">
        <rect :x="f.x" y="8" width="36" height="40" rx="3" fill="none" :stroke="INK" stroke-opacity="0.35" stroke-width="1.5" />
        <line :x1="f.x + 4" y1="40" :x2="f.x + 32" y2="40" :stroke="INK" stroke-opacity="0.2" stroke-width="1.5" />
        <circle :cx="f.x + f.dot[0]" :cy="8 + f.dot[1]" r="3" :fill="INK" />
      </g>
      <circle cx="146" cy="28" r="10" fill="none" :stroke="ACCENT" stroke-width="1.75" />
      <path d="M141 28.5l3.5 3.5 6-7" fill="none" :stroke="ACCENT" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" />
    </g>
  </svg>
</template>
