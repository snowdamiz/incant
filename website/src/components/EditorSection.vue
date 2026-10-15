<script setup lang="ts">
import { nextTick, ref } from 'vue'
import EditorWindow from './editor/EditorWindow.vue'
import SectionHeading from './SectionHeading.vue'
import { stageSteps, type StageStep } from '../content'

const step = ref<StageStep>('ask')
const tabs = ref<HTMLButtonElement[]>([])

const descriptions: Record<StageStep, string> = {
  ask: 'Recreation of the Incant editor, with a live viewport. A lighthouse stands on a headland at dusk with its Lantern selected. In the agent panel you ask for a beam that sweeps the bay every 8 seconds, and the agent reads the scene and plans four edits.',
  review:
    'Recreation of the Incant editor, with a live viewport. A dashed preview beam shows the proposed change. The agent asks to apply four changes: create a Beam spot light, attach a sweep script, set the Lantern emission and set the bay fog, with Deny and Allow.',
  play: 'Recreation of the Incant editor, with a live viewport. The game is playing and the new beam sweeps the bay. The agent reports a passed play-test with three checks, and History lists the agent’s four edits as one entry.',
  undo: 'Recreation of the Incant editor, with a live viewport. The beam is gone and the scene is back to revision 41. History shows the agent’s entry as undone, with Redo available.',
}

function select(next: StageStep): void {
  step.value = next
}

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
  <section id="how" aria-labelledby="how-title" class="relative py-24 sm:py-32">
    <div class="mx-auto max-w-[90rem] px-5 sm:px-8 lg:px-12">
      <SectionHeading id="how-title" index="01" label="Editor" title="You and the agent, in the same scene.">
        The agent works through the same commands as your mouse and keyboard. It shows you the exact change, proves it in a
        play-test, and leaves one entry in your history.
      </SectionHeading>

      <div role="tablist" aria-label="One request, step by step" class="mt-16 grid grid-cols-4 gap-x-3 sm:gap-x-6">
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
          class="group border-t-2 pt-4 pb-5 text-left transition-colors"
          :class="s.id === step ? 'border-accent' : 'border-line-strong hover:border-fg-2'"
          @click="step = s.id"
          @keydown="onKeydown($event, i)"
        >
          <span class="flex items-baseline gap-2.5">
            <span class="hidden font-mono text-[12px] sm:inline" :class="s.id === step ? 'text-accent' : 'text-muted'">{{ s.numeral }}</span>
            <span class="text-[15px] font-semibold tracking-[-0.01em] whitespace-nowrap sm:text-[17px]" :class="s.id === step ? 'text-fg' : 'text-muted group-hover:text-fg'">{{ s.label }}</span>
          </span>
          <span class="mt-1.5 hidden text-[14.5px] leading-snug text-muted md:block">{{ s.line }}</span>
        </button>
      </div>

      <div id="stage-panel" role="tabpanel" :aria-labelledby="`stage-tab-${step}`" tabindex="0" class="mt-4 rounded-2xl sm:mt-6">
        <div
          role="img"
          :aria-label="descriptions[step]"
          class="rounded-[15px] bg-[linear-gradient(to_bottom,rgb(255_255_255/0.16),rgb(255_255_255/0.04))] p-px shadow-[0_60px_140px_-40px_rgb(0_0_0/0.9),0_0_0_1px_rgb(0_0_0/0.6)]"
        >
          <EditorWindow :step="step" class="md:h-[40rem] lg:h-[46rem]" @go="select" />
        </div>
      </div>
      <p class="mt-5 text-[13px] text-muted">A recreation of the Incant editor. Its viewport renders live in your browser.</p>
    </div>
  </section>
</template>
