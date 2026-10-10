import { fireEvent, render, screen, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import { createFixtureBridge } from '../bridge/fixture';

const renderSample = () => render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
const problems = () => screen.getByRole('list', { name: 'Problems' });
const headings = () => [...problems().querySelectorAll('.problems__heading-name')].map((node) => node.textContent);

describe('Problems', () => {
  it('groups problems under their entity, worst first, with project problems last', async () => {
    const { container } = renderSample();
    expect(headings()).toEqual(['Crate 03', 'Grapple Controller', 'Pier Planks', 'Broken Railing', 'Project']);
    const railing = within(problems()).getByRole('list', { name: /Broken Railing/ });
    expect(within(railing).getAllByRole('button')).toHaveLength(3);
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => v.id)).toEqual([]);
  });

  it('filters by severity and says when everything is filtered out', () => {
    renderSample();
    const filters = screen.getByRole('group', { name: 'Show problem severities' });
    const errors = within(filters).getByRole('button', { name: /^Errors/ });
    expect(errors.getAttribute('aria-pressed')).toBe('true');
    expect(errors.textContent).toContain('6');
    fireEvent.click(errors);
    expect(errors.getAttribute('aria-pressed')).toBe('false');
    // Only the warning on Grapple Controller and Pier Planks, and the project info, remain.
    expect(headings()).toEqual(['Grapple Controller', 'Pier Planks', 'Project']);
    fireEvent.click(within(filters).getByRole('button', { name: /^Warnings/ }));
    fireEvent.click(within(filters).getByRole('button', { name: /^Info/ }));
    expect(screen.getByText('All problems are filtered out')).toBeTruthy();
    fireEvent.click(errors);
    expect(headings()[0]).toBe('Crate 03');
  });
});

describe('History', () => {
  it('draws the timeline with a Now marker between applied and undone edits', () => {
    renderSample();
    fireEvent.click(screen.getByRole('tab', { name: /^History/ }));
    const list = screen.getByRole('list', { name: 'Edit history, oldest first' });
    const items = [...list.children];
    const now = items.findIndex((item) => item.getAttribute('aria-label') === 'Current position');
    // Four of five transactions are applied: the marker follows the fourth.
    expect(now).toBe(4);
    expect(items[now + 1]?.textContent).toContain('(undone)');
    expect(items[now - 1]?.textContent).not.toContain('(undone)');
    expect(screen.getByText('4 of 5 applied')).toBeTruthy();
  });
});

describe('Console', () => {
  it('lists messages as rows of time, level, source and message, filtered by level', () => {
    renderSample();
    fireEvent.click(screen.getByRole('tab', { name: /^Console/ }));
    const log = screen.getByRole('log', { name: 'Console messages' });
    // Debug is hidden by default.
    expect(within(log).getAllByRole('listitem')).toHaveLength(4);
    fireEvent.click(screen.getByRole('button', { name: /^Debug/ }));
    expect(within(log).getAllByRole('listitem')).toHaveLength(5);
    const row = within(log).getAllByRole('listitem')[0]!;
    expect(row.querySelector('time')?.getAttribute('dateTime')).toBeTruthy();
    expect(row.querySelector('.console__level')?.textContent).toBe('info');
  });
});
