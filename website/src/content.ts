/** Genuine destinations only: the public repository and its product plan. */
export const links = {
  repository: 'https://github.com/snowdamiz/incant',
  productGuide: 'https://github.com/snowdamiz/incant/blob/main/PLAN.md',
} as const

export interface NavItem {
  label: string
  href: string
}

export const navItems: NavItem[] = [
  { label: 'Editor', href: '#how' },
  { label: 'Modeling', href: '#create' },
  { label: 'Toolset', href: '#toolset' },
  { label: 'Platforms', href: '#ship' },
  { label: 'FAQ', href: '#faq' },
]

/* ---------- Editor walkthrough ---------- */

export type StageStep = 'ask' | 'review' | 'play' | 'undo'

export interface StageStepCopy {
  id: StageStep
  numeral: string
  label: string
  line: string
}

export const stageSteps: StageStepCopy[] = [
  { id: 'ask', numeral: '01', label: 'Ask', line: 'Describe the change in plain words.' },
  { id: 'review', numeral: '02', label: 'Review', line: 'Read every edit before it lands.' },
  { id: 'play', numeral: '03', label: 'Play-test', line: 'The agent plays it and shows proof.' },
  { id: 'undo', numeral: '04', label: 'Undo', line: 'One step takes it all back.' },
]

export const agentPrompt = 'Give the lighthouse a beam that sweeps the bay every 8 seconds.'

/* ---------- Principles ---------- */

export interface Principle {
  id: 'history' | 'text' | 'evidence'
  title: string
  body: string
}

export const principles: Principle[] = [
  {
    id: 'history',
    title: 'One history',
    body: 'Clicks, scripts, teammates and the agent share one undoable timeline.',
  },
  {
    id: 'text',
    title: 'Plain-text worlds',
    body: 'Scenes, materials and graphs are typed documents. Diff them, review them, merge them.',
  },
  {
    id: 'evidence',
    title: 'Evidence, not guesses',
    body: 'The agent plays the build and attaches frames, logs and checks before it calls a change done.',
  },
]

/* ---------- Modeling ---------- */

/* The geometry graph that builds the lighthouse in the modeling viewport. Values are illustrative. */
export interface GraphNode {
  id: 'cylinder' | 'taper' | 'extrude' | 'array' | 'boolean' | 'scatter' | 'output'
  op: string
  detail: string
  /** What this node adds, used in the preview's accessible description. */
  adds: string
}

export const lighthouseGraph: GraphNode[] = [
  { id: 'cylinder', op: 'Cylinder', detail: 'r 1.5 · h 9 · 48 sides', adds: 'a plain cylinder for the tower' },
  { id: 'taper', op: 'Taper', detail: 'top 0.66', adds: 'a taper that narrows the top of the tower' },
  { id: 'extrude', op: 'Extrude', detail: 'gallery · lantern · roof', adds: 'a gallery deck, lantern room and roof extruded from the top' },
  { id: 'array', op: 'Array', detail: 'window × 4 · spiral', adds: 'four windows arrayed in a spiral' },
  { id: 'boolean', op: 'Boolean', detail: 'subtract · doorway', adds: 'a doorway cut with a boolean' },
  { id: 'scatter', op: 'Scatter', detail: 'rock × 9 · noise 0.3', adds: 'rocks scattered around the base' },
  { id: 'output', op: 'Output', detail: 'materials · UVs · LOD 0–2', adds: 'materials, UVs and levels of detail' },
]

/* ---------- Toolset ---------- */

export type ToolIcon =
  | 'graph'
  | 'mesh'
  | 'shader'
  | 'terrain'
  | 'light'
  | 'anim'
  | 'vfx'
  | 'physics'
  | 'net'
  | 'sprite'
  | 'text'
  | 'nav'

export interface Tool {
  icon: ToolIcon
  title: string
  body: string
}

export const tools: Tool[] = [
  { icon: 'graph', title: 'Geometry graphs', body: 'Booleans, bevels, arrays, lofts, scatter and instancing.' },
  { icon: 'mesh', title: 'Mesh tools', body: 'Cleanup, retopology, UVs, LODs and baking.' },
  { icon: 'shader', title: 'Shader graphs', body: 'Node materials compiled to WGSL, with custom nodes.' },
  { icon: 'terrain', title: 'Terrain', body: 'Heightmaps, layered materials and foliage.' },
  { icon: 'light', title: 'Lighting', body: 'Shadows, lightmaps, probe volumes and ambient occlusion.' },
  { icon: 'anim', title: 'Animation', body: 'Retargeting, IK rigs, blend trees and timelines.' },
  { icon: 'vfx', title: 'Effects', body: 'GPU particles and flipbooks.' },
  { icon: 'physics', title: 'Physics', body: 'Rapier in 3D and 2D, with a deterministic mode.' },
  { icon: 'net', title: 'Multiplayer', body: 'QUIC transport, rollback and cross-play.' },
  { icon: 'sprite', title: '2D', body: 'Sprites, atlases, tilemaps and 2D lights.' },
  { icon: 'text', title: 'UI and text', body: 'Flexbox layout, shaping, bidi and localization.' },
  { icon: 'nav', title: 'Navigation', body: 'Navmeshes, pathfinding and steering.' },
]

/* ---------- Ship ---------- */

export interface ExportTarget {
  id: 'windows' | 'macos' | 'linux' | 'ios' | 'android' | 'web'
  output: string
}

export const exportTargets: ExportTarget[] = [
  { id: 'windows', output: 'Desktop build' },
  { id: 'macos', output: 'Desktop build' },
  { id: 'linux', output: 'Desktop build' },
  { id: 'ios', output: 'Xcode project' },
  { id: 'android', output: 'Gradle project' },
  { id: 'web', output: 'Browser build' },
]

/* ---------- FAQ ---------- */

export interface FaqItem {
  question: string
  answer: string
}

export const faqItems: FaqItem[] = [
  {
    question: 'What is Incant?',
    answer:
      'A 3D and 2D modeling tool, game engine and editor in one application. Build by hand, or ask the AI agent to make the change through the same commands.',
  },
  {
    question: 'Can the agent change things without me?',
    answer:
      'Only as far as you allow. Every agent action is a transaction in your history: inspect it, amend it, or undo it in one step.',
  },
  {
    question: 'Which AI model does it use?',
    answer:
      'Yours. Connect OpenAI, Anthropic, Google or a local model and pick one per task. Credentials stay on your machine, and every turn shows its cost.',
  },
  {
    question: 'Does it work offline?',
    answer:
      'Yes. The editor needs no connection and no Incant account. The agent needs a model provider, or a local model.',
  },
  {
    question: 'Can my team work on the same project?',
    answer:
      'Yes. Projects merge concurrent edits, from teammates and the agent alike. An Incant account adds cloud sync and invitations.',
  },
  {
    question: 'Do I still need Blender?',
    answer:
      'For most game assets, no. Detailed sculpting stays in dedicated tools by design, and those meshes import as glTF or FBX.',
  },
  {
    question: 'What kinds of games is it for?',
    answer:
      '2D and 3D games in TypeScript, from third-person, first-person, platformer and top-down templates, with online multiplayer and cross-play built in.',
  },
]
