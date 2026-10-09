<script setup lang="ts">
import FeatureCard from './FeatureCard.vue'
import GameScene from './GameScene.vue'
import WispMark from './WispMark.vue'

const inputs = [
  { label: 'Mouse', dot: 'bg-ember-400' },
  { label: 'Keyboard', dot: 'bg-ember-400' },
  { label: 'Script', dot: 'bg-spell-400' },
  { label: 'Agent', dot: 'bg-wisp-400' },
] as const

const history = [
  { who: 'You', tone: 'bg-ember-400/15 text-ember-300', what: 'Move West Isle' },
  { who: 'Agent', tone: 'bg-wisp-500/25 text-wisp-200', what: 'Add GrappleHook' },
  { who: 'Script', tone: 'bg-spell-400/15 text-spell-300', what: 'Rebuild nav mesh' },
  { who: 'You', tone: 'bg-ember-400/15 text-ember-300', what: 'Tune pull speed' },
] as const
</script>

<template>
  <section id="features" aria-labelledby="features-title" class="relative pt-12 pb-24 sm:pt-16 sm:pb-32">
    <div class="mx-auto max-w-7xl px-5 sm:px-8">
      <div class="max-w-3xl">
        <p class="font-mono text-xs font-medium tracking-[0.2em] text-ember-300 uppercase">Features</p>
        <h2 id="features-title" class="text-balance-safe mt-4 text-4xl font-semibold tracking-[-0.035em] text-white sm:text-5xl">
          Two authors. One source of truth.
        </h2>
        <p class="mt-5 max-w-2xl text-lg leading-relaxed text-ink-300">
          Incant treats you and the agent as equals on the same project. Same commands, same history,
          same undo. Nothing happens behind a curtain.
        </p>
      </div>

      <div class="mt-14 grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-6 lg:gap-5">
        <FeatureCard class="md:col-span-2 lg:col-span-4" eyebrow="Command bus" title="Every hand on the scene speaks the same language.">
          A click, a shortcut, a script and an agent request all become the same typed command.
          There is no GUI-only path and no AI-only path, so whatever one author does, the other can see.
          <template #art>
            <div aria-hidden="true" class="grid h-full min-h-44 grid-cols-[auto_1fr_auto] items-center gap-3 sm:gap-5">
              <ul class="space-y-2.5">
                <li
                  v-for="input in inputs"
                  :key="input.label"
                  class="flex items-center gap-2 rounded-full border border-white/10 bg-ink-950/60 px-3 py-1.5 text-[12px] font-medium text-ink-200 sm:text-[13px]"
                >
                  <span class="size-1.5 rounded-full" :class="input.dot" />{{ input.label }}
                </li>
              </ul>
              <svg viewBox="0 0 200 160" preserveAspectRatio="none" class="h-40 w-full" fill="none">
                <g stroke-width="1.5" stroke-dasharray="4 4" class="motion-safe:animate-flow">
                  <path d="M0 20C80 20 100 80 200 80" stroke="#ffb86b" />
                  <path d="M0 60C80 60 110 80 200 80" stroke="#ffb86b" />
                  <path d="M0 100C80 100 110 80 200 80" stroke="#5ee6c4" />
                  <path d="M0 140C80 140 100 80 200 80" stroke="#8b78ff" />
                </g>
              </svg>
              <div class="rounded-2xl border border-wisp-400/40 bg-wisp-600/20 p-3 shadow-[0_0_40px_-8px_rgba(98,70,229,0.8)] sm:p-4">
                <p class="font-mono text-[10px] tracking-[0.16em] text-wisp-200 uppercase">command</p>
                <p class="mt-1 font-mono text-[12px] text-white sm:text-[13px]">set_field</p>
                <p class="mt-2 hidden font-mono text-[11px] text-ink-300 sm:block">apply · invert · describe</p>
              </div>
            </div>
          </template>
        </FeatureCard>

        <FeatureCard class="lg:col-span-2" eyebrow="Text-native" title="Your game is a document you can read.">
          Scenes, prefabs and materials are typed, schema-validated text with stable IDs. Diffs stay small and merges stay sane.
          <template #art>
            <pre
              aria-hidden="true"
              class="overflow-hidden rounded-2xl border border-white/[0.07] bg-ink-950/70 p-4 font-mono text-[11.5px] leading-[1.7] text-ink-300"
            ><span class="text-wisp-300">Entity</span>(
  id: <span class="text-ember-300">"01JB8Q…4M"</span>,
  name: <span class="text-ember-300">"Player"</span>,
  components: [
<span class="block bg-spell-400/10 text-spell-300">+   GrappleHook(
+     range: 18.0,
+     pull: EaseOut,
+   ),</span>  ],
)</pre>
          </template>
        </FeatureCard>

        <FeatureCard class="lg:col-span-2" eyebrow="Shared senses" title="The agent sees what you see.">
          Viewport captures, the console, play-test results and validation errors are all tools the agent can use. It checks its own work.
          <template #art>
            <div aria-hidden="true" class="overflow-hidden rounded-2xl border border-white/[0.07] bg-ink-950/70">
              <div class="relative aspect-[16/8]">
                <GameScene uid="senses" :gizmo="false" class="absolute inset-0" />
                <span class="absolute top-2 left-2 rounded bg-ink-950/75 px-1.5 py-0.5 font-mono text-[10px] text-ink-200">capture</span>
              </div>
              <div class="space-y-1 p-3 font-mono text-[10.5px]">
                <p class="text-ink-400">[play] grapple fired at Crystal Anchor</p>
                <p class="text-spell-300">&#10003; player reached High Ledge</p>
              </div>
            </div>
          </template>
        </FeatureCard>

        <FeatureCard class="lg:col-span-2" eyebrow="History" title="Every action can be undone.">
          Agent changes are atomic transactions in the same history as yours, tagged with where they came from.
          <template #art>
            <ol aria-hidden="true" class="space-y-2">
              <li
                v-for="(item, i) in history"
                :key="i"
                class="flex items-center gap-3 rounded-xl border border-white/[0.06] bg-ink-950/60 px-3 py-2"
                :class="i === 1 ? 'border-wisp-400/40 bg-wisp-600/10' : ''"
              >
                <span class="w-14 rounded-md px-1.5 py-0.5 text-center text-[11px] font-semibold" :class="item.tone">{{ item.who }}</span>
                <span class="truncate text-[13px] text-ink-200">{{ item.what }}</span>
                <span v-if="i === 1" class="ml-auto shrink-0 font-mono text-[11px] whitespace-nowrap text-wisp-200">&#8630; undo</span>
              </li>
            </ol>
          </template>
        </FeatureCard>

        <FeatureCard class="lg:col-span-2" eyebrow="Your model" title="Your keys never leave your machine.">
          Connect your own model provider. Calls go straight from your computer to the provider, and credentials live in your OS keychain.
          <template #art>
            <div aria-hidden="true" class="flex h-full min-h-40 items-center justify-between gap-1 rounded-2xl border border-white/[0.07] bg-ink-950/60 px-3 py-6 sm:gap-2 sm:px-4">
              <div class="flex flex-col items-center gap-2">
                <div class="grid size-14 place-items-center rounded-2xl bg-wisp-600/30 ring-1 ring-wisp-400/40">
                  <WispMark class="h-8 w-auto" body="#8b78ff" />
                </div>
                <span class="text-[11px] text-ink-300">Your machine</span>
              </div>
              <svg viewBox="0 0 120 24" class="h-6 min-w-[4.5rem] flex-1" fill="none">
                <path d="M4 12h112" stroke="#5ee6c4" stroke-width="1.5" stroke-dasharray="4 5" class="motion-safe:animate-flow" />
                <rect x="48" y="2" width="24" height="20" rx="6" fill="#0d0b18" stroke="#5ee6c4" />
                <path d="M56 11v-2a4 4 0 0 1 8 0v2" stroke="#9af2da" stroke-width="1.5" />
                <rect x="54" y="11" width="12" height="7" rx="1.5" fill="#9af2da" />
              </svg>
              <div class="flex flex-col items-center gap-2">
                <div class="grid size-14 place-items-center rounded-2xl bg-ink-800 ring-1 ring-white/10">
                  <svg viewBox="0 0 24 24" class="size-7 text-ink-200" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9z" />
                    <path d="M12 12l7.8-4.5M12 12v9M12 12 4.2 7.5" />
                  </svg>
                </div>
                <span class="text-[11px] text-ink-300">Model provider</span>
              </div>
            </div>
          </template>
        </FeatureCard>

        <!-- Wide scripting story -->
        <article class="relative overflow-hidden rounded-3xl border border-white/[0.08] bg-ink-850 md:col-span-2 lg:col-span-6">
          <div aria-hidden="true" class="absolute inset-0 bg-[radial-gradient(60%_80%_at_100%_0%,rgba(98,70,229,0.28),transparent)]" />
          <div class="relative grid grid-cols-1 items-center gap-8 p-6 sm:p-10 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)] lg:gap-14">
            <div>
              <p class="font-mono text-[11px] font-medium tracking-[0.16em] text-wisp-300 uppercase">Rust core · TypeScript gameplay</p>
              <h3 class="text-balance-safe mt-2 text-2xl font-semibold tracking-[-0.025em] text-white sm:text-3xl">
                Write gameplay in the language you already know.
              </h3>
              <p class="mt-4 max-w-xl text-[15px] leading-relaxed text-ink-300">
                A Rust engine core handles the heavy lifting. Gameplay is TypeScript with hot reload, running in a sandbox that
                has no file system or network access unless your project grants it. It is the language people and models write best.
              </p>
              <ul class="mt-6 flex flex-wrap gap-2 text-[13px] text-ink-200">
                <li class="rounded-full border border-white/10 px-3 py-1">Hot reload</li>
                <li class="rounded-full border border-white/10 px-3 py-1">Sandboxed scripts</li>
                <li class="rounded-full border border-white/10 px-3 py-1">Typed engine API</li>
              </ul>
            </div>
            <figure class="min-w-0">
              <pre
                class="overflow-x-auto rounded-2xl border border-white/[0.08] bg-ink-950/80 p-5 font-mono text-[12.5px] leading-[1.75] text-ink-300 sm:text-[13px]"
                tabindex="0"
                aria-label="Example TypeScript gameplay script for a grappling hook"
              ><span class="text-ink-400">// scripts/grapple.ts</span>
<span class="text-wisp-300">export default</span> <span class="text-ember-300">behavior</span>(<span class="text-spell-300">'GrappleHook'</span>, (hook, ctx) =&gt; {
  <span class="text-wisp-300">if</span> (!ctx.input.pressed(<span class="text-spell-300">'Grapple'</span>)) <span class="text-wisp-300">return</span>
  <span class="text-wisp-300">const</span> hit = ctx.physics.raycast(ctx.aim, hook.range)
  <span class="text-wisp-300">if</span> (hit) ctx.pull(hook.owner, hit.point, hook.pull)
})</pre>
              <figcaption class="mt-3 text-xs text-ink-400">Illustrative gameplay script.</figcaption>
            </figure>
          </div>
        </article>
      </div>
    </div>
  </section>
</template>
