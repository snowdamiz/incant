<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import StagePanel from './StagePanel.vue'
import StageScene from './StageScene.vue'
import WindowBar from './WindowBar.vue'
import { stageSteps, type StageStep } from '../content'

const step = ref<StageStep>('ask')
const tabs = ref<HTMLButtonElement[]>([])
const active = computed(() => stageSteps.find((s) => s.id === step.value) ?? stageSteps[0]!)

const descriptions: Record<StageStep, string> = {
  ask: 'Illustration of the Incant editor, not a screenshot. A cloaked wanderer stands selected on a cliff at dusk, across a gap from a floating isle with an iron ring. In the agent panel the user asks for a grappling hook that pulls the wanderer to whatever it hits, and the agent answers with a three-step plan.',
  review:
    'Illustration of the Incant editor, not a screenshot. A dashed path predicts the grapple from the wanderer to the ring on the floating isle. The panel shows the proposed text change, which adds a GrappleHook component and a script, with Apply and Discard.',
  play: 'Illustration of the Incant editor, not a screenshot. The game is playing. Faded frames trace the wanderer swinging across the gap and landing on the isle, and the play-test report lists three passed checks.',
  undo: 'Illustration of the Incant editor, not a screenshot. The scene is back exactly as it was before the agent worked. The history panel lists the agent’s grappling hook transaction as undone.',
}

const status: Record<StageStep, string> = {
  ask: 'Agent is planning',
  review: '4 edits pending review',
  play: 'Play-test passed',
  undo: 'Transaction undone',
}

const outline = computed(() => {
  const hooked = step.value === 'review' || step.value === 'play'
  return [
    { name: 'Cliffs', depth: 0, tone: 'scene' },
    { name: 'Wanderer', depth: 1, tone: step.value === 'play' ? 'plain' : 'selected' },
    ...(hooked ? [{ name: 'GrappleHook', depth: 2, tone: 'new' }] : []),
    { name: 'Floating Isle', depth: 1, tone: 'plain' },
    { name: 'Anchor Ring', depth: 2, tone: 'plain' },
    { name: 'Lighthouse', depth: 1, tone: 'plain' },
    { name: 'Moonlight', depth: 1, tone: 'plain' },
  ] as const
})

async function onKeydown(event: KeyboardEvent, index: number): Promise<void> {
  const last = stageSteps.length - 1
  const keyMap: Record<string, number> = {
    ArrowRight: index === last ? 0 : index + 1,
    ArrowDown: index === last ? 0 : index + 1,
    ArrowLeft: index === 0 ? last : index - 1,
    ArrowUp: index === 0 ? last : index - 1,
    Home: 0,
    End: last,
  }
  const next = keyMap[event.key]
  if (next === undefined) return
  event.preventDefault()
  const target = stageSteps[next]
  if (!target) return
  step.value = target.id
  await nextTick()
  tabs.value[next]?.focus()
}
</script>

