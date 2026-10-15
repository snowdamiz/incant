<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import GlCanvas from './GlCanvas.vue'
import SectionHeading from './SectionHeading.vue'
import { studioFragment } from '../gl/studio.glsl'
import { lighthouseGraph, type GraphNode } from '../content'

const selected = ref(lighthouseGraph.length - 1)
const node = computed<GraphNode>(() => lighthouseGraph[selected.value]!)

/* Each node switches on one model parameter; the selected node's part is highlighted. */
const focusPart: Record<GraphNode['id'], number> = {
  cylinder: 1,
  taper: 1,
  extrude: 2,
  array: 3,
  boolean: 4,
  scatter: 5,
  output: 0,
}
const values = [0, 0, 0, 0, 0, 0]
const START_ORBIT = 2.45

const gl = ref<InstanceType<typeof GlCanvas> | null>(null)
watch(selected, () => gl.value?.invalidate())

function frame(time: number, dt: number) {
  const k = dt === 0 ? 1 : 1 - Math.exp(-dt * 6)
  for (let i = 0; i < values.length; i++) {
    const target = selected.value >= i + 1 ? 1 : 0
    values[i]! += (target - values[i]!) * k
  }
  return {
    uTime: time,
    uOrbit: START_ORBIT + time * 0.22,
    uStages: [values[0]!, values[1]!, values[2]!, values[3]!],
    uStages2: [values[4]!, values[5]!],
    uFocus: focusPart[node.value.id],
  }
}

const label = computed(() => {
  const parts = lighthouseGraph.slice(0, selected.value + 1).map((n) => n.adds)
  return `Live preview of the Incant geometry graph, displaying the ${node.value.op} node. The lighthouse model is built from ${parts.join(', then ')}.`
})

/* Graph layout: a vertical chain that steps left and right, like a node editor. */
const PITCH = 78
const NODE_W = 212
const NODE_H = 56
const xs = [28, 132]
const layout = lighthouseGraph.map((_, i) => ({ x: xs[i % 2]!, y: 26 + i * PITCH }))
const wires = computed(() =>
  layout.slice(0, -1).map((a, i) => {
  const b = layout[i + 1]!
  const x1 = a.x + NODE_W / 2
  const y1 = a.y + NODE_H
  const x2 = b.x + NODE_W / 2
  const y2 = b.y
  const my = (y1 + y2) / 2
    return { d: `M${x1} ${y1} C${x1} ${my} ${x2} ${my} ${x2} ${y2}`, live: i + 1 <= selected.value }
  }),
)
const graphHeight = 26 * 2 + (lighthouseGraph.length - 1) * PITCH + NODE_H
</script>

