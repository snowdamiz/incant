<script setup lang="ts">
import SectionHeading from './SectionHeading.vue'

/* The sweep script from the editor walkthrough, written against the real native script API (sdk/ts). */
const source = `import type { NativeBehavior } from '@incant/runtime/native'

/** Turn the beam once every \`period\` seconds. */
export default {
  update({ chunks, dt, instances }) {
    const period = Number(instances[0]?.props.period ?? 8)
    const step = (Math.PI * 2 * dt) / period
    for (const chunk of chunks) {
      for (let i = 0; i < chunk.ids.length; i++) {
        const o = i * chunk.transformStride
        const y = chunk.transforms[o + 4], w = chunk.transforms[o + 6]
        const angle = 2 * Math.atan2(y, w) + step
        chunk.transforms[o + 4] = Math.sin(angle / 2) // rotation y
        chunk.transforms[o + 6] = Math.cos(angle / 2) // rotation w
      }
    }
    return undefined
  },
} satisfies NativeBehavior`

type Tok = { text: string; cls: string }
const KEYWORDS = new Set(['import', 'type', 'from', 'export', 'default', 'const', 'let', 'for', 'of', 'return', 'satisfies', 'undefined'])
const TOKEN = /(\/\*\*[\s\S]*?\*\/|\/\/[^\n]*)|('[^']*'|`[^`]*`)|(\b\d+(?:\.\d+)?\b)|([A-Za-z_$][\w$]*)|(\s+)|([^\sA-Za-z_$\d]+)/g

function highlight(line: string): Tok[] {
  const out: Tok[] = []
  for (const m of line.matchAll(TOKEN)) {
    const [text, comment, str, num, word] = m
    let cls = 'text-ed-soft'
    if (comment) cls = 'text-ed-subtle italic'
    else if (str) cls = 'text-[#f2a983]'
    else if (num) cls = 'text-[#85b4ff]'
    else if (word && KEYWORDS.has(word)) cls = 'text-[#c79bff]'
    else if (word && /^[A-Z]/.test(word)) cls = 'text-[#7fe0a3]'
    else if (word && line[(m.index ?? 0) + word.length] === '(') cls = 'text-[#8fcdf5]'
    else if (!word) cls = 'text-ed-muted'
    out.push({ text, cls })
  }
  return out
}

const lines = source.split('\n').map(highlight)

const facts = [
  { title: 'Typed from your scene', body: 'Components, props and assets are checked as you write.' },
  { title: 'Hot reload while playing', body: 'Save a script and the running game picks it up.' },
  { title: 'WebAssembly for hot paths', body: 'Rust modules run sandboxed in the editor and ship as native code.' },
]
</script>

<template>
  <section id="script" aria-labelledby="script-title" class="border-t border-line py-24 sm:py-32">
    <div class="mx-auto max-w-[90rem] px-5 sm:px-8 lg:px-12">
      <SectionHeading id="script-title" index="04" label="Scripting" title="Gameplay in TypeScript. Hot paths in WebAssembly.">
        Write behavior the way the agent does: small typed scripts over bulk data, with no editor or document code in the
        shipped game.
      </SectionHeading>

      <div class="mt-16 grid gap-10 lg:grid-cols-12 lg:gap-12">
        <ul class="order-2 space-y-0 border-t border-line lg:order-1 lg:col-span-4">
          <li v-for="f in facts" :key="f.title" class="border-b border-line py-7">
            <h3 class="text-[17px] font-semibold tracking-[-0.015em]">{{ f.title }}</h3>
            <p class="text-pretty-safe mt-1.5 text-[15px] leading-relaxed text-muted">{{ f.body }}</p>
          </li>
        </ul>

        <figure class="order-1 min-w-0 lg:order-2 lg:col-span-8">
          <div class="overflow-hidden rounded-[14px] border border-line-strong bg-ed-app shadow-[0_50px_120px_-40px_rgb(0_0_0/0.9)]">
            <div class="flex h-10 items-end gap-1 border-b border-ed-border bg-ed-panel px-3 text-[12.5px]">
              <span class="rounded-t-md border border-b-0 border-ed-border bg-ed-app px-3 py-2 text-ed-text">sweep.ts</span>
              <span class="px-3 py-2 text-ed-subtle">bay.scene</span>
              <span class="ml-auto pb-2.5 font-mono text-[11.5px] text-ed-success">● Reloaded</span>
            </div>
            <pre
              class="code-lines overflow-x-auto py-4 font-mono text-[12.5px] leading-[1.75] sm:text-[13px]"
              tabindex="0"
              role="region"
              aria-label="sweep.ts source"
            ><code><span v-for="(line, n) in lines" :key="n" class="code-line"><span v-for="(tok, k) in line" :key="k" :class="tok.cls">{{ tok.text }}</span><template v-if="!line.length">&#8203;</template></span></code></pre>
          </div>
          <figcaption class="mt-4 text-[13px] text-muted">The beam script from the walkthrough above, against the native script API.</figcaption>
        </figure>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Line numbers are generated content, so they stay out of copied text and the accessibility tree. */
.code-lines code {
  counter-reset: line;
}
.code-line {
  display: block;
  padding-right: 1.5rem;
  white-space: pre;
}
.code-line::before {
  counter-increment: line;
  content: counter(line);
  display: inline-block;
  width: 3rem;
  padding-right: 1rem;
  text-align: right;
  color: rgb(149 156 171 / 0.55);
}
</style>
