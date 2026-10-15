<script setup lang="ts">
import { principles } from '../content'

const history = [
  { label: 'Add a sweeping beam · 4 edits', who: 'Agent', tone: 'text-ed-agent', time: '17:42' },
  { label: 'Rename Keeper’s House', who: 'Teammate', tone: 'text-[#73d6c2]', time: '17:41' },
  { label: 'Set Transform on Lantern', who: 'You', tone: 'text-ed-user', time: '17:40' },
  { label: 'Import cliffs.glb', who: 'Import', tone: 'text-ed-import', time: '17:38' },
]

const frames = [-18, 24, 62]
</script>

<template>
  <section aria-labelledby="principles-title" class="border-t border-line py-24 sm:py-32">
    <div class="mx-auto max-w-[90rem] px-5 sm:px-8 lg:px-12">
      <div class="grid gap-8 lg:grid-cols-12 lg:items-end">
        <h2 id="principles-title" class="display text-balance-safe text-[2.5rem] leading-[1] sm:text-[3.5rem] lg:col-span-7 lg:text-[4rem]">
          Nothing happens behind your back.
        </h2>
        <p class="text-pretty-safe max-w-xl text-[17px] leading-relaxed text-fg-2 sm:text-lg lg:col-span-5 lg:pb-1">
          Three guarantees hold for every edit, whoever makes it: you, a teammate, a script or the agent.
        </p>
      </div>

      <ul class="mt-16 grid gap-5 md:grid-cols-3">
        <li v-for="p in principles" :key="p.id" class="flex min-w-0 flex-col">
          <!-- Artifact: what the guarantee looks like in the product. -->
          <div class="h-[14.5rem] overflow-hidden rounded-xl border border-line bg-panel font-mono text-[12px] text-ed-soft" aria-hidden="true">
            <template v-if="p.id === 'history'">
              <div class="flex h-10 items-center justify-between border-b border-line px-4 font-sans text-[13px]">
                <span class="font-medium text-ed-text">History</span><span class="text-ed-subtle">4 of 4 applied</span>
              </div>
              <ul class="px-4 py-2">
                <li v-for="h in history" :key="h.label" class="flex h-9 items-center gap-3 border-b border-line/60 last:border-0">
                  <span class="min-w-0 truncate font-sans text-[13px]">{{ h.label }}</span>
                  <span class="ml-auto shrink-0 font-sans text-[12.5px]" :class="h.tone">{{ h.who }}</span>
                  <span class="shrink-0 text-ed-subtle">{{ h.time }}</span>
                </li>
              </ul>
            </template>

            <template v-else-if="p.id === 'text'">
              <div class="flex h-10 items-center justify-between border-b border-line px-4 font-sans text-[13px]">
                <span class="font-medium text-ed-text">bay.scene</span>
                <span><span class="text-ed-success">+3</span> <span class="text-ed-subtle">−0</span></span>
              </div>
              <pre class="px-4 py-3 leading-[1.75]"><span class="text-ed-subtle">  "01JC8W3KQ9…": {</span>
<span class="text-ed-subtle">    "name": <span class="text-[#f2a983]">"Beam"</span>,</span>
<span class="-mx-4 block bg-ed-success/10 px-4"><span class="text-ed-success">+</span>   "parent": <span class="text-[#f2a983]">"01JC8W2F7M…"</span>,</span><span class="-mx-4 block bg-ed-success/10 px-4"><span class="text-ed-success">+</span>   "spot_light": { "angle": <span class="text-[#85b4ff]">6</span> },</span><span class="-mx-4 block bg-ed-success/10 px-4"><span class="text-ed-success">+</span>   "script": <span class="text-[#f2a983]">"sweep.ts"</span></span><span class="text-ed-subtle">  }</span></pre>
            </template>

            <template v-else>
              <div class="flex h-10 items-center justify-between border-b border-line px-4 font-sans text-[13px]">
                <span class="font-medium text-ed-text">Play-test · headless</span>
                <span class="inline-flex items-center gap-1.5 text-ed-success"><span class="size-1.5 rounded-full bg-ed-success" />Passed</span>
              </div>
              <ul class="space-y-2 px-4 pt-3.5 font-sans text-[13px]">
                <li class="flex items-center gap-2"><span class="text-ed-success">✓</span>Beam completes a turn every 8.0 s</li>
                <li class="flex items-center gap-2"><span class="text-ed-success">✓</span>Lantern stays lit after a reload</li>
                <li class="flex items-center gap-2"><span class="text-ed-success">✓</span>No new problems</li>
              </ul>
              <div class="mt-4 flex items-center gap-2 px-4">
                <span
                  v-for="angle in frames"
                  :key="angle"
                  class="relative h-11 w-16 overflow-hidden rounded-[4px] border border-line bg-[linear-gradient(to_bottom,#141a3a,#3a2a48_62%,#0b0d14_63%)]"
                >
                  <span class="absolute top-[34%] left-[58%] h-[2px] w-12 origin-left bg-[linear-gradient(to_right,#ffd29a,transparent)]" :style="{ transform: `rotate(${angle}deg)` }" />
                  <span class="absolute top-[30%] left-[55%] size-1.5 rounded-full bg-[#ffe1b0]" />
                </span>
                <span class="ml-1 text-ed-subtle">3 frames · 412 log lines</span>
              </div>
            </template>
          </div>

          <h3 class="mt-7 text-[20px] font-semibold tracking-[-0.02em]">{{ p.title }}</h3>
          <p class="text-pretty-safe mt-2 max-w-sm text-[16px] leading-relaxed text-muted">{{ p.body }}</p>
        </li>
      </ul>
    </div>
  </section>
</template>
