<script setup lang="ts">
import GameScene from './GameScene.vue'
import WispMark from './WispMark.vue'
import type { WorkflowStepId } from '../content'

defineProps<{ step: WorkflowStepId }>()

const labels: Record<WorkflowStepId, string> = {
  describe:
    'Conceptual illustration: a prompt asking for a grappling hook, followed by the agent plan to read the player schema, add the component, write the script and play-test it.',
  inspect:
    'Conceptual illustration: a text diff of the scene document adding a GrappleHook component to the player, marked as validated against the schema.',
  playtest:
    'Conceptual illustration: the game viewport during a play-test, with the grapple line reaching the crystal and a list of passed checks.',
  rewind:
    'Conceptual illustration: the shared history list with the agent transaction selected and an undo action that restores the scene without the grappling hook.',
}

const plan = [
  ['Read Player schema', true],
  ['Add GrappleHook component', true],
  ['Write scripts/grapple.ts', true],
  ['Play-test the reach to High Ledge', false],
] as const

const diff = [
  [' ', 'Entity('],
  [' ', '  id: "01JB8Q…4M",'],
  [' ', '  name: "Player",'],
  [' ', '  components: ['],
  [' ', '    Transform(…),'],
  ['+', '    GrappleHook('],
  ['+', '      range: 18.0,'],
  ['+', '      pull: EaseOut,'],
  ['+', '      input: "Grapple",'],
  ['+', '    ),'],
  [' ', '  ],'],
  [' ', ')'],
] as const

const checks = ['Grapple fires on input', 'Hit point found on Crystal Anchor', 'Player lands on High Ledge'] as const

const timeline = [
  { who: 'You', tone: 'bg-ember-400/15 text-ember-300', what: 'Move West Isle', active: false },
  { who: 'Agent', tone: 'bg-wisp-500/25 text-wisp-200', what: 'Add grappling hook · 4 commands', active: true },
  { who: 'You', tone: 'bg-ember-400/15 text-ember-300', what: 'Tune pull speed', active: false },
] as const
</script>

