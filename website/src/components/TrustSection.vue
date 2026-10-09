<script setup lang="ts">
const pillars = [
  {
    title: 'Offline by default',
    body: 'The editor works fully offline with no account. No editing feature sits behind a login.',
    icon: 'M4 12a8 8 0 1 0 16 0 8 8 0 1 0-16 0M12 8v4l3 2',
  },
  {
    title: 'Project content is data',
    body: 'Anything inside a project is shown to the agent as labeled data, never as instructions it must follow.',
    icon: 'M5 4h10l4 4v12H5zM15 4v4h4M8 12h8M8 16h5',
  },
  {
    title: 'No shell for the agent',
    body: 'The agent acts through engine commands and sandboxed scripts only. It cannot run commands on your computer.',
    icon: 'M4 5h16v14H4zM8 10l3 2-3 2M13 15h3',
  },
  {
    title: 'Grants you confirm',
    body: 'Scripts start with no file or network access. A project can request a capability, and you decide.',
    icon: 'M12 3l7 3v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6zM9 12l2 2 4-4',
  },
] as const
</script>

<template>
  <section id="trust" aria-labelledby="trust-title" class="relative py-24 sm:py-28">
    <div class="mx-auto max-w-7xl px-5 sm:px-8">
      <div class="grid gap-12 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] lg:gap-20">
        <div class="lg:sticky lg:top-28 lg:self-start">
          <p class="font-mono text-xs font-medium tracking-[0.2em] text-wisp-300 uppercase">Trust</p>
          <h2 id="trust-title" class="text-balance-safe mt-4 text-4xl font-semibold tracking-[-0.035em] text-white sm:text-5xl">
            Powerful magic, clear rules.
          </h2>
          <p class="mt-5 max-w-lg text-lg leading-relaxed text-ink-300">
            An agent that can change your game should be easy to supervise. Incant draws the boundaries into the engine itself,
            so you never have to take its work on faith.
          </p>
        </div>

        <ul class="grid gap-px overflow-hidden rounded-3xl border border-white/[0.08] bg-white/[0.08] sm:grid-cols-2">
          <li v-for="pillar in pillars" :key="pillar.title" class="bg-ink-950 p-7 sm:p-8">
            <span class="grid size-11 place-items-center rounded-xl bg-wisp-600/20 text-wisp-300 ring-1 ring-wisp-400/30">
              <svg viewBox="0 0 24 24" class="size-5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path :d="pillar.icon" />
              </svg>
            </span>
            <h3 class="mt-5 text-lg font-semibold tracking-[-0.015em] text-white">{{ pillar.title }}</h3>
            <p class="mt-2 text-[15px] leading-relaxed text-ink-300">{{ pillar.body }}</p>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>
