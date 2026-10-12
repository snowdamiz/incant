import { describe, expect, it } from 'vitest';
import type { AgentSessionSnapshot, ConnectionState, Ulid } from '../../bridge/contract';
import { MAX_ALLOWANCE, allowanceOf, allowanceSpoken, allowanceText, allowanceTitle, parseAllowance, problemText } from './tokenAllowance';

const READY: ConnectionState = { status: 'ready', project: { id: '01JA45FX0000000000000PROJ1' as Ulid, name: 'Harbor' } };
const session = (used: number, max = 200_000, uncertain = false): AgentSessionSnapshot => ({
  version: 1,
  budget: { max_tokens: max, used_tokens: used },
  accounting_uncertain: uncertain,
  worker_running: false,
  turns: [],
});

describe('reading a typed allowance', () => {
  it.each([
    ['0', 0],
    ['250000', 250_000],
    ['  42  ', 42],
    ['007', 7],
    ['1,000,000', 1_000_000],
    ['1 000 000', 1_000_000],
    ['1_000_000', 1_000_000],
    ['1’000’000', 1_000_000],
    ['10,00,000', 1_000_000],
    ['12,34,567', 1_234_567],
    ['１２３', 123],
    ['9007199254740991', MAX_ALLOWANCE],
    ['9,007,199,254,740,991', MAX_ALLOWANCE],
  ])('accepts %j as %d', (text, value) => {
    expect(parseAllowance(text, 'en-US')).toEqual({ ok: true, value });
  });

  it.each([
    ['', 'empty'],
    ['   ', 'empty'],
    ['-1', 'negative'],
    ['−5', 'negative'],
    ['-0', 'negative'],
    ['1.5', 'format'],
    ['0.0', 'format'],
    ['1,5', 'format'],
    ['1,0000', 'format'],
    ['1,000.000', 'format'],
    ['1,000,00,000', 'format'],
    ['12,345,67,890', 'format'],
    ['100,00,000', 'format'],
    ['1e6', 'format'],
    ['+5', 'format'],
    ['ten', 'format'],
    ['NaN', 'format'],
    ['Infinity', 'format'],
    ['9007199254740992', 'unsafe'],
    ['99999999999999999999', 'unsafe'],
  ])('refuses %j (%s) without rounding or clamping', (text, problem) => {
    expect(parseAllowance(text, 'en-US')).toEqual({ ok: false, problem });
  });

  it('never reads the locale’s decimal mark as a digit group', () => {
    // A comma is the decimal mark in German: "1,000" is a fraction there, "1.000" a thousand.
    expect(parseAllowance('1,000', 'de-DE')).toEqual({ ok: false, problem: 'format' });
    expect(parseAllowance('1.000.000', 'de-DE')).toEqual({ ok: true, value: 1_000_000 });
    expect(parseAllowance('1.000', 'en-US')).toEqual({ ok: false, problem: 'format' });
  });

  it('says what is wrong in plain words', () => {
    expect(problemText('empty')).toBe('Enter a number of tokens.');
    expect(problemText('negative')).toBe('The allowance can’t be negative.');
    expect(problemText('format')).toBe('Enter a whole number of tokens.');
    expect(problemText('unsafe')).toBe(`The largest allowance is ${MAX_ALLOWANCE.toLocaleString()} tokens.`);
  });
});

describe('the allowance shown', () => {
  it('is the project’s authored cap where the host has one, never the session’s lagging copy', () => {
    const allowance = allowanceOf({ connection: READY, projectAgentBudget: { maxTokens: 500_000 }, agentSession: session(6_532, 200_000) });
    expect(allowance).toMatchObject({ scope: 'project', cap: 500_000, used: 6_532, left: 493_468, exhausted: false, level: 'normal' });
    expect(allowanceText(allowance!)).toBe('6,532 of 500,000 tokens used');
  });

  it('is the session’s own budget on an older host without a project cap', () => {
    const allowance = allowanceOf({ connection: READY, agentSession: session(170_000, 200_000, true) });
    expect(allowance).toMatchObject({ scope: 'session', cap: 200_000, level: 'low', uncertain: true });
    expect(allowanceText(allowance!)).toBe('170,000 of 200,000 tokens used (some estimated)');
    expect(allowanceTitle(allowance!)).toContain('belongs to the current agent session');
  });

  it('keeps usage above a lowered cap visible and clamps only what is left', () => {
    const allowance = allowanceOf({ connection: READY, projectAgentBudget: { maxTokens: 5_000 }, agentSession: session(6_532) })!;
    expect(allowance).toMatchObject({ used: 6_532, left: 0, exhausted: true, level: 'out' });
    expect(allowanceText(allowance)).toBe('6,532 of 5,000 tokens used');
    expect(allowanceSpoken(allowance)).toBe('0 tokens left.');
  });

  it('treats 0 as turned off, not as an alarm', () => {
    const allowance = allowanceOf({ connection: READY, projectAgentBudget: { maxTokens: 0 }, agentSession: session(40) })!;
    expect(allowance).toMatchObject({ exhausted: true, level: 'off' });
    expect(allowanceText(allowance)).toBe('40 of 0 tokens used');
  });

  it('shows the authored cap alone before the session has been read', () => {
    const allowance = allowanceOf({ connection: READY, projectAgentBudget: { maxTokens: 1_000_000 } })!;
    expect(allowance).toMatchObject({ used: null, left: null, exhausted: false });
    expect(allowanceText(allowance)).toBe('Allowance: 1,000,000 tokens');
    expect(allowanceSpoken(allowance)).toBe('');
  });

  it('describes uncertain usage without naming a cause, and never as money', () => {
    const allowance = allowanceOf({ connection: READY, projectAgentBudget: { maxTokens: 1_000_000 }, agentSession: session(1_000_000, 1_000_000, true) })!;
    const words = `${allowanceSpoken(allowance)} ${allowanceTitle(allowance)}`;
    expect(words).toContain('could not be confirmed or saved exactly, so a conservative amount was recorded');
    expect(words).toContain('kept across restarts and account changes');
    expect(words).not.toMatch(/\$|dollar|cost|price|bill/i);
  });

  it('is absent without an open project or anything to show', () => {
    expect(allowanceOf(null)).toBeNull();
    expect(allowanceOf({ connection: { status: 'connecting' }, projectAgentBudget: { maxTokens: 10 }, agentSession: session(1) })).toBeNull();
    expect(allowanceOf({ connection: READY })).toBeNull();
  });
});
