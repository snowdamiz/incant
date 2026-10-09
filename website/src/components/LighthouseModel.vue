<script setup lang="ts">
/**
 * Clay-shaded lighthouse for the conceptual geometry-graph viewport.
 * `step` is the index of the last applied graph node; each node adds its own geometry,
 * and the geometry the current node contributes is outlined in the editor accent.
 * Decorative: the surrounding stage carries the accessible description.
 */
import { computed } from 'vue'

const props = defineProps<{ step: number }>()

const ACCENT = '#8c95ff'
const CX = 320
const BASE_Y = 360
const TOP_Y = 110
const BASE_R = 70
const TAPER_R = 46
const SQUASH = 0.228 // ellipse ry / rx, the viewing angle

const topR = computed(() => (props.step >= 1 ? TAPER_R : BASE_R))
const halfWidthAt = (y: number): number => BASE_R - (BASE_R - topR.value) * ((BASE_Y - y) / (BASE_Y - TOP_Y))
const r1 = (n: number): number => Math.round(n * 10) / 10

const tower = computed(() => {
  const t = topR.value
  return `M${CX - BASE_R} ${BASE_Y}L${CX - t} ${TOP_Y}L${CX + t} ${TOP_Y}L${CX + BASE_R} ${BASE_Y}A${BASE_R} ${BASE_R * SQUASH} 0 0 1 ${CX - BASE_R} ${BASE_Y}Z`
})

/* Gallery deck and lantern room (Extrude). */
const DECK_R = 62
const DECK_RY = DECK_R * SQUASH
const DECK_T = 7
const deckBand = `M${CX - DECK_R} ${TOP_Y}L${CX - DECK_R} ${TOP_Y + DECK_T}A${DECK_R} ${DECK_RY} 0 0 0 ${CX + DECK_R} ${TOP_Y + DECK_T}L${CX + DECK_R} ${TOP_Y}A${DECK_R} ${DECK_RY} 0 0 1 ${CX - DECK_R} ${TOP_Y}Z`
const LANTERN_R = 24
const LANTERN_H = 38
const lanternTop = TOP_Y - LANTERN_H
const lantern = `M${CX - LANTERN_R} ${TOP_Y}L${CX - LANTERN_R} ${lanternTop}L${CX + LANTERN_R} ${lanternTop}L${CX + LANTERN_R} ${TOP_Y}A${LANTERN_R} ${r1(LANTERN_R * SQUASH)} 0 0 1 ${CX - LANTERN_R} ${TOP_Y}Z`
const dome = `M${CX - 28} ${lanternTop}C${CX - 28} ${lanternTop - 26} ${CX + 28} ${lanternTop - 26} ${CX + 28} ${lanternTop}Z`
const RAIL_R = 60
const RAIL_Y = TOP_Y - 7
const rail = `M${CX - RAIL_R} ${RAIL_Y}A${RAIL_R} ${r1(RAIL_R * SQUASH)} 0 0 0 ${CX + RAIL_R} ${RAIL_Y}`
const posts = [-48, -24, 0, 24, 48].map((dx) => ({
  x: CX + dx,
  y1: r1(RAIL_Y + RAIL_R * SQUASH * Math.sqrt(1 - (dx / RAIL_R) ** 2)),
  y2: r1(TOP_Y + DECK_RY * Math.sqrt(1 - (dx / DECK_R) ** 2)),
}))

/* Windows (Array), placed on a spiral up the front face. */
const windows = [
  { dx: -20, y: 300 },
  { dx: 16, y: 248 },
  { dx: -10, y: 196 },
  { dx: 17, y: 146 },
]

/* Doorway (Boolean subtract). The sill follows the base ellipse. */
const door = `M${CX - 11} ${BASE_Y + 16}V${BASE_Y - 6}a11 11 0 0 1 22 0V${BASE_Y + 16}Z`