<template>
  <div id="how">
    <h2 class="sr-only">How it works</h2>

    <!-- Step chapters -->
    <div role="tablist" aria-label="One request, step by step" class="grid grid-cols-4 border-t border-rule">
      <button
        v-for="(s, i) in stageSteps"
        :id="`stage-tab-${s.id}`"
        :key="s.id"
        ref="tabs"
        type="button"
        role="tab"
        :aria-selected="s.id === step"
        aria-controls="stage-panel"
        :tabindex="s.id === step ? 0 : -1"
        class="group relative -mt-px border-t-2 py-4 pr-2 text-left sm:pr-4 transition-colors sm:py-5"
        :class="s.id === step ? 'border-accent' : 'border-transparent hover:border-ink/25'"
        @click="step = s.id"
        @keydown="onKeydown($event, i)"
      >
        <span class="flex items-baseline gap-2.5">
          <span class="font-display-soft hidden text-[15px] italic min-[520px]:inline" :class="s.id === step ? 'text-accent' : 'text-muted'">{{ s.numeral }}</span>
          <span class="text-[14px] font-semibold whitespace-nowrap min-[400px]:text-[15px] sm:text-base" :class="s.id === step ? 'text-ink' : 'text-muted group-hover:text-ink'">{{ s.label }}</span>
        </span>
        <span class="mt-1 hidden text-[14px] leading-snug text-muted md:block">{{ s.line }}</span>
      </button>
    </div>
    <p class="pb-4 text-[15px] text-muted md:hidden" aria-hidden="true">{{ active.line }}</p>

    <!-- Conceptual editor -->
    <div id="stage-panel" role="tabpanel" :aria-labelledby="`stage-tab-${step}`" tabindex="0" class="rounded-xl md:mt-2">
      <div
        role="img"
        :aria-label="descriptions[step]"
        class="overflow-hidden rounded-xl bg-ed-bg text-ed-text shadow-[0_1px_0_rgb(255_255_255/0.06)_inset,0_50px_100px_-50px_rgb(21_22_26/0.55),0_20px_40px_-30px_rgb(21_22_26/0.4)] ring-1 ring-ink/10"
      >
        <WindowBar file="cliffs.scene">
          <div class="mx-auto hidden items-center gap-1 lg:flex">
            <span v-for="tool in ['M4 4l6 14 2-6 6-2z', 'M12 3v18M3 12h18', 'M20 12a8 8 0 1 1-3-6.2M20 4v4h-4', 'M5 5h6v6H5zM13 13h6v6h-6z']" :key="tool" class="grid size-7 place-items-center rounded-md first:bg-ed-raised">
              <svg viewBox="0 0 24 24" class="size-3.5 text-ed-muted" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round"><path :d="tool" /></svg>
            </span>
          </div>
          <span
            class="ml-auto inline-flex shrink-0 items-center gap-1.5 rounded-md px-2.5 py-1 text-[12px] font-medium lg:ml-0"
            :class="step === 'play' ? 'bg-ed-red/15 text-ed-red' : 'border border-ed-line bg-ed-raised text-ed-text'"
          >
            <svg v-if="step !== 'play'" viewBox="0 0 12 12" class="size-2.5" fill="currentColor"><path d="M3 2l7 4-7 4z" /></svg>
            <span v-else class="size-2 rounded-[2px] bg-ed-red" />
            {{ step === 'play' ? 'Stop' : 'Play' }}
          </span>
        </WindowBar>

        <div class="grid grid-cols-1 md:grid-cols-[minmax(0,1fr)_17.5rem] lg:grid-cols-[12.5rem_minmax(0,1fr)_20rem]">
          <!-- Outline -->
          <div class="hidden border-r border-ed-line bg-ed-panel py-3 lg:block">
            <p class="px-4 pb-2 text-[12px] font-medium text-ed-muted">Scene</p>
            <ul class="space-y-px px-2 text-[13px]">
              <li
                v-for="node in outline"
                :key="node.name"
                class="flex items-center gap-2 rounded-md py-1.5 pr-2"
                :class="[
                  node.tone === 'selected' ? 'bg-ed-selected text-ed-text' : 'text-ed-muted',
                  node.tone === 'new' ? 'text-ed-green' : '',
                ]"
                :style="{ paddingLeft: `${0.6 + node.depth * 0.9}rem` }"
              >
                <span class="truncate">{{ node.name }}</span>
                <span v-if="node.tone === 'new'" class="ml-auto text-[11px]">new</span>
              </li>
            </ul>
          </div>

          <!-- Viewport -->
          <div class="relative aspect-[16/10] min-w-0 overflow-hidden bg-ed-viewport md:aspect-auto md:min-h-[24rem] lg:min-h-[28rem]">
            <StageScene :step="step" class="absolute inset-0" />
            <span
              v-if="step === 'play'"
              class="absolute top-3 left-3 inline-flex items-center gap-1.5 rounded-md bg-ed-bg/80 px-2 py-1 text-[11.5px] font-medium text-ed-red backdrop-blur"
            >
              <span class="size-1.5 rounded-full bg-ed-red motion-safe:animate-pulse" /> Playing
            </span>
            <span
              v-if="step === 'review'"
              class="absolute top-[30%] left-[50%] rounded-md bg-ed-bg/85 px-2 py-1 font-mono text-[11px] text-ed-accent backdrop-blur"
            >Anchor Ring</span>
            <span
              v-if="step === 'undo'"
              class="absolute bottom-3 left-3 rounded-md bg-ed-bg/85 px-2.5 py-1.5 text-[12px] text-ed-text backdrop-blur"
            >Grappling hook undone</span>
          </div>

          <!-- Inspector -->
          <div :key="step" class="min-h-[17rem] border-t border-ed-line bg-ed-panel motion-safe:animate-rise md:border-t-0 md:border-l">
            <StagePanel :step="step" />
          </div>
        </div>

        <!-- Status bar -->
        <div class="flex h-8 items-center justify-between gap-4 border-t border-ed-line px-4 font-mono text-[11px] text-ed-muted">
          <span class="truncate">cliffs.scene</span>
          <span class="shrink-0" :class="step === 'play' ? 'text-ed-green' : step === 'undo' ? 'text-ed-amber' : ''">{{ status[step] }}</span>
        </div>
      </div>
    </div>
    <p class="mt-4 text-[13px] text-muted">Product illustration.</p>
  </div>
</template>
