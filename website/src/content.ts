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
    body: 'Your clicks, your scripts, your team and the agent, in one undoable timeline.',
  },
  {
    numeral: '02',
    title: 'Plain-text worlds',
    body: 'Scenes, materials and graphs are documents you can diff, review and merge.',
  },
  {
    numeral: '03',
    title: 'Evidence, not guesses',
    body: 'The agent plays the game and shows its proof before it calls a change done.',
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

/* Content tools (PLAN.md Phase 4), as a scannable index rather than prose. */
export interface ToolEntry {
  term: string
  gloss: string
}

export const toolIndex: ToolEntry[] = [
  { term: 'Geometry graphs', gloss: 'Booleans, bevels, lofts, arrays, scatter' },
  { term: 'Mesh tools', gloss: 'Retopology, UVs, LODs, baking, light sculpt' },
  { term: 'Materials', gloss: 'Shader graphs compiled to WGSL' },
  { term: 'Terrain', gloss: 'Heightmaps, layered materials, foliage' },
  { term: 'Animation and VFX', gloss: 'Retargeting, IK, timelines, particles' },
  { term: 'Generated assets', gloss: 'Image-to-3D, texture sets, motion' },
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

/* The end-to-end path (PLAN.md sections 1.1, 5 and 6): a verb and the artifact it leaves. */
export interface WorkflowStep {
  title: string
  artifact: string
}

export const workflowSteps: WorkflowStep[] = [
  { title: 'Model', artifact: 'lighthouse.geo' },
  { title: 'Surface', artifact: 'weathered_stone.mat' },
  { title: 'Script', artifact: 'lamp.ts' },
  { title: 'Review', artifact: '4 edits · 1 transaction' },
  { title: 'Play-test', artifact: '3 of 3 checks passed' },
  { title: 'Ship', artifact: '6 platforms' },
]

export const exportTargets = ['Windows', 'macOS', 'Linux', 'iOS', 'Android', 'Web'] as const
