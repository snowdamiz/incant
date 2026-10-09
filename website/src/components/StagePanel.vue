<script setup lang="ts">
/** Right-hand inspector of the conceptual editor. Content follows the selected step. */
import WispMark from './WispMark.vue'
import type { StageStep } from '../content'

defineProps<{ step: StageStep }>()

const diff = [
  [' ', 'Wanderer {'],
  [' ', '  Transform { … }'],
  ['+', '  GrappleHook {'],
  ['+', '    range: 18'],
  ['+', '    pull: ease_out'],
  ['+', '  }'],
  [' ', '}'],
] as const

const checks = ['Hook fires on input', 'Hits the anchor on the isle', 'Wanderer lands safely'] as const

const frames = [
  { x: '22%', y: '64%' },
  { x: '48%', y: '40%' },
  { x: '72%', y: '46%' },
] as const

const history = [
  { who: 'You', what: 'Placed the floating isle', undone: false },
  { who: 'You', what: 'Moved the lighthouse', undone: false },
  { who: 'Agent', what: 'Added a grappling hook', undone: true },
] as const
</script>

<template>
  <div class="flex h-full flex-col text-ed-text">
    <!-- Ask -->
    <template v-if="step === 'ask'">
      <p class="border-b border-ed-line px-4 py-3 text-[12px] font-medium text-ed-muted">Agent</p>
      <div class="flex flex-1 flex-col gap-4 p-4">
        <p class="ml-6 rounded-lg rounded-tr-sm bg-ed-raised px-3.5 py-3 text-[13px] leading-relaxed">
          Add a grappling hook that pulls the wanderer to whatever it hits. Test it, then show me.
        </p>
        <div class="flex gap-3">
          <WispMark class="mt-0.5 h-5 w-auto shrink-0" body="#a998ff" />
          <div class="text-[13px] leading-relaxed">
            <p>Here is the plan.</p>
            <ol class="mt-2 space-y-1.5 text-ed-muted">
              <li><span class="font-mono text-[11.5px] text-ed-violet">1</span>&ensp;Add GrappleHook to Wanderer</li>
              <li><span class="font-mono text-[11.5px] text-ed-violet">2</span>&ensp;Write grapple.ts</li>
              <li><span class="font-mono text-[11.5px] text-ed-violet">3</span>&ensp;Play-test the jump to the isle</li>
            </ol>
          </div>
        </div>
        <div class="mt-auto rounded-lg border border-ed-line px-3.5 py-2.5 text-[12.5px] text-ed-muted">
          Ask the agent…
        </div>
      </div>
    </template>

    <!-- Review -->
    <template v-else-if="step === 'review'">
      <div class="flex items-baseline justify-between border-b border-ed-line px-4 py-3 text-[12px]">
        <span class="font-medium text-ed-muted">Proposed change</span>
        <span class="font-mono text-[11.5px] text-ed-muted">cliffs.scene</span>
      </div>
      <div class="flex flex-1 flex-col gap-4 p-4">
        <ol class="overflow-hidden rounded-lg bg-ed-bg py-2 font-mono text-[12px] leading-[1.85]">
          <li
            v-for="([mark, line], i) in diff"
            :key="i"
            class="flex gap-3 px-3"
            :class="mark === '+' ? 'bg-ed-green/10 text-ed-green' : 'text-ed-muted'"
          >
            <span class="w-2 shrink-0">{{ mark === '+' ? '+' : '' }}</span>
            <span class="whitespace-pre">{{ line }}</span>
          </li>
        </ol>
        <p class="font-mono text-[12px] text-ed-green">+ scripts/grapple.ts</p>
        <p class="text-[13px] leading-relaxed text-ed-muted">Four edits in one transaction. Nothing changes until you apply it.</p>
        <div class="mt-auto flex gap-2 text-[12.5px] font-medium">
          <span class="flex-1 rounded-md bg-ed-text py-2 text-center text-ed-bg">Apply</span>
          <span class="flex-1 rounded-md border border-ed-line py-2 text-center">Discard</span>
        </div>
      </div>
    </template>

    <!-- Play-test -->
    <template v-else-if="step === 'play'">
      <div class="flex items-center justify-between border-b border-ed-line px-4 py-3 text-[12px]">
        <span class="font-medium text-ed-muted">Play-test report</span>
        <span class="text-ed-green">Passed</span>
      </div>
      <div class="flex flex-1 flex-col gap-4 p-4">
        <div class="grid grid-cols-3 gap-2">
          <div
            v-for="(frame, i) in frames"
            :key="i"
            class="relative aspect-[4/3] overflow-hidden rounded-md bg-gradient-to-b from-[#2a2048] via-[#6a4268] to-[#c98a72]"
          >
            <span class="absolute bottom-0 left-0 h-[22%] w-[42%] bg-[#120e1c]" />
            <span class="absolute right-0 bottom-[38%] h-[8%] w-[34%] rounded-sm bg-[#1d1630]" />
            <span class="absolute size-1.5 rounded-full bg-[#efe6d4]" :style="{ left: frame.x, top: frame.y }" />
          </div>
        </div>
        <ul class="space-y-2 text-[13px]">
          <li v-for="check in checks" :key="check" class="flex items-center gap-2.5">
            <svg viewBox="0 0 16 16" class="size-4 shrink-0 text-ed-green" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3.5 8.5l3 3 6-7" />
            </svg>
            {{ check }}
          </li>
        </ul>
        <div class="mt-auto flex gap-3 border-t border-ed-line pt-4">
          <WispMark class="mt-0.5 h-5 w-auto shrink-0" body="#a998ff" />
          <p class="text-[13px] leading-relaxed">It works. The wanderer reaches the isle in one pull.</p>
        </div>
      </div>
    </template>

    <!-- Undo -->
    <template v-else>
      <p class="border-b border-ed-line px-4 py-3 text-[12px] font-medium text-ed-muted">History</p>
      <div class="flex flex-1 flex-col p-2">
        <ol>
          <li v-for="(entry, i) in history" :key="i" class="flex items-center gap-3 rounded-md px-2.5 py-2.5" :class="entry.undone ? 'bg-ed-raised' : ''">
            <span
              class="grid size-6 shrink-0 place-items-center rounded-full text-[10.5px] font-semibold"
              :class="entry.who === 'Agent' ? 'bg-ed-violet/20 text-ed-violet' : 'bg-ed-amber/15 text-ed-amber'"
            >{{ entry.who === 'Agent' ? 'A' : 'Y' }}</span>
            <span class="min-w-0 flex-1 text-[13px]" :class="entry.undone ? 'text-ed-muted line-through decoration-ed-muted/60' : ''">
              {{ entry.what }}
            </span>
            <span v-if="entry.undone" class="shrink-0 text-[11.5px] text-ed-amber">Undone</span>
          </li>
        </ol>
        <p class="mt-auto px-2.5 pb-2 text-[12.5px] leading-relaxed text-ed-muted">
          The whole transaction came back out in one step. Redo brings it back.
        </p>
      </div>
    </template>
  </div>
</template>