<template>
  <section id="create" aria-labelledby="create-title" class="relative border-t border-line py-24 sm:py-32">
    <div class="mx-auto max-w-[90rem] px-5 sm:px-8 lg:px-12">
      <SectionHeading id="create-title" index="02" label="Modeling" title="Model it where you ship it.">
        Procedural geometry graphs live inside the engine, next to the game that uses them. Every node is text the agent
        can write and you can review.
      </SectionHeading>

      <div
        class="mt-16 overflow-hidden rounded-[15px] bg-[linear-gradient(to_bottom,rgb(255_255_255/0.16),rgb(255_255_255/0.04))] p-px shadow-[0_60px_140px_-40px_rgb(0_0_0/0.9)]"
      >
        <div class="overflow-hidden rounded-[14px] bg-ed-app text-[13px] text-ed-text">
          <div class="flex h-11 items-center gap-3 border-b border-ed-border px-4">
            <span class="flex gap-2" aria-hidden="true">
              <span class="size-3 rounded-full bg-[#ff5f57]" /><span class="size-3 rounded-full bg-[#febc2e]" /><span class="size-3 rounded-full bg-[#28c840]" />
            </span>
            <span class="ml-2 font-medium">lighthouse.geo</span>
            <span class="text-ed-subtle">Geometry graph</span>
            <span class="ml-auto hidden font-mono text-[12px] text-ed-subtle sm:inline">{{ selected + 1 }} / {{ lighthouseGraph.length }} nodes</span>
          </div>

          <div class="grid lg:grid-cols-[27rem_minmax(0,1fr)]">
            <!-- Node graph -->
            <div
              role="group"
              aria-label="Geometry graph nodes"
              class="order-2 border-t border-ed-border bg-ed-panel bg-[radial-gradient(rgb(255_255_255/0.07)_1px,transparent_1px)] [background-size:18px_18px] lg:order-1 lg:border-t-0 lg:border-r"
            >
              <!-- Wide screens: positioned nodes and wires. -->
              <div class="relative mx-auto hidden lg:block" :style="{ width: `${xs[1]! + NODE_W + 28}px`, height: `${graphHeight}px` }">
                <svg class="absolute inset-0 size-full overflow-visible" aria-hidden="true">
                  <path
                    v-for="(w, i) in wires"
                    :key="i"
                    :d="w.d"
                    fill="none"
                    stroke-width="1.6"
                    :stroke="w.live ? '#8c95ff' : '#3a3e48'"
                    :stroke-dasharray="w.live ? undefined : '3 4'"
                  />
                </svg>
                <button
                  v-for="(n, i) in lighthouseGraph"
                  :key="n.id"
                  type="button"
                  :aria-pressed="i === selected"
                  class="absolute flex flex-col justify-center rounded-lg border px-3.5 text-left transition-colors"
                  :class="[
                    i === selected ? 'border-ed-accent bg-ed-selected' : i < selected ? 'border-ed-border-strong bg-ed-raised hover:border-ed-muted' : 'border-ed-border bg-ed-raised/60 hover:border-ed-muted',
                  ]"
                  :style="{ left: `${layout[i]!.x}px`, top: `${layout[i]!.y}px`, width: `${NODE_W}px`, height: `${NODE_H}px` }"
                  @click="selected = i"
                >
                  <span class="flex items-center gap-2">
                    <span class="size-2 rounded-[2px]" :class="i <= selected ? 'bg-ed-accent' : 'bg-ed-subtle/50'" aria-hidden="true" />
                    <span class="font-semibold" :class="i <= selected ? 'text-ed-text' : 'text-ed-muted'">{{ n.op }}</span>
                    <span v-if="i === selected" class="ml-auto font-mono text-[10.5px] tracking-wide text-ed-accent uppercase">Display</span>
                  </span>
                  <span class="mt-1 truncate font-mono text-[11.5px] text-ed-subtle">{{ n.detail }}</span>
                </button>
              </div>

              <!-- Narrow screens: a scrollable strip. -->
              <div class="flex gap-2 overflow-x-auto p-3 lg:hidden">
                <button
                  v-for="(n, i) in lighthouseGraph"
                  :key="n.id"
                  type="button"
                  :aria-pressed="i === selected"
                  class="shrink-0 rounded-lg border px-3 py-2 text-left"
                  :class="i === selected ? 'border-ed-accent bg-ed-selected' : 'border-ed-border bg-ed-raised'"
                  @click="selected = i"
                >
                  <span class="block font-semibold" :class="i <= selected ? 'text-ed-text' : 'text-ed-muted'">{{ n.op }}</span>
                </button>
              </div>
            </div>

            <!-- Viewport -->
            <div class="relative order-1 aspect-[4/3] min-w-0 lg:order-2 lg:aspect-auto lg:min-h-[38rem]">
              <GlCanvas ref="gl" class="absolute inset-0" :fragment="studioFragment" :frame="frame" :max-scale="1.5" :label="label">
                <template #fallback>
                  <div class="size-full bg-[radial-gradient(70%_60%_at_50%_55%,#1d1f26_0%,#0e0f12_75%)]" />
                </template>
              </GlCanvas>
              <div class="pointer-events-none absolute top-3 left-3 flex gap-2 text-[11px]" aria-hidden="true">
                <span class="rounded-[5px] bg-ed-app/80 px-2 py-1 text-ed-muted backdrop-blur">Turntable</span>
                <span class="rounded-[5px] bg-ed-app/80 px-2 py-1 text-ed-text backdrop-blur">{{ node.op }}</span>
              </div>
              <p class="pointer-events-none absolute bottom-3 left-3 rounded-[5px] bg-ed-app/80 px-2 py-1 font-mono text-[11.5px] text-ed-soft backdrop-blur" aria-hidden="true">
                {{ node.detail }}
              </p>
            </div>
          </div>
        </div>
      </div>
      <p class="mt-5 text-[13px] text-muted">Select a node to display the model up to that step. Rendered live in your browser.</p>
    </div>
  </section>
</template>
