<script setup lang="ts">
/**
 * One small viewport drawing per content tool. Decorative SVG: the parent cell carries the
 * accessible description. Palette: editor clay greys, the editor accent, muted natural tones.
 */
import type { VignetteId } from '../content'

defineProps<{ id: VignetteId }>()

const ACCENT = '#8c95ff'

/* Retopology: wire sphere and UV islands. */
const SPHERE = { cx: 64, cy: 70, r: 38 }
const latitudes = [-26, -13, 0, 13, 26].map((dy) => {
  const rx = Math.sqrt(SPHERE.r ** 2 - dy ** 2)
  return { cy: SPHERE.cy + dy, rx: Math.round(rx * 10) / 10, ry: Math.round(rx * 2.8) / 10 }
})

/* LODs: one rock at three detail levels, each with a fan of interior edges. */
const lods = [
  { x: 40, pts: [[-26, 0], [-28, -10], [-22, -22], [-12, -30], [0, -33], [12, -29], [22, -24], [28, -12], [27, -2], [20, 4], [0, 6], [-16, 5]] },
  { x: 100, pts: [[-26, 0], [-24, -18], [-10, -31], [8, -31], [24, -20], [28, -4], [14, 5], [-14, 5]] },
  { x: 160, pts: [[-26, 2], [-18, -24], [10, -32], [28, -6], [6, 6]] },
].map(({ x, pts }) => ({
  x,
  points: pts.map(([px, py]) => `${x + px!},${96 + py!}`).join(' '),
  fan: pts.map(([px, py]) => `M${x} ${84}L${x + px!} ${96 + py!}`).join(''),
}))

/* Terrain trees. */
const trees = [
  [40, 99],
  [48, 101],
  [118, 92],
  [126, 94],
  [166, 97],
] as const

/* Rig keyframes. */
const keys = [30, 70, 112, 154] as const
</script>

