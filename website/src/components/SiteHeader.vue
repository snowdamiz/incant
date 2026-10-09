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
    class="fixed inset-x-0 top-0 z-50 border-b transition-colors duration-200"
    :class="open ? 'border-rule bg-paper' : scrolled ? 'border-rule bg-paper/90 backdrop-blur-md' : 'border-transparent bg-paper/0'"
  >
    <nav aria-label="Primary" class="mx-auto flex h-16 max-w-[82rem] items-center justify-between gap-6 px-5 sm:px-8 lg:px-12">
      <a href="#top" class="flex items-center gap-2.5 py-1 pr-2" @click="close()">
        <WispMark class="h-7 w-auto" />
        <span class="font-display-soft text-[22px] leading-none font-[500] tracking-[-0.01em]">Incant</span>
      </a>

      <ul class="hidden items-center gap-8 md:flex">
        <li v-for="item in navItems" :key="item.href">
          <a :href="item.href" class="text-[14.5px] text-muted transition-colors hover:text-ink">{{ item.label }}</a>
        </li>
        <li>
          <a :href="links.repository" class="inline-flex items-center gap-1.5 text-[14.5px] font-medium text-ink">
            GitHub
            <svg viewBox="0 0 16 16" class="size-3" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M5 11l6-6M6 5h5v5" /></svg>
            <span class="sr-only">(Incant repository, may require access)</span>
          </a>
        </li>
      </ul>

      <button
        ref="toggle"
        type="button"
        class="-mr-2 inline-flex size-11 items-center justify-center rounded-md text-ink md:hidden"
        :aria-expanded="open"
        aria-controls="mobile-menu"
        @click="open = !open"
      >
        <span class="sr-only">{{ open ? 'Close menu' : 'Open menu' }}</span>
        <svg viewBox="0 0 24 24" class="size-6" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
          <path v-if="!open" d="M4 9h16M4 15h16" />
          <path v-else d="M6 6l12 12M18 6 6 18" />
        </svg>
      </button>
    </nav>

    <div v-show="open" id="mobile-menu" ref="panel" class="border-t border-rule px-5 pt-2 pb-8 sm:px-8 md:hidden">
      <ul>
        <li v-for="item in navItems" :key="item.href" class="border-b border-rule">
          <a :href="item.href" class="font-display-soft block py-4 text-[26px] font-[420] tracking-[-0.015em]" @click="close()">
            {{ item.label }}
          </a>
        </li>
      </ul>
      <a
        :href="links.repository"
        class="mt-6 inline-flex items-center gap-2 rounded-md bg-ink px-5 py-3 text-[15px] font-medium text-paper"
        @click="close()"
      >
        Open the repository on GitHub
      </a>
    </div>
  </header>
</template>
