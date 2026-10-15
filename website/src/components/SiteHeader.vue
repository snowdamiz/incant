<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import WispMark from './WispMark.vue'
import ArrowIcon from './ArrowIcon.vue'
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
  scrolled.value = window.scrollY > 24
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
    class="fixed inset-x-0 top-0 z-50 border-b transition-[background-color,border-color] duration-300"
    :class="open ? 'border-line bg-night' : scrolled ? 'border-line bg-night/75 backdrop-blur-xl' : 'border-transparent bg-transparent'"
  >
    <nav aria-label="Primary" class="mx-auto flex h-16 max-w-[90rem] items-center justify-between gap-6 px-5 sm:px-8 lg:px-12">
      <a href="#top" class="flex items-center gap-2.5 py-1 pr-2" @click="close()">
        <WispMark class="h-6 w-auto" body="#6b52ff" />
        <span class="text-[19px] leading-none font-semibold tracking-[-0.03em]">Incant</span>
      </a>

      <ul class="hidden items-center gap-1 md:flex">
        <li v-for="item in navItems" :key="item.href">
          <a :href="item.href" class="rounded-full px-3.5 py-2 text-[14px] text-fg-2 transition-colors hover:text-fg">{{ item.label }}</a>
        </li>
      </ul>

      <a
        :href="links.repository"
        class="hidden h-9 items-center gap-2 rounded-full border border-line-strong px-4 text-[13.5px] font-medium text-fg transition-colors hover:border-fg-2 hover:bg-white/5 md:inline-flex"
      >
        GitHub
        <ArrowIcon class="size-3" />
        <span class="sr-only">(Incant repository)</span>
      </a>

      <button
        ref="toggle"
        type="button"
        class="-mr-2 inline-flex size-11 items-center justify-center rounded-md text-fg md:hidden"
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

    <div v-show="open" id="mobile-menu" ref="panel" class="h-[calc(100svh-4rem)] overflow-y-auto border-t border-line px-5 pt-2 pb-10 sm:px-8 md:hidden">
      <ul>
        <li v-for="item in navItems" :key="item.href" class="border-b border-line">
          <a :href="item.href" class="block py-5 text-[28px] font-semibold tracking-[-0.03em]" @click="close()">
            {{ item.label }}
          </a>
        </li>
      </ul>
      <a
        :href="links.repository"
        class="mt-8 inline-flex h-12 items-center gap-2.5 rounded-full bg-fg px-6 text-[15px] font-semibold text-night"
        @click="close()"
      >
        Open the repository on GitHub
        <ArrowIcon class="size-3.5" />
      </a>
    </div>
  </header>
</template>
