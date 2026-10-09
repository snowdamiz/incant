<script setup lang="ts">
import GameScene from './GameScene.vue'
import WispMark from './WispMark.vue'

const hierarchy = [
  { name: 'Sky Harbor', depth: 0, kind: 'scene' },
  { name: 'Player', depth: 1, kind: 'selected' },
  { name: 'GrappleHook', depth: 2, kind: 'new' },
  { name: 'Camera', depth: 2, kind: 'plain' },
  { name: 'West Isle', depth: 1, kind: 'plain' },
  { name: 'High Ledge', depth: 1, kind: 'plain' },
  { name: 'Crystal Anchor', depth: 2, kind: 'plain' },
  { name: 'Dusk Light', depth: 1, kind: 'plain' },
] as const

const commands = [
  ['add_component', 'Player · GrappleHook'],
  ['set_field', 'GrappleHook.range'],
  ['create_script', 'grapple.ts'],
  ['bind_input', 'Grapple → Fire 2'],
] as const
</script>

<template>
  <figure class="relative">
    <div
      role="img"
      aria-label="Conceptual illustration of the Incant editor, not a screenshot. A scene hierarchy lists a player with a newly added grappling hook. The viewport shows a character on a floating island with a grapple line reaching a crystal on a higher ledge. An agent panel shows the request to add a grappling hook, the agent's reply after play-testing it, and a four-command transaction that can be kept or undone."
      class="overflow-hidden rounded-2xl border border-white/10 bg-ink-900/90 shadow-[0_40px_120px_-30px_rgba(75,43,199,0.65),0_0_0_1px_rgba(255,255,255,0.03)_inset] backdrop-blur"
    >
      <!-- Window chrome -->
      <div class="flex items-center gap-3 border-b border-white/[0.07] bg-ink-850 px-4 py-2.5">
        <div class="flex gap-1.5" aria-hidden="true">
          <span class="size-2.5 rounded-full bg-[#ff6b7f]/80" />
          <span class="size-2.5 rounded-full bg-[#ffc96b]/80" />
          <span class="size-2.5 rounded-full bg-[#5ee6c4]/80" />
        </div>
        <div class="flex min-w-0 items-center gap-2 font-mono text-[11px] text-ink-300">
          <WispMark class="h-3.5 w-auto shrink-0" body="#8b78ff" />
          <span class="truncate">sky-harbor <span class="text-ink-400">/</span> harbor.scene</span>
        </div>
        <div class="ml-auto hidden items-center gap-2 font-mono text-[10px] sm:flex">
          <span class="rounded-md border border-white/10 px-2 py-0.5 text-ink-300">Edit</span>
          <span class="rounded-md bg-spell-400/15 px-2 py-0.5 text-spell-300">&#9654; Play</span>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-[minmax(0,1fr)_17rem] lg:grid-cols-[11.5rem_minmax(0,1fr)_19rem]">
        <!-- Hierarchy -->
        <div class="hidden border-r border-white/[0.07] bg-ink-900 p-3 lg:block">
          <p class="mb-2 px-1 font-mono text-[10px] tracking-[0.14em] text-ink-400 uppercase">Hierarchy</p>
          <ul class="space-y-0.5 text-[12px]">
            <li
              v-for="node in hierarchy"
              :key="node.name"
              class="flex items-center gap-1.5 rounded-md py-1 pr-1"
              :class="[
                node.kind === 'selected' ? 'bg-wisp-500/25 text-white' : 'text-ink-300',
                node.kind === 'new' ? 'text-spell-300' : '',
              ]"
              :style="{ paddingLeft: `${0.35 + node.depth * 0.8}rem` }"
            >
              <span
                class="size-1.5 shrink-0 rounded-[2px]"
                :class="
                  node.kind === 'scene'
                    ? 'bg-ember-400'
                    : node.kind === 'new'
                      ? 'bg-spell-400'
                      : node.kind === 'selected'
                        ? 'bg-wisp-300'
                        : 'bg-ink-400'
                "
              />
              <span class="truncate">{{ node.name }}</span>
              <span v-if="node.kind === 'new'" class="ml-auto rounded bg-spell-400/15 px-1 font-mono text-[9px] text-spell-300">new</span>
            </li>
          </ul>
          <div class="mt-5 rounded-lg border border-white/[0.07] bg-ink-850 p-2.5">
            <p class="font-mono text-[10px] tracking-[0.14em] text-ink-400 uppercase">GrappleHook</p>
            <dl class="mt-2 space-y-1.5 font-mono text-[11px]">
              <div class="flex justify-between"><dt class="text-ink-400">range</dt><dd class="text-ember-300">18.0</dd></div>
              <div class="flex justify-between"><dt class="text-ink-400">pull</dt><dd class="text-ember-300">ease_out</dd></div>
              <div class="flex justify-between"><dt class="text-ink-400">input</dt><dd class="text-ember-300">Fire 2</dd></div>
            </dl>
          </div>
        </div>

        <!-- Viewport -->
        <div class="relative flex flex-col bg-ink-950">
          <div class="relative aspect-[16/10] w-full overflow-hidden md:aspect-auto md:min-h-[21rem] md:flex-1">
            <GameScene uid="hero" class="absolute inset-0" />
            <div class="absolute top-3 left-3 flex gap-1.5 font-mono text-[10px]">
              <span class="rounded-md bg-ink-950/70 px-2 py-1 text-ink-200 backdrop-blur">Perspective</span>
              <span class="hidden rounded-md bg-ink-950/70 px-2 py-1 text-ink-200 backdrop-blur sm:inline">Lit</span>
            </div>
            <div class="absolute right-3 bottom-3 rounded-md bg-ink-950/75 px-2 py-1 font-mono text-[10px] text-spell-300 backdrop-blur">
              &#10003; Player reached High Ledge
            </div>
          </div>
          <div class="flex items-center gap-3 border-t border-white/[0.07] bg-ink-900 px-3 py-2 font-mono text-[10.5px] text-ink-300">
            <span class="rounded bg-wisp-500/20 px-1.5 py-0.5 text-wisp-200">agent</span>
            <span class="truncate">Transaction applied · 4 commands · validated</span>
            <span class="ml-auto hidden text-ink-400 sm:inline">&#8984;Z to undo</span>
          </div>
        </div>

        <!-- Agent panel -->
        <div class="flex flex-col gap-3 border-t border-white/[0.07] bg-ink-900 p-3.5 md:border-t-0 md:border-l">
          <p class="font-mono text-[10px] tracking-[0.14em] text-ink-400 uppercase">Agent</p>
          <div class="ml-5 rounded-xl rounded-tr-sm bg-wisp-600 px-3 py-2.5 text-[12.5px] leading-snug text-white">
            Add a grappling hook that pulls the player toward the hit point. Test it and show me.
          </div>
          <div class="flex gap-2">
            <WispMark class="mt-0.5 h-5 w-auto shrink-0" body="#8b78ff" />
            <p class="text-[12.5px] leading-snug text-ink-200">
              Added <span class="font-mono text-[11.5px] text-spell-300">GrappleHook</span> to Player and bound it to Fire 2.
              In a play-test the player reached High Ledge in one pull.
            </p>
          </div>
          <div class="rounded-xl border border-white/[0.08] bg-ink-850 p-3">
            <div class="flex items-center justify-between">
              <p class="text-[11px] font-semibold text-white">Transaction</p>
              <p class="font-mono text-[10px] text-ink-400">4 commands</p>
            </div>
            <ul class="mt-2 space-y-1 font-mono text-[10.5px]">
              <li v-for="[verb, target] in commands" :key="verb" class="flex gap-2">
                <span class="text-spell-300">+</span>
                <span class="text-wisp-300">{{ verb }}</span>
                <span class="truncate text-ink-300">{{ target }}</span>
              </li>
            </ul>
            <div class="mt-3 flex gap-2 text-[11px] font-semibold" aria-hidden="true">
              <span class="flex-1 rounded-lg bg-white py-1.5 text-center text-ink-950">Keep</span>
              <span class="flex-1 rounded-lg border border-white/15 py-1.5 text-center text-ink-100">Undo all</span>
            </div>
          </div>
        </div>
      </div>
    </div>
    <figcaption class="mt-4 text-center text-xs text-ink-400">
      Conceptual product illustration.
    </figcaption>
  </figure>
</template>
