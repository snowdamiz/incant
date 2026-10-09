<script setup lang="ts">
/**
 * The Incant wisp, redrawn as a local vector from the director's brand reference.
 * Decorative by default; pass `label` to expose it as an image.
 */
withDefaults(
  defineProps<{
    label?: string
    body?: string
    eyes?: string
    blink?: boolean
  }>(),
  { label: undefined, body: '#4b2bc7', eyes: '#ffffff', blink: false },
)
</script>

<template>
  <svg
    viewBox="0 0 420 570"
    :role="label ? 'img' : undefined"
    :aria-label="label"
    :aria-hidden="label ? undefined : 'true'"
    focusable="false"
  >
    <path
      :fill="body"
      d="M285 5C340 12 405 55 408 110C411 160 362 180 332 205C302 230 318 256 346 283C386 322 411 352 408 402C404 472 330 540 245 562C262 546 268 522 236 513C170 505 76 494 36 430C0 374 6 290 62 230C122 170 220 150 282 128C322 113 333 80 320 50C312 30 300 15 285 5Z"
    />
    <g :fill="eyes" :class="blink ? 'origin-[50%_65%] motion-safe:animate-blink' : undefined">
      <ellipse cx="88" cy="355" rx="25" ry="51" transform="rotate(-8 88 355)" />
      <ellipse cx="207" cy="380" rx="34" ry="53" transform="rotate(-16 207 380)" />
    </g>
  </svg>
</template>
