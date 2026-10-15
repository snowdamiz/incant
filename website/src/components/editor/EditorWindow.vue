<script setup lang="ts">
import { computed } from 'vue'
import EdIcon from './EdIcon.vue'
import EditorViewport from './EditorViewport.vue'
import WispMark from '../WispMark.vue'
import { agentPrompt, type StageStep } from '../../content'
import type { EdIconName } from './icons'

/*
 * A recreation of the Incant editor (layout and tokens from editor/ui), driven by the
 * walkthrough step. It is one picture for assistive technology; the step tabs carry the
 * interaction, and the pointer shortcuts below only duplicate them.
 */
const props = defineProps<{ step: StageStep }>()
const emit = defineEmits<{ go: [step: StageStep] }>()

const applied = computed(() => props.step === 'play')

interface Row {
  name: string
  icon: EdIconName
  depth: number
  tone?: 'kind-light' | 'kind-camera' | 'kind-mesh' | 'kind-script'
  selected?: boolean
  added?: boolean
  open?: boolean
}

const rows = computed<Row[]>(() => {
  const r: Row[] = [
    { name: 'Lighthouse Bay', icon: 'scene', depth: 0, open: true },
    { name: 'Environment', icon: 'group', depth: 1, open: true },
    { name: 'Sky', icon: 'group', depth: 2 },
    { name: 'Sea', icon: 'mesh', depth: 2, tone: 'kind-mesh' },
    { name: 'Moonlight', icon: 'light', depth: 2, tone: 'kind-light' },
    { name: 'Headland', icon: 'group', depth: 1, open: true },
    { name: 'Rocks', icon: 'mesh', depth: 2, tone: 'kind-mesh' },
    { name: 'Keeper’s House', icon: 'mesh', depth: 2, tone: 'kind-mesh' },
    { name: 'Lighthouse', icon: 'mesh', depth: 2, tone: 'kind-mesh', open: true },
    { name: 'Lantern', icon: 'light', depth: 3, tone: 'kind-light', selected: !applied.value },
  ]
  if (applied.value) {
    r.push({ name: 'Beam', icon: 'light', depth: 3, tone: 'kind-light', selected: true, added: true })
    r.push({ name: 'sweep.ts', icon: 'script', depth: 4, tone: 'kind-script', added: true })
  }
  r.push(
    { name: 'Cameras', icon: 'group', depth: 1, open: true },
    { name: 'Shore Camera', icon: 'camera', depth: 2, tone: 'kind-camera' },
    { name: 'Follow Camera', icon: 'camera', depth: 2, tone: 'kind-camera' },
  )
  return r
})

const toneClass: Record<NonNullable<Row['tone']>, string> = {
  'kind-light': 'text-ed-light',
  'kind-camera': 'text-ed-camera',
  'kind-mesh': 'text-ed-mesh',
  'kind-script': 'text-ed-script',
}

interface HistoryEntry {
  label: string
  origin: 'Import' | 'User' | 'Agent'
  time: string
  undone?: boolean
  pending?: boolean
}

const history = computed<HistoryEntry[]>(() => {
  const base: HistoryEntry[] = [
    { label: 'Import cliffs.glb', origin: 'Import', time: '17:38:02' },
    { label: 'Set Transform on Lantern', origin: 'User', time: '17:40:11' },
    { label: 'Rename Keeper’s House', origin: 'User', time: '17:41:37' },
  ]
  if (props.step === 'review') base.unshift({ label: 'Add a sweeping beam · 4 edits', origin: 'Agent', time: 'awaiting', pending: true })
  if (props.step === 'play') base.unshift({ label: 'Add a sweeping beam · 4 edits', origin: 'Agent', time: '17:42:20' })
  if (props.step === 'undo') base.unshift({ label: 'Add a sweeping beam · 4 edits', origin: 'Agent', time: '17:42:20', undone: true })
  return base
})

const originClass = { Import: 'text-ed-import', User: 'text-ed-user', Agent: 'text-ed-agent' } as const

const agentStatus = computed(() => {
  switch (props.step) {
    case 'ask':
      return { label: 'Working', dot: 'bg-ed-user motion-safe:animate-pulse-soft' }
    case 'review':
      return { label: 'Needs approval', dot: 'bg-ed-warning' }
    case 'play':
      return { label: 'Ready', dot: 'bg-ed-success' }
    default:
      return { label: 'Ready', dot: 'bg-ed-success' }
  }
})

