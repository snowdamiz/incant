/** Genuine destinations only. The repository is private; links may ask for GitHub access. */
export const links = {
  repository: 'https://github.com/snowdamiz/incant',
  productGuide: 'https://github.com/snowdamiz/incant/blob/main/PLAN.md',
} as const

export interface NavItem {
  label: string
  href: string
}

export const navItems: NavItem[] = [
  { label: 'Features', href: '#features' },
  { label: 'Workflow', href: '#workflow' },
  { label: 'Trust', href: '#trust' },
  { label: 'FAQ', href: '#faq' },
]

export type WorkflowStepId = 'describe' | 'inspect' | 'playtest' | 'rewind'

export interface WorkflowStep {
  id: WorkflowStepId
  index: string
  label: string
  title: string
  body: string
  points: string[]
}

export const workflowSteps: WorkflowStep[] = [
  {
    id: 'describe',
    index: '01',
    label: 'Describe',
    title: 'Say what you want, in plain words.',
    body: 'Ask for a mechanic the way you would ask a teammate. The agent reads your scene through the same schemas the editor uses, so it never guesses at field names.',
    points: ['Works alongside your own clicks and drags', 'Plans against real component schemas'],
  },
  {
    id: 'inspect',
    index: '02',
    label: 'Inspect',
    title: 'Read every change before it lands.',
    body: 'Edits arrive as typed commands grouped into a single transaction. The project is a readable text document, so the change is a diff you can actually review.',
    points: ['Stable IDs, never fragile names', 'Validation errors point to exact paths'],
  },
  {
    id: 'playtest',
    index: '03',
    label: 'Play-test',
    title: 'Watch it play, not just compile.',
    body: 'The agent runs the game, looks at the viewport, reads the console and reports back with what it saw. You get evidence, not a hopeful summary.',
    points: ['Viewport captures and logs as tools', 'Results land next to the transaction'],
  },
  {
    id: 'rewind',
    index: '04',
    label: 'Rewind',
    title: 'Keep it, tweak it, or undo it.',
    body: 'Agent work sits in the same history as yours. One undo reverts the whole transaction, and provenance shows exactly who did what.',
    points: ['One history for people and agents', 'Undo is atomic, never partial'],
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
      'Incant is a game engine and editor in one application. You can build the way you would in a traditional editor, by clicking, dragging and writing scripts, or describe what you want and let an AI agent make the change in the same scene.',
  },
  {
    question: 'Does the AI change things without asking me?',
    answer:
      'Every agent action is a transaction made of typed commands, exactly like your own edits. You can review it in the history, amend it, or undo the whole thing in one step. Provenance records whether a change came from you, the agent, a script or an import.',
  },
  {
    question: 'Which AI model does it use?',
    answer:
      'Yours. You connect your own model provider account, starting with OpenAI. Model calls go directly from your machine to the provider, and keys are stored in your operating system keychain, never on Incant servers.',
  },
  {
    question: 'Can I use the editor offline?',
    answer:
      'Yes. The manual editor works fully offline with no Incant account, and no editing feature is gated behind a login. The agent needs a model connection, so it is available offline only when you connect a local model.',
  },
  {
    question: 'What languages do I write games in?',
    answer:
      'Gameplay is written in TypeScript, running in a sandboxed runtime with hot reload. The engine core is Rust. Scripts get no file system or network access unless your project grants that capability and you confirm it.',
  },
  {
    question: 'Where can I learn more?',
    answer:
      'The Incant repository on GitHub is the home of the project, including the full product guide that describes the architecture, the document model and the agent. The repository is private, so GitHub may ask you to sign in with an account that has access.',
  },
]
