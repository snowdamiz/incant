<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { createGlView, type GlView, type UniformValue } from '../gl/renderer'

const props = withDefaults(
  defineProps<{
    fragment: string
    /** Accessible description of the rendered picture; omit when a parent describes it. */
    label?: string
    animate?: boolean
    maxScale?: number
    minScale?: number
    frame: (time: number, dt: number, aspect: number) => Record<string, UniformValue>
  }>(),
  { label: undefined, animate: true, maxScale: 1.5, minScale: 0.35 },
)

// Consumers position the root element (absolute or relative); the canvas fills it.
const canvas = ref<HTMLCanvasElement | null>(null)
const supported = ref(true)
let view: GlView | null = null

onMounted(() => {
  if (!canvas.value) return
  view = createGlView(canvas.value, {
    fragment: props.fragment,
    animate: props.animate,
    maxScale: props.maxScale,
    minScale: props.minScale,
    frame: (time, dt, aspect) => props.frame(time, dt, aspect),
    onFallback: () => { supported.value = false },
  })
  supported.value = view !== null
})

onBeforeUnmount(() => view?.destroy())

defineExpose({ invalidate: () => view?.invalidate() })
</script>

<template>
  <div :role="label ? 'img' : undefined" :aria-label="label">
    <canvas v-show="supported" ref="canvas" class="absolute inset-0 block size-full" aria-hidden="true" />
    <div v-if="!supported" data-gl-fallback class="absolute inset-0" aria-hidden="true">
      <slot name="fallback" />
    </div>
  </div>
</template>
