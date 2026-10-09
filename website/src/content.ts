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
  id: 'history' | 'text' | 'evidence'
  numeral: string
  title: string
  body: string
}

export const principles: Principle[] = [
  {
    id: 'history',
    numeral: '01',
    title: 'One history',
    body: 'Your clicks, your scripts, your team and the agent, in one undoable timeline.',
  },
  {
    id: 'text',
    numeral: '02',
    title: 'Plain-text worlds',
    body: 'Scenes, materials and graphs are documents you can diff, review and merge.',
  },
  {
    id: 'evidence',
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

/*
 * Content tools (PLAN.md Phase 4), told as a strip of small illustrated viewports.
 * `label` is the only visible text; `description` is the accessible description of the picture;
 * `detail` feeds the optional "whole toolset" disclosure.
 */
export type VignetteId = 'retopo' | 'lod' | 'material' | 'terrain' | 'rig' | 'generate'

export interface ToolVignette {
  id: VignetteId
  label: string
  description: string
}

export const toolVignettes: ToolVignette[] = [
  {
    id: 'retopo',
    label: 'Retopology · UVs',
    description: 'Automatic retopology and UVs: a clean quad-mesh sphere beside its unwrapped UV islands.',
  },
  {
    id: 'lod',
    label: 'LODs · Decimation',
    description: 'Level-of-detail generation: the same rock at three decreasing polygon counts.',
  },
  {
    id: 'material',
    label: 'Shader graphs',
    description: 'A material graph: two texture nodes wired into a shaded stone sphere.',
  },
  {
    id: 'terrain',
    label: 'Terrain · Foliage',
    description: 'Terrain: a heightmapped ridge with layered rock, grass and sand materials and scattered trees.',
  },
  {
    id: 'rig',
    label: 'Rigs · IK · Timelines',
    description: 'Animation: a character rig mid-stride with an IK target on one foot, above a timeline of keyframes.',
  },
  {
    id: 'generate',
    label: 'Image to 3D',
    description: 'Generated assets: a flat picture of a crate turned into a clean, game-ready 3D crate.',
  },
]

export interface ToolDetail {
  term: string
  gloss: string
}

export const toolDetails: ToolDetail[] = [
  { term: 'Geometry graphs', gloss: 'Primitives, booleans, extrude, bevel, subdivision, arrays, curves, lofts, scatter, instancing, noise' },
  { term: 'Mesh tools', gloss: 'Cleanup, retopology, UVs, LODs, decimation, normal and AO baking, light sculpting' },
  { term: 'Materials', gloss: 'Shader graphs compiled to WGSL, with custom WGSL nodes' },
  { term: 'Terrain and light', gloss: 'Heightmaps, layered materials, foliage, lightmaps, probe volumes' },
  { term: 'Animation and VFX', gloss: 'Retargeting, IK rigs, animation graphs, cinematic timelines, GPU particles' },
  { term: 'Generated assets', gloss: 'Image-to-3D, texture sets, animation from text, alongside glTF and FBX' },
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

/* The end-to-end path (PLAN.md sections 1.1, 5 and 6), drawn as a pipeline with one line each. */
export interface WorkflowStep {
  id: 'model' | 'surface' | 'script' | 'review' | 'playtest' | 'ship'
  title: string
  body: string
}

export const workflowSteps: WorkflowStep[] = [
  { id: 'model', title: 'Model', body: 'Build meshes in a geometry graph, or import your own and clean them up.' },
  { id: 'surface', title: 'Surface', body: 'Layer materials in a shader graph that compiles to WGSL for every platform.' },
  { id: 'script', title: 'Script', body: 'Write gameplay in TypeScript, typed from your scene, with hot reload.' },
  { id: 'review', title: 'Review', body: 'Read the agent’s work as one diff, then keep it or undo it in a step.' },
  { id: 'playtest', title: 'Play-test', body: 'A headless run returns frames, logs and passed checks as proof it works.' },
  { id: 'ship', title: 'Ship', body: 'Export the same project to desktop, mobile and the web.' },
]
