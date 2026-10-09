/** Genuine destinations only: the public repository and its product guide. */
export const links = {
  repository: 'https://github.com/snowdamiz/incant',
  productGuide: 'https://github.com/snowdamiz/incant/blob/main/PLAN.md',
} as const

export interface NavItem {
  label: string
  href: string
}

export const navItems: NavItem[] = [
  { label: 'How it works', href: '#how' },
  { label: 'Modeling', href: '#create' },
  { label: 'Workflow', href: '#workflow' },
  { label: 'FAQ', href: '#faq' },
]

export type StageStep = 'ask' | 'review' | 'play' | 'undo'

export interface StageStepCopy {
  id: StageStep
  numeral: string
  label: string
  line: string
}

export const stageSteps: StageStepCopy[] = [
  { id: 'ask', numeral: '01', label: 'Ask', line: 'Describe the mechanic in plain words.' },
  { id: 'review', numeral: '02', label: 'Review', line: 'Read the exact change before it lands.' },
  { id: 'play', numeral: '03', label: 'Play-test', line: 'The agent plays it and shows its evidence.' },
  { id: 'undo', numeral: '04', label: 'Undo', line: 'One step takes the whole change back.' },
]

export interface Principle {
  numeral: string
  title: string
  body: string
}

export const principles: Principle[] = [
  {
    numeral: '01',
    title: 'One history',
    body: 'Clicks, scripts, teammates and the agent all make the same kind of edit, in one timeline, with a name on each.',
  },
  {
    numeral: '02',
    title: 'Plain-text worlds',
    body: 'Scenes, materials and graphs are readable documents. Concurrent edits merge, and you review a change the way you review code.',
  },
  {
    numeral: '03',
    title: 'Evidence, not guesses',
    body: 'The agent plays the game, watches the viewport and reads the console before it says it is done.',
  },
  {
    numeral: '04',
    title: 'Yours to run',
    body: 'Connect your own model provider. The editor works offline, with no Incant account required.',
  },
]

export interface FaqItem {
  question: string
  answer: string
}

export const faqItems: FaqItem[] = [
  {
    question: 'What is Incant?',
    answer:
      'One application that is a 3D and 2D modeling tool, a game engine and an editor. Build by hand, or describe what you want and let an AI agent make the change in the same project, through the same commands.',
  },
  {
    question: 'Can the agent change things without me?',
    answer:
      'Every agent action is a transaction in the same history as your own edits. Choose whether it asks first, inspect what it did, amend it, or undo the whole thing in one step. Each entry records who made it.',
  },
  {
    question: 'Which AI model does it use?',
    answer:
      'Yours. Connect OpenAI, Anthropic, Google or a local model, and pick one per task. Calls go directly from your machine to the provider, credentials stay on your machine, and every turn shows its cost against a budget you set.',
  },
  {
    question: 'Does it work offline?',
    answer:
      'Yes. The editor works fully offline with no Incant account, and no editing feature sits behind a login. The agent needs a model connection, or a local model if you prefer to stay offline.',
  },
  {
    question: 'Can my team work on the same project?',
    answer:
      'Yes. A project is a mergeable document, so teammates and the agent can edit the same scene at once without conflicts. An Incant account adds cloud sync and invitations. Everything else works without one.',
  },
  {
    question: 'Do I still need Blender?',
    answer:
      'For most game assets, no. Geometry graphs, mesh cleanup, materials, terrain and animation all live in the engine. Detailed sculpting stays in dedicated tools by design, and those meshes import as glTF or FBX.',
  },
  {
    question: 'What kinds of games is it for?',
    answer:
      '2D and 3D games written in TypeScript, from templates for third-person, first-person, 2D platformer and top-down. Online multiplayer is built in, with server-authoritative and rollback modes, and cross-play between desktop and mobile.',
  },
]

/* Content tools (PLAN.md Phase 4). Prose groups, not a feature checklist. */
export interface ToolGroup {
  title: string
  body: string
}