<template>
  <div role="img" :aria-label="labels[step]" class="relative h-full overflow-hidden rounded-2xl border border-white/10 bg-ink-900 shadow-[0_30px_80px_-30px_rgba(75,43,199,0.6)]">
    <!-- Describe -->
    <div v-if="step === 'describe'" class="flex h-full flex-col gap-4 p-5 sm:p-7">
      <div class="rounded-2xl border border-wisp-400/40 bg-ink-950/70 p-4 shadow-[0_0_0_4px_rgba(98,70,229,0.12)]">
        <p class="font-mono text-[10px] tracking-[0.16em] text-ink-400 uppercase">Ask the agent</p>
        <p class="mt-2 text-[15px] leading-relaxed text-white sm:text-base">
          Add a grappling hook that pulls the player toward the hit point. Test it and show me.<span class="ml-0.5 inline-block h-4 w-[2px] translate-y-0.5 bg-wisp-300 motion-safe:animate-pulse" />
        </p>
      </div>
      <div class="flex items-start gap-3">
        <WispMark class="mt-1 h-7 w-auto shrink-0" body="#8b78ff" blink />
        <div class="flex-1 rounded-2xl border border-white/[0.07] bg-ink-850 p-4">
          <p class="text-[13px] font-semibold text-white">Plan</p>
          <ul class="mt-3 space-y-2.5">
            <li v-for="[text, done] in plan" :key="text" class="flex items-center gap-2.5 text-[13px]">
              <span
                class="grid size-4 shrink-0 place-items-center rounded-full text-[9px]"
                :class="done ? 'bg-spell-400 text-ink-950' : 'border border-wisp-300 text-wisp-300'"
              >{{ done ? '✓' : '' }}</span>
              <span :class="done ? 'text-ink-200' : 'text-white'">{{ text }}</span>
            </li>
          </ul>
        </div>
      </div>
    </div>

    <!-- Inspect -->
    <div v-else-if="step === 'inspect'" class="flex h-full flex-col">
      <div class="flex items-center justify-between gap-3 border-b border-white/[0.07] bg-ink-850 px-4 py-2.5 font-mono text-[11px]">
        <span class="truncate text-ink-200">scenes/harbor.scene</span>
        <span class="shrink-0 rounded bg-spell-400/15 px-1.5 py-0.5 text-spell-300">&#10003; schema valid</span>
      </div>
      <ol class="flex-1 overflow-hidden py-3 font-mono text-[11.5px] leading-[1.8] sm:text-[12.5px]">
        <li
          v-for="([mark, line], i) in diff"
          :key="i"
          class="flex gap-3 px-4"
          :class="mark === '+' ? 'bg-spell-400/10 text-spell-300' : 'text-ink-300'"
        >
          <span class="w-5 shrink-0 text-right text-ink-400">{{ i + 1 }}</span>
          <span class="w-2 shrink-0">{{ mark }}</span>
          <span class="whitespace-pre">{{ line }}</span>
        </li>
      </ol>
      <div class="border-t border-white/[0.07] bg-ink-850 px-4 py-2.5 font-mono text-[11px] text-ink-300">
        Transaction · add_component · set_field · create_script · bind_input
      </div>
    </div>

    <!-- Play-test -->
    <div v-else-if="step === 'playtest'" class="flex h-full flex-col">
      <div class="relative min-h-52 flex-1">
        <GameScene uid="wf-play" :gizmo="false" class="absolute inset-0" />
        <span class="absolute top-3 left-3 flex items-center gap-1.5 rounded-md bg-ink-950/75 px-2 py-1 font-mono text-[10px] text-ember-300">
          <span class="size-1.5 rounded-full bg-rose-400 motion-safe:animate-pulse" /> Play-test
        </span>
      </div>
      <ul class="grid gap-2 border-t border-white/[0.07] bg-ink-850 p-4 sm:grid-cols-3">
        <li v-for="check in checks" :key="check" class="flex items-start gap-2 text-[12.5px] text-ink-200">
          <span class="mt-0.5 grid size-4 shrink-0 place-items-center rounded-full bg-spell-400 text-[9px] text-ink-950">&#10003;</span>
          {{ check }}
        </li>
      </ul>
    </div>

    <!-- Rewind -->
    <div v-else class="grid h-full gap-0 sm:grid-cols-[1fr_1.1fr]">
      <div class="flex flex-col gap-2 p-5">
        <p class="font-mono text-[10px] tracking-[0.16em] text-ink-400 uppercase">History</p>
        <ol class="mt-1 space-y-2">
          <li
            v-for="(item, i) in timeline"
            :key="i"
            class="rounded-xl border px-3 py-2.5"
            :class="item.active ? 'border-wisp-400/50 bg-wisp-600/15' : 'border-white/[0.06] bg-ink-950/50'"
          >
            <div class="flex items-center gap-2">
              <span class="rounded-md px-1.5 py-0.5 text-[11px] font-semibold" :class="item.tone">{{ item.who }}</span>
              <span class="truncate text-[13px] text-ink-200">{{ item.what }}</span>
            </div>
            <p v-if="item.active" class="mt-2 inline-flex items-center gap-1.5 rounded-lg bg-white px-2.5 py-1 text-[12px] font-semibold text-ink-950">
              &#8630; Undo transaction
            </p>
          </li>
        </ol>
      </div>
      <div class="relative min-h-48 border-t border-white/[0.07] sm:border-t-0 sm:border-l">
        <GameScene uid="wf-rewind" :grapple="false" :gizmo="false" class="absolute inset-0" />
        <span class="absolute right-3 bottom-3 rounded-md bg-ink-950/75 px-2 py-1 font-mono text-[10px] text-ink-200">Scene restored</span>
      </div>
    </div>
  </div>
</template>
