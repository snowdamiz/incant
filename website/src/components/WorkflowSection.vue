<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import WorkflowVisual from './WorkflowVisual.vue'
import { workflowSteps, type WorkflowStepId } from '../content'

const activeId = ref<WorkflowStepId>('describe')
const active = computed(() => workflowSteps.find((s) => s.id === activeId.value) ?? workflowSteps[0]!)
const tabRefs = ref<HTMLButtonElement[]>([])

function select(id: WorkflowStepId): void {
  activeId.value = id
}

async function onKeydown(event: KeyboardEvent, index: number): Promise<void> {
  const last = workflowSteps.length - 1
  let next: number | null = null
  switch (event.key) {
    case 'ArrowRight':
    case 'ArrowDown':
      next = index === last ? 0 : index + 1
      break
    case 'ArrowLeft':
    case 'ArrowUp':
      next = index === 0 ? last : index - 1
      break
    case 'Home':
      next = 0
      break
    case 'End':
      next = last
      break
  }
  if (next === null) return
  event.preventDefault()
  const step = workflowSteps[next]
  if (!step) return
  select(step.id)
  await nextTick()
  tabRefs.value[next]?.focus()
}
</script>

<template>
  <section id="workflow" aria-labelledby="workflow-title" class="relative overflow-hidden border-y border-white/[0.06] bg-ink-900 py-24 sm:py-32">
    <div aria-hidden="true" class="grain pointer-events-none absolute inset-0 opacity-60" />
    <div class="relative mx-auto max-w-7xl px-5 sm:px-8">
      <div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
        <div class="max-w-2xl">
          <p class="font-mono text-xs font-medium tracking-[0.2em] text-spell-300 uppercase">Workflow</p>
          <h2 id="workflow-title" class="text-balance-safe mt-4 text-4xl font-semibold tracking-[-0.035em] text-white sm:text-5xl">
            From a sentence to a play-test.
          </h2>
        </div>
        <p class="max-w-md text-lg leading-relaxed text-ink-300">
          Follow one request through the editor. Choose a step to see what you would see.
        </p>
      </div>

      <div class="mt-12 grid grid-cols-1 gap-6 lg:grid-cols-[19rem_minmax(0,1fr)] lg:gap-10">
        <div role="tablist" aria-label="Workflow steps" class="grid grid-cols-2 gap-2 lg:grid-cols-1 lg:gap-3">
          <button
            v-for="(step, i) in workflowSteps"
            :id="`tab-${step.id}`"
            :key="step.id"
            ref="tabRefs"
            type="button"
            role="tab"
            :aria-selected="step.id === activeId"
            :aria-controls="`panel-${step.id}`"
            :tabindex="step.id === activeId ? 0 : -1"
            class="group relative flex items-center gap-3 rounded-2xl border px-4 py-3.5 text-left transition-colors lg:py-5"
            :class="
              step.id === activeId
                ? 'border-wisp-400/60 bg-wisp-600/20 text-white'
                : 'border-white/[0.08] bg-ink-850/60 text-ink-300 hover:border-white/20 hover:text-white'
            "
            @click="select(step.id)"
            @keydown="onKeydown($event, i)"
          >
            <span
              class="font-mono text-xs"
              :class="step.id === activeId ? 'text-wisp-200' : 'text-ink-400'"
            >{{ step.index }}</span>
            <span class="text-[15px] font-semibold sm:text-base">{{ step.label }}</span>
            <span
              aria-hidden="true"
              class="ml-auto hidden size-2 rounded-full lg:block"
              :class="step.id === activeId ? 'bg-wisp-300 shadow-[0_0_12px_rgba(179,168,255,0.9)]' : 'bg-white/10'"
            />
          </button>
        </div>

        <div
          v-for="step in workflowSteps"
          v-show="step.id === activeId"
          :id="`panel-${step.id}`"
          :key="step.id"
          role="tabpanel"
          :aria-labelledby="`tab-${step.id}`"
          tabindex="0"
          class="grid grid-cols-1 gap-8 rounded-3xl lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)] lg:items-center lg:gap-10"
        >
          <div class="lg:py-4">
            <h3 class="text-balance-safe text-2xl font-semibold tracking-[-0.025em] text-white sm:text-3xl">{{ step.title }}</h3>
            <p class="mt-4 text-[16px] leading-relaxed text-ink-300">{{ step.body }}</p>
            <ul class="mt-6 space-y-3">
              <li v-for="point in step.points" :key="point" class="flex items-start gap-3 text-[15px] text-ink-200">
                <span aria-hidden="true" class="mt-2 size-1.5 shrink-0 rounded-full bg-spell-400" />
                {{ point }}
              </li>
            </ul>
          </div>
          <Transition
            appear
            enter-from-class="opacity-0 translate-y-2"
            enter-active-class="transition duration-500 ease-out"
          >
            <WorkflowVisual v-if="step.id === active.id" :step="step.id" class="min-h-[19rem] sm:min-h-[22rem] lg:min-h-[26rem]" />
          </Transition>
        </div>
      </div>
    </div>
  </section>
</template>