<template>
  <svg viewBox="0 0 200 140" aria-hidden="true" focusable="false" class="block h-auto w-full">
    <!-- Retopology and UVs -->
    <g v-if="id === 'retopo'">
      <defs>
        <radialGradient id="tv-retopo-clay" cx="0.38" cy="0.32" r="0.75">
          <stop offset="0" stop-color="#e3e4e7" />
          <stop offset="0.55" stop-color="#a6a8ae" />
          <stop offset="1" stop-color="#5c5f66" />
        </radialGradient>
      </defs>
      <circle :cx="SPHERE.cx" :cy="SPHERE.cy" :r="SPHERE.r" fill="url(#tv-retopo-clay)" />
      <g fill="none" stroke="#0b0c0f" stroke-opacity="0.38" stroke-width="1">
        <ellipse v-for="l in latitudes" :key="l.cy" :cx="SPHERE.cx" :cy="l.cy" :rx="l.rx" :ry="l.ry" />
        <ellipse :cx="SPHERE.cx" :cy="SPHERE.cy" rx="13" :ry="SPHERE.r" />
        <ellipse :cx="SPHERE.cx" :cy="SPHERE.cy" rx="27" :ry="SPHERE.r" />
        <line :x1="SPHERE.cx" :y1="SPHERE.cy - SPHERE.r" :x2="SPHERE.cx" :y2="SPHERE.cy + SPHERE.r" />
      </g>
      <path d="M107 70h9m-3-3 3 3-3 3" fill="none" stroke="#676e7b" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
      <rect x="120" y="37" width="66" height="66" fill="none" stroke="#676e7b" stroke-dasharray="2 3" />
      <g :fill="ACCENT" fill-opacity="0.14" :stroke="ACCENT" stroke-width="1.25" stroke-linejoin="round">
        <rect x="126" y="43" width="24" height="18" rx="1" />
        <rect x="154" y="43" width="26" height="28" rx="1" />
        <rect x="126" y="65" width="24" height="32" rx="1" />
        <polygon points="154,75 180,75 180,97 166,97 154,87" />
      </g>
    </g>

    <!-- LODs -->
    <g v-else-if="id === 'lod'">
      <defs>
        <linearGradient id="tv-lod-clay" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stop-color="#a6a8ae" />
          <stop offset="0.4" stop-color="#d3d4d8" />
          <stop offset="1" stop-color="#5c5f66" />
        </linearGradient>
      </defs>
      <g v-for="(rock, i) in lods" :key="rock.x">
        <ellipse :cx="rock.x" cy="102" rx="28" ry="5" fill="#000000" fill-opacity="0.35" />
        <polygon :points="rock.points" fill="url(#tv-lod-clay)" stroke="#0b0c0f" stroke-opacity="0.4" stroke-linejoin="round" />
        <path :d="rock.fan" fill="none" stroke="#0b0c0f" stroke-opacity="0.28" />
        <circle :cx="rock.x" cy="120" r="2.5" :fill="i === 0 ? ACCENT : '#676e7b'" />
      </g>
    </g>

    <!-- Shader graph -->
    <g v-else-if="id === 'material'">
      <defs>
        <radialGradient id="tv-material-stone" cx="0.36" cy="0.3" r="0.78">
          <stop offset="0" stop-color="#e6dccd" />
          <stop offset="0.55" stop-color="#a8957f" />
          <stop offset="1" stop-color="#4b4239" />
        </radialGradient>
      </defs>
      <rect x="12" y="30" width="62" height="30" rx="5" fill="#1a1c21" stroke="#2e323a" />
      <rect x="18" y="36" width="18" height="18" rx="3" fill="#b8a690" />
      <path d="M42 41h24M42 49h17" stroke="#676e7b" stroke-width="2" stroke-linecap="round" />
      <rect x="12" y="80" width="62" height="30" rx="5" fill="#1a1c21" stroke="#2e323a" />
      <rect x="18" y="86" width="18" height="18" rx="3" fill="#2a2d34" />
      <g fill="#9a9ca3">
        <circle cx="22" cy="90" r="1.4" /><circle cx="29" cy="92" r="1.1" /><circle cx="33" cy="88" r="1.3" />
        <circle cx="24" cy="98" r="1.2" /><circle cx="31" cy="100" r="1.5" /><circle cx="21" cy="102" r="1" />
      </g>
      <path d="M42 91h24M42 99h17" stroke="#676e7b" stroke-width="2" stroke-linecap="round" />
      <path d="M74 45C98 45 96 70 114 70M74 95C98 95 96 72 114 72" fill="none" :stroke="ACCENT" stroke-width="1.5" />
      <circle cx="74" cy="45" r="3" :fill="ACCENT" />
      <circle cx="74" cy="95" r="3" :fill="ACCENT" />
      <circle cx="152" cy="71" r="36" fill="url(#tv-material-stone)" />
      <g fill="#4b4239" fill-opacity="0.45">
        <circle cx="140" cy="80" r="1.6" /><circle cx="160" cy="90" r="1.2" /><circle cx="168" cy="66" r="1.4" />
        <circle cx="146" cy="96" r="1.1" /><circle cx="172" cy="82" r="1.5" />
      </g>
    </g>

    <!-- Terrain -->
    <g v-else-if="id === 'terrain'">
      <path d="M0 78L22 66 44 72 70 44 92 58 112 50 140 64 166 54 200 70V140H0Z" fill="#3c4453" />
      <path d="M0 98L28 84 52 90 84 58 104 74 128 66 156 82 180 76 200 86V140H0Z" fill="#8b8f98" />
      <path d="M84 58L78 66H90Z" fill="#d3d4d8" />
      <path d="M0 108L30 96 56 100 82 84 106 92 130 86 158 96 182 92 200 98V140H0Z" fill="#55705b" />
      <path d="M0 124L40 118 80 122 120 116 160 121 200 117V140H0Z" fill="#a89a80" />
      <path v-for="([x, y], i) in trees" :key="i" :d="`M${x} ${y - 14}L${x - 5} ${y}H${x + 5}Z`" fill="#2f4a39" />
      <circle cx="102" cy="88" r="12" fill="none" :stroke="ACCENT" stroke-width="1.5" stroke-dasharray="3 3" />
      <circle cx="102" cy="88" r="1.5" :fill="ACCENT" />
    </g>

    <!-- Rig, IK and timeline -->
    <g v-else-if="id === 'rig'">
      <line x1="56" y1="106" x2="146" y2="106" stroke="#ffffff" stroke-opacity="0.15" />
      <g fill="none" stroke="#d3d4d8" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
        <path d="M100 70V44" />
        <path d="M100 47L86 58 80 72" />
        <path d="M100 47L114 57 122 67" />
        <path d="M100 70L90 88 80 104" />
        <path d="M100 70L110 88 116 104" />
      </g>
      <circle cx="100" cy="33" r="7" fill="none" stroke="#d3d4d8" stroke-width="2.5" />
      <g fill="#ececf1">
        <circle cx="100" cy="70" r="2.5" /><circle cx="100" cy="47" r="2.5" /><circle cx="86" cy="58" r="2.5" />
        <circle cx="114" cy="57" r="2.5" /><circle cx="90" cy="88" r="2.5" /><circle cx="110" cy="88" r="2.5" />
      </g>
      <circle cx="80" cy="104" r="6" fill="none" :stroke="ACCENT" stroke-width="1.5" />
      <path d="M71 104h4M85 104h4M80 95v4M80 109v4" :stroke="ACCENT" stroke-width="1.5" stroke-linecap="round" />
      <line x1="16" y1="126" x2="184" y2="126" stroke="#2e323a" stroke-width="2" stroke-linecap="round" />
      <rect
        v-for="k in keys"
        :key="k"
        :x="k - 3.5"
        y="122.5"
        width="7"
        height="7"
        :fill="k === 112 ? ACCENT : '#a9aeb9'"
        :transform="`rotate(45 ${k} 126)`"
      />
      <line x1="112" y1="114" x2="112" y2="136" :stroke="ACCENT" stroke-width="1.25" />
    </g>

    <!-- Image to 3D -->
    <g v-else>
      <rect x="14" y="36" width="64" height="64" rx="4" fill="#262a33" stroke="#2e323a" />
      <rect x="28" y="50" width="36" height="36" fill="#8d7a62" />
      <path d="M28 62H64M28 74H64M28 86L64 50" stroke="#6e5d49" stroke-width="2" />
      <path d="M86 68h18m-4-4 4 4-4 4" fill="none" stroke="#676e7b" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
      <ellipse cx="152" cy="106" rx="30" ry="6" fill="#000000" fill-opacity="0.35" />
      <polygon points="152,44 178,58 152,72 126,58" fill="#b8a690" />
      <polygon points="126,58 152,72 152,102 126,88" fill="#8d7a62" />
      <polygon points="152,72 178,58 178,88 152,102" fill="#6e5d49" />
      <path d="M126 68L152 82M126 78L152 92M152 82L178 68M152 92L178 78" stroke="#5a4b3a" stroke-width="1.5" />
      <g fill="none" :stroke="ACCENT" stroke-opacity="0.75" stroke-width="1" stroke-linejoin="round">
        <polygon points="152,44 178,58 152,72 126,58" />
        <polygon points="126,58 152,72 152,102 126,88" />
        <polygon points="152,72 178,58 178,88 152,102" />
      </g>
    </g>
  </svg>
</template>
