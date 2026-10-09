<script setup lang="ts">
/**
 * Illustrated geometry-graph editor. Selecting a node previews the model up to that node,
 * the way a procedural modeling tool evaluates a graph. Nodes are toggle buttons rather than
 * tabs, so the page keeps a single tab list (the hero walkthrough).
 */
import { computed, onMounted, ref } from 'vue'
import LighthouseModel from './LighthouseModel.vue'
import WindowBar from './WindowBar.vue'
import WispMark from './WispMark.vue'
import { lighthouseGraph } from '../content'

const last = lighthouseGraph.length - 1
const step = ref(last)
const strip = ref<HTMLOListElement | null>(null)

// On phones the nodes are a horizontal strip and the default selection is the last node.
// Scroll only the strip (never the page) so the selected node starts in view.
onMounted(() => {
  if (strip.value) strip.value.scrollLeft = strip.value.scrollWidth
})

const current = computed(() => lighthouseGraph[step.value] ?? lighthouseGraph[last]!)

const description = computed(() => {
  const applied = lighthouseGraph.slice(0, step.value + 1).map((n) => n.adds)
  return `Illustration of the Incant geometry graph editor, not a screenshot. A clay-shaded lighthouse evaluated up to the ${current.value.op} node: ${applied.join(', then ')}.`
})
</script>

<template>
  <div
    class="overflow-hidden rounded-xl bg-ed-bg text-ed-text shadow-[0_1px_0_rgb(255_255_255/0.06)_inset,0_50px_100px_-50px_rgb(21_22_26/0.55),0_20px_40px_-30px_rgb(21_22_26/0.4)] ring-1 ring-ink/10"
  >
    <WindowBar file="lighthouse.geo">
      <span class="ml-auto shrink-0 text-[12px] text-ed-muted">Geometry graph</span>
    </WindowBar>

    <div class="grid grid-cols-1 md:grid-cols-[17rem_minmax(0,1fr)] lg:grid-cols-[19rem_minmax(0,1fr)]">
      <!-- Viewport (first on small screens) -->
      <div
        role="img"
        :aria-label="description"
        class="relative aspect-[4/3] min-w-0 overflow-hidden bg-ed-viewport md:order-2 md:aspect-auto md:min-h-[26rem] lg:min-h-[30rem]"
      >
        <LighthouseModel :step="step" class="absolute inset-0" />
        <span class="absolute top-3 left-3 rounded-md bg-ed-bg/80 px-2 py-1 font-mono text-[11.5px] text-ed-muted">Clay · Perspective</span>
        <span
          v-if="step === last"
          class="absolute right-3 bottom-3 rounded-md bg-ed-bg/80 px-2 py-1 font-mono text-[11.5px] text-ed-accent"
        >LOD 0 of 3</span>
      </div>

      <!-- Graph: a vertical node list from md up, a scrollable strip of node names on phones -->
      <div class="flex min-w-0 flex-col border-t border-ed-line bg-ed-panel md:order-1 md:border-t-0 md:border-r">
        <p class="hidden border-b border-ed-line px-4 py-3 text-[12px] font-medium text-ed-muted md:block">Graph</p>
        <ol ref="strip" class="relative flex flex-1 gap-1 overflow-x-auto px-2 py-2 md:block md:overflow-visible">
          <span class="absolute top-6 bottom-6 left-[1.375rem] hidden w-px bg-ed-line md:block" aria-hidden="true" />
          <li v-for="(node, i) in lighthouseGraph" :key="node.id" class="relative shrink-0">
            <button
              type="button"
              :aria-pressed="i === step"
              class="flex min-h-11 w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left transition-colors duration-150 focus-visible:outline-ed-accent md:gap-3"
              :class="i === step ? 'bg-ed-selected' : 'hover:bg-white/[0.04]'"
              @click="step = i"
            >
              <span
                class="size-2 shrink-0 rounded-full border"
                :class="i <= step ? 'border-ed-accent bg-ed-accent' : 'border-ed-control bg-ed-panel'"
                aria-hidden="true"
              />
              <span class="min-w-0">
                <span class="sr-only">Preview after </span>
                <span class="block text-[13px] font-medium whitespace-nowrap" :class="i > step ? 'text-ed-muted' : ''">{{ node.op }}</span>
                <span class="hidden truncate font-mono text-[12px] text-ed-muted md:block">{{ node.detail }}</span>
              </span>
            </button>
          </li>
        </ol>
        <div class="hidden items-center gap-2.5 border-t border-ed-line px-4 py-3 md:flex">
          <WispMark class="h-4 w-auto shrink-0" body="#8c95ff" />
          <span class="truncate text-[12.5px] text-ed-muted">Ask the agent to edit this graph…</span>
        </div>
      </div>
    </div>

    <div class="flex h-8 items-center justify-between gap-4 border-t border-ed-line px-4 font-mono text-[11px] text-ed-muted">
      <span class="truncate">lighthouse.geo · {{ lighthouseGraph.length }} nodes</span>
      <span class="shrink-0">Previewing {{ current.op }}</span>
    </div>
    <p class="sr-only" aria-live="polite">Previewing {{ current.op }}, which adds {{ current.adds }}.</p>
  </div>
</template>