/* Scattered rocks. Back rocks are drawn before the tower so it occludes them. */
const rockBody = '-12,0 -9,-8 -2,-12 7,-10 12,-3 10,0'
const rockTop = '-9,-8 -2,-12 7,-10 1,-6'
const backRocks = [
  { x: 236, y: 352, s: 1 },
  { x: 406, y: 350, s: 0.9 },
]
const frontRocks = [
  { x: 190, y: 384, s: 0.6 },
  { x: 212, y: 372, s: 1.3 },
  { x: 238, y: 386, s: 0.9 },
  { x: 260, y: 379, s: 0.7 },
  { x: 388, y: 383, s: 1.1 },
  { x: 414, y: 372, s: 1.4 },
  { x: 438, y: 386, s: 0.8 },
  { x: 454, y: 368, s: 0.7 },
]

/* Output: retopologised wireframe over the finished tower. */
const wireRings = computed(() =>
  [1, 2, 3, 4, 5, 6, 7].map((k) => {
    const y = TOP_Y + (k * (BASE_Y - TOP_Y)) / 8
    const w = r1(halfWidthAt(y))
    return `M${r1(CX - w)} ${r1(y)}A${w} ${r1(w * SQUASH)} 0 0 0 ${r1(CX + w)} ${r1(y)}`
  }),
)
const wireSpokes = computed(() =>
  [-70, -45, -20, 0, 20, 45, 70].map((deg) => {
    const a = (deg * Math.PI) / 180
    const t = topR.value
    return `M${r1(CX + BASE_R * Math.sin(a))} ${r1(BASE_Y + BASE_R * SQUASH * Math.cos(a))}L${r1(CX + t * Math.sin(a))} ${r1(TOP_Y + t * SQUASH * Math.cos(a))}`
  }),
)

/* Ground grid in perspective, clipped below the horizon. */
const gridRows = [300, 310, 323, 340, 362, 392, 432]
const gridCols = Array.from({ length: 13 }, (_, i) => -400 + i * 120)

const outline = (node: number): { stroke: string; 'stroke-width': number } =>
  props.step === node ? { stroke: ACCENT, 'stroke-width': 2 } : { stroke: 'none', 'stroke-width': 0 }
</script>

