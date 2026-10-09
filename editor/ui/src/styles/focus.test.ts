import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

/** Selector lists (text before each `{`), with comments removed. */
function selectorLists(css: string): string[] {
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, '');
  return [...clean.matchAll(/([^{}]+)\{/g)].map((match) => match[1]!.trim()).filter((s) => !s.startsWith('@'));
}

// WebKit draws no :focus-visible ring after script focus (src/shell/inputModality.ts), so
// every :focus-visible selector needs a [data-focus-visible] twin.
describe.each(['app.css', 'assets.css'])('%s keyboard focus twins', (file) => {
  it('pairs every :focus-visible selector with a [data-focus-visible] twin', () => {
    const css = readFileSync(join(process.cwd(), 'src/styles', file), 'utf8');
    const all = new Set(selectorLists(css).flatMap((list) => list.split(',').map((s) => s.trim())));
    const missing = [...all].filter(
      (selector) => selector.includes(':focus-visible') && !all.has(selector.replaceAll(':focus-visible', '[data-focus-visible]')),
    );
    expect(missing).toEqual([]);
  });
});
