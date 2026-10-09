<script setup lang="ts">
/**
 * Original illustrated dusk scene used inside the conceptual editor viewport.
 * Purely decorative: the surrounding figure carries the accessible description.
 */
withDefaults(defineProps<{ grapple?: boolean; gizmo?: boolean; uid?: string }>(), {
  grapple: true,
  gizmo: true,
  uid: 'scene',
})
</script>

<template>
  <svg viewBox="0 0 640 360" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false" class="block size-full">
    <defs>
      <linearGradient :id="`${uid}-sky`" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#140c33" />
        <stop offset="0.5" stop-color="#3b1d6e" />
        <stop offset="0.78" stop-color="#a2457a" />
        <stop offset="1" stop-color="#ffb070" />
      </linearGradient>
      <radialGradient :id="`${uid}-sun`" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#ffe2b8" />
        <stop offset="0.45" stop-color="#ffb86b" stop-opacity="0.85" />
        <stop offset="1" stop-color="#ff8a6b" stop-opacity="0" />
      </radialGradient>
      <linearGradient :id="`${uid}-sea`" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#3a1d5c" />
        <stop offset="1" stop-color="#0d0920" />
      </linearGradient>
      <linearGradient :id="`${uid}-rock`" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#3b2a6b" />
        <stop offset="1" stop-color="#140d2c" />
      </linearGradient>
      <radialGradient :id="`${uid}-crystal`" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#9af2da" stop-opacity="0.9" />
        <stop offset="1" stop-color="#5ee6c4" stop-opacity="0" />
      </radialGradient>
    </defs>

    <rect width="640" height="360" :fill="`url(#${uid}-sky)`" />
    <g fill="#fff">
      <circle cx="60" cy="40" r="1.2" opacity="0.7" />
      <circle cx="140" cy="78" r="0.9" opacity="0.5" />
      <circle cx="232" cy="28" r="1.4" opacity="0.8" />
      <circle cx="318" cy="64" r="0.8" opacity="0.5" />
      <circle cx="392" cy="22" r="1.1" opacity="0.7" />
      <circle cx="560" cy="48" r="1.3" opacity="0.6" />
      <circle cx="606" cy="96" r="0.9" opacity="0.5" />
      <circle cx="28" cy="120" r="0.9" opacity="0.4" />
    </g>

    <circle cx="330" cy="262" r="96" :fill="`url(#${uid}-sun)`" />
    <path d="M0 268 70 228 128 250 196 204 262 246 330 218 404 252 470 206 548 244 640 214V300H0Z" fill="#2a1650" opacity="0.85" />
    <path d="M0 284 92 256 170 276 258 248 352 278 440 254 526 280 640 258V300H0Z" fill="#1d1040" />

    <rect y="290" width="640" height="70" :fill="`url(#${uid}-sea)`" />
    <g stroke="#ffb86b" stroke-linecap="round" opacity="0.45">
      <path d="M300 302h60M314 312h34M290 324h28M338 324h40M322 338h22" stroke-width="2" />
    </g>

    <!-- Left island -->
    <g>
      <path d="M78 252h214l-26 30-40 10-32 34-22-26-44 12-30-34z" :fill="`url(#${uid}-rock)`" />
      <path d="M74 246c40-10 170-12 222 0v10H74z" fill="#5ee6c4" opacity="0.9" />
      <path d="M74 252h222v6H74z" fill="#2c8c7a" />
      <path d="M110 246c4-14 10-20 14-20s6 10 6 20zM262 246c3-10 7-14 10-14s5 8 5 14z" fill="#3fbf9f" />
    </g>

    <!-- High island with crystal -->
    <g>
      <path d="M402 150h148l-18 22-30 6-22 26-18-20-34 8-14-22z" :fill="`url(#${uid}-rock)`" />
      <path d="M398 146c34-8 120-8 156 0v8H398z" fill="#5ee6c4" opacity="0.9" />
      <circle cx="476" cy="118" r="34" :fill="`url(#${uid}-crystal)`" />
      <path d="M476 92l14 26-14 28-14-28z" fill="#9af2da" />
      <path d="M476 92l14 26-14 28z" fill="#5ee6c4" />
    </g>

    <!-- Grapple line and hit marker -->
    <g v-if="grapple">
      <path
        d="M214 214C280 150 380 112 466 122"
        fill="none"
        stroke="#ffd09c"
        stroke-width="2.5"
        stroke-dasharray="7 5"
        stroke-linecap="round"
        class="motion-safe:animate-flow"
      />
      <circle cx="466" cy="122" r="10" fill="none" stroke="#ffd09c" stroke-width="2" />
      <circle cx="466" cy="122" r="3" fill="#ffd09c" />
    </g>

    <!-- Hero character -->
    <g>
      <ellipse cx="200" cy="247" rx="20" ry="4" fill="#0d0920" opacity="0.5" />
      <rect x="188" y="208" width="24" height="38" rx="11" fill="#eeedf6" />
      <circle cx="200" cy="198" r="13" fill="#eeedf6" />
      <path d="M187 210c8 4 18 4 26 0l2 6c-4 2-6 4-6 10l-4-6c-6 2-12 1-18-3z" fill="#ffb86b" />
      <circle cx="205" cy="197" r="2.4" fill="#140c33" />
      <rect x="208" y="214" width="10" height="5" rx="2.5" fill="#d6d3e6" transform="rotate(-38 208 214)" />
    </g>

    <!-- Selection gizmo -->
    <g v-if="gizmo">
      <path
        d="M176 186v-8h8M216 178h8v8M224 242v8h-8M184 250h-8v-8"
        fill="none"
        stroke="#b3a8ff"
        stroke-width="2"
      />
      <path d="M200 228h46" stroke="#ff7a93" stroke-width="2.5" />
      <path d="M246 223l9 5-9 5z" fill="#ff7a93" />
      <path d="M200 228v-58" stroke="#5ee6c4" stroke-width="2.5" />
      <path d="M195 170l5-9 5 9z" fill="#5ee6c4" />
      <circle cx="200" cy="228" r="3.5" fill="#fff" />
    </g>
  </svg>
</template>