<template>
  <svg viewBox="0 0 640 440" preserveAspectRatio="xMidYMid meet" aria-hidden="true" focusable="false" class="block size-full">
    <defs>
      <linearGradient id="lh-clay" gradientUnits="userSpaceOnUse" x1="250" y1="0" x2="390" y2="0">
        <stop offset="0" stop-color="#9a9ca3" />
        <stop offset="0.3" stop-color="#d3d4d8" />
        <stop offset="0.62" stop-color="#b3b5bb" />
        <stop offset="1" stop-color="#5c5f66" />
      </linearGradient>
      <linearGradient id="lh-clay-small" gradientUnits="userSpaceOnUse" x1="292" y1="0" x2="348" y2="0">
        <stop offset="0" stop-color="#a3a5ab" />
        <stop offset="0.35" stop-color="#d8d9dc" />
        <stop offset="1" stop-color="#686b72" />
      </linearGradient>
      <radialGradient id="lh-pool" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#ffffff" stop-opacity="0.05" />
        <stop offset="1" stop-color="#ffffff" stop-opacity="0" />
      </radialGradient>
      <clipPath id="lh-ground"><rect y="300" width="640" height="140" /></clipPath>
    </defs>

    <!-- Viewport ground grid and a soft pool of light -->
    <ellipse :cx="CX" cy="250" rx="300" ry="220" fill="url(#lh-pool)" />
    <g clip-path="url(#lh-ground)" stroke="#ffffff" stroke-opacity="0.07" stroke-width="1">
      <line v-for="y in gridRows" :key="`r${y}`" x1="0" :y1="y" x2="640" :y2="y" />
      <line v-for="x in gridCols" :key="`c${x}`" :x1="CX" y1="230" :x2="x" y2="440" />
    </g>
    <ellipse :cx="CX" :cy="BASE_Y + 3" rx="112" ry="22" fill="#000000" fill-opacity="0.35" />

    <!-- Scatter: rocks behind the tower -->
    <g v-if="step >= 5">
      <g v-for="(rock, i) in backRocks" :key="`b${i}`" :transform="`translate(${rock.x} ${rock.y}) scale(${rock.s})`">
        <polygon :points="rockBody" fill="#6f7279" v-bind="outline(5)" vector-effect="non-scaling-stroke" />
        <polygon :points="rockTop" fill="#a2a4aa" />
      </g>
    </g>

    <!-- Tower: Cylinder, then Taper -->
    <path :d="tower" fill="url(#lh-clay)" v-bind="step <= 1 ? outline(step) : outline(-1)" stroke-linejoin="round" />
    <ellipse v-if="step < 2" :cx="CX" :cy="TOP_Y" :rx="topR" :ry="topR * SQUASH" fill="#e3e4e7" />

    <!-- Array: spiral windows -->
    <g v-if="step >= 3">
      <rect v-for="(w, i) in windows" :key="`w${i}`" :x="CX + w.dx - 5" :y="w.y - 10" width="10" height="20" rx="5" fill="#1c1e23" v-bind="outline(3)" />
      <line v-for="(w, i) in windows" :key="`s${i}`" :x1="CX + w.dx - 7" :y1="w.y + 11" :x2="CX + w.dx + 7" :y2="w.y + 11" stroke="#e6e7ea" stroke-width="2" stroke-linecap="round" />
    </g>

    <!-- Boolean: doorway -->
    <path v-if="step >= 4" :d="door" fill="#16171b" v-bind="outline(4)" />

    <!-- Extrude: gallery deck and lantern room -->
    <g v-if="step >= 2">
      <path :d="deckBand" fill="url(#lh-clay)" />
      <ellipse :cx="CX" :cy="TOP_Y" :rx="DECK_R" :ry="DECK_RY" fill="#e6e7ea" />
      <path :d="lantern" fill="#2a2d34" />
      <g stroke="#8f939b" stroke-width="1.5">
        <line :x1="CX - 12" :y1="lanternTop" :x2="CX - 12" :y2="TOP_Y + 4.5" />
        <line :x1="CX" :y1="lanternTop" :x2="CX" :y2="TOP_Y + 5.5" />
        <line :x1="CX + 12" :y1="lanternTop" :x2="CX + 12" :y2="TOP_Y + 4.5" />
      </g>
      <path :d="dome" fill="url(#lh-clay-small)" />
      <line :x1="CX" :y1="lanternTop - 19" :x2="CX" :y2="lanternTop - 30" stroke="#c9cbd0" stroke-width="2" stroke-linecap="round" />
      <circle :cx="CX" :cy="lanternTop - 31" r="2.5" fill="#c9cbd0" />
      <path :d="rail" fill="none" stroke="#c3c5ca" stroke-width="1.5" />
      <line v-for="p in posts" :key="p.x" :x1="p.x" :y1="p.y1" :x2="p.x" :y2="p.y2" stroke="#c3c5ca" stroke-width="1.5" />
      <!-- Accent outline drawn last so it sits over the rail -->
      <g v-if="step === 2" fill="none" :stroke="ACCENT" stroke-width="2" stroke-linejoin="round">
        <path :d="deckBand" />
        <path :d="lantern" />
        <path :d="dome" />
      </g>
    </g>

    <!-- Scatter: rocks in front of the tower -->
    <g v-if="step >= 5">
      <g v-for="(rock, i) in frontRocks" :key="`f${i}`" :transform="`translate(${rock.x} ${rock.y}) scale(${rock.s})`">
        <polygon :points="rockBody" fill="#74777e" v-bind="outline(5)" vector-effect="non-scaling-stroke" />
        <polygon :points="rockTop" fill="#a9abb1" />
      </g>
    </g>

    <!-- Output: cleaned-up topology shown as a wireframe -->
    <g v-if="step >= 6" fill="none" :stroke="ACCENT" stroke-opacity="0.6" stroke-width="1">
      <path v-for="d in wireRings" :key="d" :d="d" />
      <path v-for="d in wireSpokes" :key="d" :d="d" />
    </g>
  </svg>
</template>