const statusLine = computed(() => {
  switch (props.step) {
    case 'ask':
      return { text: 'Agent working', tone: 'text-ed-muted' }
    case 'review':
      return { text: '4 edits awaiting approval', tone: 'text-ed-warning' }
    case 'play':
      return { text: 'Play-test passed', tone: 'text-ed-success' }
    default:
      return { text: 'Transaction undone', tone: 'text-ed-warning' }
  }
})

const edits = [
  { mark: '+', text: ['Create ', 'Beam', ' with Transform, SpotLight'] },
  { mark: '+', text: ['Attach ', 'sweep.ts', ' to Beam'] },
  { mark: '~', text: ['Set Emission on ', 'Lantern', ''] },
  { mark: '~', text: ['Set Fog on ', 'Bay', ''] },
] as const

const checks = ['Beam completes a turn every 8.0 s', 'Lantern stays lit after a reload', 'No new problems']
</script>

<template>
  <div class="flex flex-col overflow-hidden rounded-[14px] bg-ed-app text-[13px] text-ed-text select-none">
    <!-- Titlebar -->
    <div class="flex h-11 shrink-0 items-center gap-3 border-b border-ed-border px-4">
      <span class="flex gap-2" aria-hidden="true">
        <span class="size-3 rounded-full bg-[#ff5f57]" /><span class="size-3 rounded-full bg-[#febc2e]" /><span class="size-3 rounded-full bg-[#28c840]" />
      </span>
      <WispMark class="ml-2 h-4 w-auto" body="#8c95ff" />
      <span class="truncate font-medium">Lighthouse Bay</span>
      <span class="hidden text-ed-subtle sm:inline">/</span>
      <span class="hidden text-ed-muted sm:inline">bay.scene</span>
      <div class="mx-auto hidden items-center gap-1 lg:flex">
        <span
          class="grid size-7 place-items-center rounded-md"
          :class="step === 'play' ? 'cursor-pointer text-ed-text hover:bg-ed-hover' : 'text-ed-muted'"
          @click="step === 'play' && emit('go', 'undo')"
        ><EdIcon name="undo" class="size-4" /></span>
        <span class="grid size-7 place-items-center rounded-md" :class="step === 'undo' ? 'text-ed-text' : 'text-ed-subtle/60'"><EdIcon name="redo" class="size-4" /></span>
        <span class="mx-2 h-4 w-px bg-ed-border" />
        <span class="grid size-7 place-items-center rounded-md bg-ed-raised text-ed-soft"><EdIcon name="panel-left" class="size-4" /></span>
        <span class="grid size-7 place-items-center rounded-md bg-ed-raised text-ed-soft"><EdIcon name="panel-bottom" class="size-4" /></span>
        <span class="grid size-7 place-items-center rounded-md bg-ed-raised text-ed-soft"><EdIcon name="panel-right" class="size-4" /></span>
      </div>
      <span class="ml-auto grid size-7 place-items-center rounded-full border border-ed-border-strong text-[11px] font-semibold text-ed-soft lg:ml-0">A</span>
    </div>

    <div class="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[minmax(0,1fr)_19rem] lg:grid-cols-[14.5rem_minmax(0,1fr)_20.5rem]">
      <!-- Hierarchy -->
      <div class="hidden min-h-0 flex-col border-r border-ed-border bg-ed-panel lg:flex">
        <div class="flex h-10 items-end gap-5 border-b border-ed-border px-4">
          <span class="border-b-2 border-ed-accent pb-2 font-medium">Hierarchy <span class="ml-1 text-ed-subtle">{{ rows.length }}</span></span>
          <span class="pb-2.5 text-ed-muted">Assets <span class="ml-1 text-ed-subtle">9</span></span>
        </div>
        <div class="px-2.5 pt-2.5">
          <div class="flex h-8 items-center gap-2 rounded-md bg-ed-raised/70 px-2.5 text-ed-subtle">
            <EdIcon name="search" class="size-3.5" />Filter
          </div>
        </div>
        <ul class="flex-1 space-y-px overflow-hidden px-2 py-2">
          <li
            v-for="row in rows"
            :key="row.name"
            class="flex h-[27px] items-center gap-1.5 rounded-md pr-2"
            :class="row.selected ? 'bg-ed-selected text-ed-text' : 'text-ed-soft'"
            :style="{ paddingLeft: `${0.35 + row.depth * 0.8}rem` }"
          >
            <EdIcon v-if="row.open" name="chevron" class="size-3 shrink-0 text-ed-subtle" />
            <span v-else class="w-3 shrink-0" />
            <EdIcon :name="row.icon" class="size-3.5 shrink-0" :class="row.tone ? toneClass[row.tone] : 'text-ed-muted'" />
            <span class="truncate" :class="row.depth === 0 ? 'font-medium' : ''">{{ row.name }}</span>
            <span v-if="row.added" class="ml-auto text-[11px] text-ed-agent">agent</span>
          </li>
        </ul>
      </div>

      <!-- Viewport and dock -->
      <div class="flex min-h-0 min-w-0 flex-col">
        <div class="flex h-10 shrink-0 items-center gap-2 border-b border-ed-border bg-ed-panel px-4">
          <span class="font-medium">Viewport</span>
          <span class="inline-flex items-center gap-1.5 text-ed-muted">
            <span class="size-1.5 rounded-full" :class="step === 'play' ? 'bg-ed-danger' : 'bg-ed-success'" />{{ step === 'play' ? 'Playing' : 'Live' }}
          </span>
          <span class="ml-auto hidden items-center gap-1.5 text-ed-soft sm:inline-flex">
            <EdIcon name="camera" class="size-3.5 text-ed-camera" />Shore Camera<EdIcon name="chevron" class="size-3 text-ed-subtle" />
          </span>
          <span class="ml-auto flex items-center gap-1 sm:ml-3">
            <span class="grid size-7 place-items-center rounded-md" :class="step === 'play' ? 'bg-ed-raised text-ed-text' : 'text-ed-muted'"><EdIcon name="play" class="size-3" /></span>
            <span class="grid size-7 place-items-center rounded-md text-ed-muted"><EdIcon name="step" class="size-3" /></span>
            <span class="grid size-7 place-items-center rounded-md" :class="step === 'play' ? 'text-ed-danger' : 'text-ed-subtle/60'"><EdIcon name="stop" class="size-3" /></span>
          </span>
        </div>

        <div class="relative aspect-[4/3] min-h-0 sm:aspect-[16/10] md:aspect-auto md:flex-1">
          <EditorViewport :step="step" class="absolute inset-0" />
        </div>

        <!-- History dock -->
        <div class="hidden h-[11.5rem] shrink-0 flex-col border-t border-ed-border bg-ed-panel lg:flex">
          <div class="flex h-10 items-end gap-6 border-b border-ed-border px-4">
            <span class="pb-2.5 text-ed-muted">Problems</span>
            <span class="pb-2.5 text-ed-muted">Console <span class="ml-1 text-ed-subtle">2</span></span>
            <span class="border-b-2 border-ed-accent pb-2 font-medium">History <span class="ml-1 text-ed-subtle">{{ history.length }}</span></span>
          </div>
          <div class="flex h-9 items-center gap-4 px-4 text-ed-muted">
            <span
              class="inline-flex items-center gap-1.5"
              :class="step === 'play' ? 'cursor-pointer text-ed-text' : ''"
              @click="step === 'play' && emit('go', 'undo')"
            ><EdIcon name="undo" class="size-3.5" />Undo</span>
            <span class="inline-flex items-center gap-1.5" :class="step === 'undo' ? 'text-ed-text' : 'text-ed-subtle'"><EdIcon name="redo" class="size-3.5 opacity-60" />Redo</span>
            <span class="ml-auto text-ed-subtle">{{ step === 'undo' ? '3 of 4 applied' : step === 'play' ? '4 of 4 applied' : '3 of 3 applied' }}</span>
          </div>
          <ul class="px-4">
            <li v-for="entry in history" :key="entry.label + entry.time" class="flex h-[26px] items-center gap-3">
              <span class="truncate" :class="entry.undone ? 'text-ed-subtle line-through' : entry.pending ? 'text-ed-muted italic' : 'text-ed-soft'">{{ entry.label }}</span>
              <span class="ml-auto" :class="originClass[entry.origin]">{{ entry.origin }}</span>
              <span class="w-[4.5rem] text-right font-mono text-[12px] text-ed-subtle">{{ entry.time }}</span>
            </li>
          </ul>
        </div>
      </div>

      <!-- Inspector and agent -->
      <div class="flex min-h-0 flex-col border-t border-ed-border bg-ed-side md:border-t-0 md:border-l">
        <div class="hidden min-h-0 flex-1 flex-col overflow-hidden md:flex">
          <div class="flex h-10 shrink-0 items-center gap-2 border-b border-ed-border px-4">
            <EdIcon name="light" class="size-3.5 text-ed-light" />
            <span class="font-medium">{{ applied ? 'Beam' : 'Lantern' }}</span>
            <span class="ml-auto flex gap-2 text-ed-subtle"><EdIcon name="lock" class="size-3.5" /><EdIcon name="copy" class="size-3.5" /></span>
          </div>
          <div class="space-y-1 overflow-hidden px-4 py-3">
            <p class="flex items-center gap-1.5 pb-1 font-medium"><EdIcon name="chevron" class="size-3 text-ed-subtle" /><EdIcon name="move" class="size-3.5 text-ed-muted" />Transform</p>
            <div class="grid grid-cols-[5.2rem_1fr] items-center gap-y-1.5">
              <span class="text-ed-muted">Translation</span>
              <span class="grid grid-cols-3 gap-px font-mono text-[12px]">
                <span class="rounded-l-[5px] bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-x">X</b>0</span>
                <span class="bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-y">Y</b>9.86</span>
                <span class="rounded-r-[5px] bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-z">Z</b>0</span>
              </span>
              <span class="text-ed-muted">Rotation</span>
              <span class="grid grid-cols-3 gap-px font-mono text-[12px]">
                <span class="rounded-l-[5px] bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-x">X</b>0</span>
                <span class="bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-y">Y</b>{{ applied ? '0.38' : '0' }}</span>
                <span class="rounded-r-[5px] bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-z">Z</b>0</span>
              </span>
            </div>
            <p class="flex items-center gap-1.5 pt-3 pb-1 font-medium"><EdIcon name="chevron" class="size-3 text-ed-subtle" /><EdIcon name="light" class="size-3.5 text-ed-light" />{{ applied ? 'Spot Light' : 'Point Light' }}</p>
            <div class="grid grid-cols-[5.2rem_1fr] items-center gap-y-1.5">
              <span class="text-ed-muted">Color</span>
              <span class="flex gap-px font-mono text-[12px]">
                <span class="w-6 rounded-l-[5px] bg-[#ffb862]" />
                <span class="flex-1 bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-x">R</b>1</span>
                <span class="flex-1 bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-y">G</b>.72</span>
                <span class="flex-1 rounded-r-[5px] bg-ed-raised px-1.5 py-1"><b class="mr-1 font-medium text-ed-z">B</b>.38</span>
              </span>
              <span class="text-ed-muted">Intensity</span>
              <span class="rounded-[5px] bg-ed-raised px-2 py-1 font-mono text-[12px]">{{ applied ? '12000' : '800' }} <span class="text-ed-subtle">cd</span></span>
              <span class="text-ed-muted">{{ applied ? 'Angle' : 'Range' }}</span>
              <span class="rounded-[5px] bg-ed-raised px-2 py-1 font-mono text-[12px]">{{ applied ? '6' : '24' }} <span class="text-ed-subtle">{{ applied ? 'deg' : 'm' }}</span></span>
            </div>
          </div>
        </div>

        <!-- Agent -->
        <div class="flex shrink-0 flex-col border-ed-border md:border-t">
          <div class="flex h-10 items-center gap-2 px-4">
            <EdIcon name="chevron" class="size-3 text-ed-subtle" />
            <WispMark class="h-3.5 w-auto" body="#8c95ff" />
            <span class="font-medium">Agent</span>
            <span class="ml-auto inline-flex items-center gap-1.5 text-ed-muted"><span class="size-1.5 rounded-full" :class="agentStatus.dot" />{{ agentStatus.label }}</span>
          </div>

          <div class="min-h-[12rem] px-4 pb-3 md:min-h-[13.5rem]">
            <!-- Ask -->
            <template v-if="step === 'ask'">
              <p class="rounded-lg bg-ed-well px-3 py-2.5 leading-relaxed text-ed-soft">{{ agentPrompt }}</p>
              <ul class="mt-3 space-y-1.5 text-ed-muted">
                <li class="flex items-center gap-2"><EdIcon name="check" class="size-3.5 text-ed-success" />Read Lighthouse, Lantern and Bay</li>
                <li class="flex items-center gap-2"><EdIcon name="check" class="size-3.5 text-ed-success" />Found the Lantern light at 9.86 m</li>
                <li class="flex items-center gap-2 text-ed-soft">
                  <span class="grid size-3.5 place-items-center"><span class="size-2 rounded-full border border-ed-user motion-safe:animate-pulse-soft" /></span>Planning 4 edits<span class="motion-safe:animate-caret">…</span>
                </li>
              </ul>
            </template>

            <!-- Review -->
            <div v-else-if="step === 'review'" class="rounded-lg border border-ed-border-strong bg-ed-panel p-3">
              <p class="font-semibold">Apply 4 changes to the project?</p>
              <p class="mt-1 leading-snug text-ed-muted">“{{ agentPrompt }}”</p>
              <ul class="mt-2.5 space-y-1 font-[450]">
                <li v-for="edit in edits" :key="edit.text.join('')" class="flex gap-2 truncate">
                  <span class="w-2.5 shrink-0" :class="edit.mark === '+' ? 'text-ed-success' : 'text-ed-muted'">{{ edit.mark }}</span>
                  <span class="truncate text-ed-muted">{{ edit.text[0] }}<span class="text-ed-text">{{ edit.text[1] }}</span>{{ edit.text[2] }}</span>
                </li>
              </ul>
              <div class="mt-3 flex justify-end gap-2">
                <span class="rounded-md border border-ed-border-strong px-3 py-1.5 font-medium">Deny</span>
                <span
                  class="cursor-pointer rounded-md bg-ed-accent px-3 py-1.5 font-semibold text-ed-app hover:bg-[#a9b0ff]"
                  @click="emit('go', 'play')"
                >Allow</span>
              </div>
            </div>

            <!-- Play-test -->
            <template v-else-if="step === 'play'">
              <p class="leading-relaxed text-ed-soft">Added a beam that sweeps the bay every 8 seconds. The play-test passed.</p>
              <ul class="mt-3 space-y-1.5">
                <li v-for="check in checks" :key="check" class="flex items-center gap-2 text-ed-soft">
                  <EdIcon name="check" class="size-3.5 shrink-0 text-ed-success" />{{ check }}
                </li>
              </ul>
              <p class="mt-3 flex items-center justify-between text-ed-muted">
                <span>4 edits · 18,204 tokens · $0.06</span>
                <span class="text-ed-soft underline underline-offset-2">Show in History</span>
              </p>
            </template>

            <!-- Undo -->
            <template v-else>
              <p class="leading-relaxed text-ed-soft">Undid “Add a sweeping beam”. All 4 edits were reverted in one step, and the scene matches revision 41.</p>
              <p class="mt-3 flex items-center gap-2 text-ed-muted"><EdIcon name="redo" class="size-3.5" />Redo brings it back unchanged.</p>
            </template>
          </div>

          <div class="mx-3 mb-3 rounded-lg border border-ed-border-strong bg-ed-well px-3 pt-2.5 pb-2">
            <p class="text-ed-subtle">Ask for a change…</p>
            <p class="mt-2.5 flex items-center gap-3 text-[12px] text-ed-muted">
              <span>Ask before edits</span>
              <span class="ml-auto grid size-7 place-items-center rounded-md" :class="step === 'ask' ? 'bg-ed-accent text-ed-app' : 'bg-ed-raised text-ed-subtle'">
                <EdIcon :name="step === 'ask' ? 'stop' : 'send'" class="size-3.5" />
              </span>
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- Status bar -->
    <div class="flex h-8 shrink-0 items-center gap-4 border-t border-ed-border px-4 text-[12px] text-ed-muted">
      <span class="inline-flex items-center gap-1.5"><span class="size-1.5 rounded-full bg-ed-success" />Native engine</span>
      <span class="hidden items-center gap-1.5 sm:inline-flex"><EdIcon name="check" class="size-3.5 text-ed-success" />No problems</span>
      <span class="ml-auto" :class="statusLine.tone">{{ statusLine.text }}</span>
    </div>
  </div>
</template>