export const toolGroups: ToolGroup[] = [
  {
    title: 'Geometry graphs',
    body: 'Build meshes from primitives, booleans, extrusions, bevels, subdivision, arrays, curves and lofts, then scatter, instance and displace them with noise. Graphs are saved as text, so the agent can write one and you can review it.',
  },
  {
    title: 'Mesh tools',
    body: 'Cleanup turns imported and generated meshes into engine-ready ones, with automatic retopology and UVs, LODs, decimation, and normal and AO baking. A light brush sculpt mode handles adjustments.',
  },
  {
    title: 'Materials',
    body: 'A shader graph compiles to WGSL, with nodes for PBR inputs, math, textures, UV operations and vertex animation. When no node fits, you or the agent can write WGSL directly.',
  },
  {
    title: 'Terrain and light',
    body: 'Paint heightmaps, layer materials and scatter foliage across terrain that streams in. Baked lightmaps and probe volumes light every tier, with screen-space GI on desktop.',
  },
  {
    title: 'Animation and effects',
    body: 'Retarget animation between rigs, set up IK, build animation graphs and cut cinematics on a timeline. GPU particles get a graph editor of their own.',
  },
  {
    title: 'Generated assets',
    body: 'Plug in providers for image-to-3D with automatic cleanup, texture sets and humanoid animation from text. Results go through the same pipeline as imported glTF and FBX, with licensing metadata attached.',
  },
]

/* The interactive geometry graph in the modeling stage. Values are illustrative. */
export interface GraphNode {
  id: string
  op: string
  detail: string
  /** What this node adds, used in the preview's accessible description. */
  adds: string
}

export const lighthouseGraph: GraphNode[] = [
  { id: 'cylinder', op: 'Cylinder', detail: 'radius 3.5 · height 24 · sides 32', adds: 'a plain cylinder for the tower' },
  { id: 'taper', op: 'Taper', detail: 'top 0.66', adds: 'a taper that narrows the top of the tower' },
  { id: 'extrude', op: 'Extrude', detail: 'gallery · out 0.8 · bevel 0.1', adds: 'a gallery deck and lantern room extruded from the top' },
  { id: 'array', op: 'Array', detail: 'window × 4 · spiral', adds: 'four windows arrayed in a spiral' },
  { id: 'boolean', op: 'Boolean', detail: 'subtract · door', adds: 'a doorway cut with a boolean' },
  { id: 'scatter', op: 'Scatter', detail: 'rock × 40 · noise 0.3', adds: 'rocks scattered around the base with noise' },
  { id: 'output', op: 'Output', detail: 'retopo · auto-UV · LOD 0–2', adds: 'cleanup, UVs and levels of detail, shown as a wireframe' },
]

/* The end-to-end path (PLAN.md sections 1.1, 5 and 6). */
export interface WorkflowStep {
  numeral: string
  title: string
  body: string
  artifact: string
}

export const workflowSteps: WorkflowStep[] = [
  {
    numeral: '01',
    title: 'Model',
    body: 'Shape the lighthouse in a geometry graph, or import a mesh and let cleanup make it game-ready.',
    artifact: 'lighthouse.geo',
  },
  {
    numeral: '02',
    title: 'Surface',
    body: 'Layer a weathered stone material in the shader graph. It compiles to WGSL for every tier.',
    artifact: 'weathered_stone.mat',
  },
  {
    numeral: '03',
    title: 'Script',
    body: 'Write the lamp’s behavior in TypeScript, with types from the schema and hot reload.',
    artifact: 'lamp.ts',
  },
  {
    numeral: '04',
    title: 'Review',
    body: 'Agent changes arrive as one transaction with a readable diff, beside your own edits.',
    artifact: '4 edits · 1 transaction',
  },
  {
    numeral: '05',
    title: 'Play-test',
    body: 'The headless runner plays the scene and returns frames, logs and assertion results.',
    artifact: '3 of 3 checks passed',
  },
  {
    numeral: '06',
    title: 'Ship',
    body: 'Export the same project to desktop, mobile and the web, with native projects you can open.',
    artifact: '6 targets',
  },
]

export const exportTargets = ['Windows', 'macOS', 'Linux', 'iOS', 'Android', 'Web'] as const
