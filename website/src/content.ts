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
  { label: 'How it works', href: '#how' },
  { label: 'Principles', href: '#principles' },
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
    body: 'Clicks, scripts and agent requests become the same kind of edit, in one timeline, with a name on each.',
  },
  {
    numeral: '02',
    title: 'Plain-text worlds',
    body: 'Scenes and prefabs are readable documents. Review a change the way you review code, then keep it.',
  },
  {
    numeral: '03',
    title: 'Evidence, not guesses',
    body: 'The agent plays the game, watches the viewport and reads the console before it says it is done.',
  },
  {
    numeral: '04',
    title: 'Yours to run',
    body: 'Bring your own model account. The editor works offline, with no Incant account required.',
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
      'A game engine and editor in one application. Build by hand the way you would in any editor, or describe what you want and let an AI agent make the change in the same scene.',
  },
  {
    question: 'Can the agent change things without me?',
    answer:
      'Every agent action is a transaction in the same history as your own edits. You can inspect it, amend it, or undo the whole thing in one step, and each entry records who made it.',
  },
  {
    question: 'Which AI model does it use?',
    answer:
      'Yours. Connect your own model provider account, starting with OpenAI. Calls go directly from your machine to the provider, and keys stay in your operating system keychain.',
  },
  {
    question: 'Does it work offline?',
    answer:
      'The editor works fully offline with no Incant account, and no editing feature sits behind a login. The agent needs a model connection, or a local model if you prefer to stay offline.',
  },
  {
    question: 'What do I write gameplay in?',
    answer:
      'TypeScript, with hot reload, in a sandbox that has no file or network access unless your project asks and you agree. The engine core is written in Rust.',
  },
]
