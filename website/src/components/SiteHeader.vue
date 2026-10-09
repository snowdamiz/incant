<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import WispMark from './WispMark.vue'
import { links, navItems } from '../content'

const open = ref(false)
const scrolled = ref(false)
const toggle = ref<HTMLButtonElement | null>(null)
const panel = ref<HTMLElement | null>(null)

function close(returnFocus = false): void {
  if (!open.value) return
  open.value = false
  if (returnFocus) void nextTick(() => toggle.value?.focus())
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && open.value) {
    event.preventDefault()
    close(true)
  }
}

function onScroll(): void {
  scrolled.value = window.scrollY > 8
}

function onResize(): void {
  if (window.matchMedia('(min-width: 768px)').matches) close()
}

watch(open, async (isOpen) => {
  if (!isOpen) return
  await nextTick()
  panel.value?.querySelector<HTMLAnchorElement>('a')?.focus()
})

onMounted(() => {
  onScroll()
  window.addEventListener('scroll', onScroll, { passive: true })
  window.addEventListener('resize', onResize)
  document.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('scroll', onScroll)
  window.removeEventListener('resize', onResize)
  document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <header
    class="fixed inset-x-0 top-0 z-50 transition-colors duration-300"
    :class="
      open
        ? 'border-b border-white/[0.07] bg-ink-950 shadow-[0_24px_60px_-12px_rgba(0,0,0,0.8)]'
        : scrolled
          ? 'border-b border-white/[0.07] bg-ink-950/85 backdrop-blur-xl'
          : 'border-b border-transparent'
    "
  >
    <nav
      aria-label="Primary"
      class="mx-auto flex h-16 max-w-7xl items-center justify-between gap-4 px-5 sm:px-8"
    >
      <a
        href="#top"
        class="group flex items-center gap-2.5 rounded-lg py-1 pr-2"
        @click="close()"
      >
        <WispMark
          class="h-8 w-auto drop-shadow-[0_0_14px_rgba(98,70,229,0.55)] transition-transform duration-300 group-hover:-rotate-6"
          body="#6246e5"
        />
        <span class="text-[1.15rem] font-semibold tracking-[-0.02em] text-white">Incant</span>
      </a>

      <ul class="hidden items-center gap-1 md:flex">
        <li v-for="item in navItems" :key="item.href">
          <a
            :href="item.href"
            class="rounded-full px-3.5 py-2 text-sm font-medium text-ink-300 transition-colors hover:bg-white/[0.06] hover:text-white"
          >
            {{ item.label }}
          </a>
        </li>
      </ul>

      <div class="flex items-center gap-2">
        <a
          :href="links.repository"
          class="hidden items-center gap-2 rounded-full border border-white/12 bg-white/[0.04] px-4 py-2 text-sm font-semibold text-white transition-colors hover:border-wisp-400/60 hover:bg-wisp-500/15 sm:inline-flex"
        >
          <svg viewBox="0 0 16 16" class="size-4" aria-hidden="true" fill="currentColor">
            <path
              d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38v-1.34c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.05-.49.05-.49.8.06 1.23.83 1.23.83.72 1.22 1.87.87 2.33.66.07-.52.28-.87.5-1.07-1.78-.2-3.65-.89-3.65-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
            />
          </svg>
          GitHub
          <span class="sr-only">(Incant repository, may require access)</span>
        </a>

        <button
          ref="toggle"
          type="button"
          class="inline-flex size-11 items-center justify-center rounded-full border border-white/12 bg-white/[0.04] text-white md:hidden"
          :aria-expanded="open"
          aria-controls="mobile-menu"
          @click="open = !open"
        >
          <span class="sr-only">{{ open ? 'Close menu' : 'Open menu' }}</span>
          <svg viewBox="0 0 24 24" class="size-5" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path v-if="!open" d="M4 7h16M4 12h16M4 17h10" />
            <path v-else d="M6 6l12 12M18 6 6 18" />
          </svg>
        </button>
      </div>
    </nav>

    <div
      v-show="open"
      id="mobile-menu"
      ref="panel"
      class="border-t border-white/[0.07] px-5 pt-3 pb-6 md:hidden"
    >
      <ul class="flex flex-col">
        <li v-for="item in navItems" :key="item.href">
          <a
            :href="item.href"
            class="flex items-center justify-between rounded-xl px-3 py-3.5 text-base font-medium text-ink-100 hover:bg-white/[0.05]"
            @click="close()"
          >
            {{ item.label }}
            <span aria-hidden="true" class="text-ink-400">&rarr;</span>
          </a>
        </li>
        <li class="mt-3">
          <a
            :href="links.repository"
            class="flex items-center justify-center rounded-full bg-white px-4 py-3 text-base font-semibold text-ink-950"
            @click="close()"
          >
            Open the repository on GitHub
          </a>
        </li>
      </ul>
    </div>
  </header>
</template>
