<script setup lang="ts">
/**
 * Original illustrated level used in the conceptual editor viewport.
 * Decorative: the surrounding stage carries the accessible description.
 */
import type { StageStep } from '../content'

defineProps<{ step: StageStep }>()

const stars = [
  [64, 52, 1.3], [138, 112, 0.9], [212, 38, 1.1], [296, 86, 0.8], [372, 30, 1.4], [452, 104, 0.9],
  [520, 48, 1.1], [598, 120, 0.8], [808, 64, 1.2], [884, 128, 0.9], [918, 40, 1.1], [30, 168, 0.8],
] as const
</script>

<template>
  <svg viewBox="0 0 960 600" preserveAspectRatio="xMidYMid slice" aria-hidden="true" focusable="false" class="block size-full">
    <defs>
      <linearGradient id="stage-sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#18132e" />
        <stop offset="0.52" stop-color="#352657" />
        <stop offset="0.78" stop-color="#8c5470" />
        <stop offset="1" stop-color="#e3a273" />
      </linearGradient>
      <linearGradient id="stage-mist" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#d79a86" stop-opacity="0" />
        <stop offset="1" stop-color="#d79a86" stop-opacity="0.55" />
      </linearGradient>
      <radialGradient id="stage-moon" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0.45" stop-color="#f4e7cf" stop-opacity="0.16" />
        <stop offset="1" stop-color="#f4e7cf" stop-opacity="0" />
      </radialGradient>
      <radialGradient id="stage-lamp" cx="0.5" cy="0.5" r="0.5">
        <stop offset="0" stop-color="#f2b679" stop-opacity="0.55" />
        <stop offset="1" stop-color="#f2b679" stop-opacity="0" />
      </radialGradient>
      <g id="stage-wanderer" transform="scale(1.2)">
        <path d="M-7 -32C-18 -37 -29 -30 -38 -37C-32 -25 -19 -25 -6 -26Z" fill="#e8834f" />
        <path d="M-14 0H14L8 -34Q0 -41 -8 -34Z" fill="#efe6d4" />
        <path d="M-14 0H14L12 -9H-12Z" fill="#d8ccb6" />
        <circle cy="-43" r="9.5" fill="#efe6d4" />
        <ellipse cx="3.2" cy="-42.5" rx="4.8" ry="3.4" fill="#1d1828" />
        <rect x="7" y="-25" width="13" height="5" rx="2" fill="#8a829b" transform="rotate(-28 7 -25)" />
      </g>
    </defs>

    <!-- Sky, moon, stars -->
    <rect width="960" height="600" fill="url(#stage-sky)" />
    <circle cx="800" cy="140" r="110" fill="url(#stage-moon)" />
    <circle cx="800" cy="140" r="40" fill="#f4e7cf" />
    <circle cx="788" cy="132" r="7" fill="#e6d6ba" />
    <circle cx="814" cy="152" r="5" fill="#e6d6ba" />
    <g fill="#f4e7cf">
      <circle v-for="([x, y, r], i) in stars" :key="i" :cx="x" :cy="y" :r="r" :opacity="0.4 + (i % 3) * 0.2" />
    </g>

    <!-- Distant spires and ridges, atmospheric perspective -->
    <path d="M0 392L60 360 96 372 150 318 168 326 190 268 206 330 250 352 318 330 380 356 446 338 520 362 580 340 612 300 626 344 690 352 760 330 830 350 900 332 960 346V600H0Z" fill="#5b4078" />
    <path d="M0 424L80 400 160 414 240 392 330 418 420 398 520 420 610 402 700 424 790 404 880 420 960 406V600H0Z" fill="#432f63" />
    <rect y="408" width="960" height="192" fill="url(#stage-mist)" />

    <!-- Lighthouse headland -->
    <path d="M770 600V372C800 360 840 352 880 356C920 360 944 366 960 372V600Z" fill="#251b3c" />
    <circle cx="861" cy="286" r="34" fill="url(#stage-lamp)" />
    <rect x="852" y="292" width="18" height="70" fill="#33264f" />
    <path d="M849 281H873L861 272Z" fill="#33264f" />
    <rect x="850" y="281" width="22" height="11" rx="1" fill="#f2b679" />

    <!-- Floating isle with grapple anchor -->
    <path d="M552 296H730L716 318 690 324 670 352 650 342 628 378 606 336 584 330 566 316Z" fill="#1d1630" />
    <path d="M548 292C590 284 690 284 734 292V300H548Z" fill="#3d7c70" />
    <path d="M548 292C590 286 690 286 734 292" stroke="#6bb3a0" stroke-width="2" fill="none" />
    <path d="M586 292V246H560" stroke="#6f6780" stroke-width="5" stroke-linecap="round" fill="none" />
    <circle cx="560" cy="262" r="11" fill="none" stroke="#d9a86b" stroke-width="4" />

    <!-- Foreground cliff -->
    <path d="M0 404C70 398 180 396 300 400C330 401 362 404 384 410L376 450 392 486 370 530 384 600H0Z" fill="#120e1c" />
    <path d="M0 400C90 393 220 392 300 396C334 398 362 401 386 408L384 418C350 411 300 407 240 406C150 405 60 409 0 414Z" fill="#3d7c70" />
    <path d="M0 400C90 393 220 392 300 396C334 398 362 401 386 408" stroke="#6bb3a0" stroke-width="2.5" fill="none" />
    <path d="M70 398c3-16 8-24 12-24s5 12 5 24zM328 400c2-11 6-16 9-16s4 9 4 16z" fill="#2f6559" />

    <!-- Review: predicted path -->
    <g v-if="step === 'review'">
      <path d="M272 366Q410 196 560 262" fill="none" stroke="#a998ff" stroke-width="2.5" stroke-dasharray="8 6" stroke-linecap="round" class="motion-safe:animate-march" />
      <circle cx="560" cy="262" r="22" fill="none" stroke="#a998ff" stroke-width="2" stroke-dasharray="4 5" />
    </g>

    <!-- Play-test: rope and onion-skin frames along the swing -->
    <g v-if="step === 'play'">
      <path d="M272 366Q410 196 560 262" fill="none" stroke="#efe6d4" stroke-width="1.5" opacity="0.35" />
      <use href="#stage-wanderer" x="250" y="402" opacity="0.14" />
      <use href="#stage-wanderer" x="331" y="324" opacity="0.22" />
      <use href="#stage-wanderer" x="419" y="284" opacity="0.34" />
      <use href="#stage-wanderer" x="493" y="284" opacity="0.5" />
      <use href="#stage-wanderer" x="680" y="292" />
    </g>

    <!-- The wanderer at the cliff edge -->
    <use v-if="step !== 'play'" href="#stage-wanderer" x="250" y="402" />

    <!-- Selection and move gizmo -->
    <g v-if="step === 'ask' || step === 'review'">
      <rect x="226" y="332" width="48" height="74" fill="none" stroke="#a998ff" stroke-width="1.5" />
      <g fill="#a998ff">
        <rect x="223" y="329" width="6" height="6" />
        <rect x="271" y="329" width="6" height="6" />
        <rect x="223" y="403" width="6" height="6" />
        <rect x="271" y="403" width="6" height="6" />
      </g>
      <path d="M250 402H306" stroke="#f38b86" stroke-width="2.5" />
      <path d="M306 396l11 6-11 6z" fill="#f38b86" />
      <path d="M250 402V318" stroke="#86dcbc" stroke-width="2.5" />
      <path d="M244 318l6-11 6 11z" fill="#86dcbc" />
      <rect x="246" y="398" width="8" height="8" fill="#ebe7f2" />
    </g>
  </svg>
</template>
