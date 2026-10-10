import { render, screen, within } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { Diagnostic } from '../../bridge/contract';
import { NumberListView } from './ValueLists';

it('keeps exact costs and their diagnostic accessible when a collapsed grid becomes invalid', () => {
  // Canvas drawing is unrelated to this read-only values/navigation regression.
  const canvas = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  const values = [1.25, 2.5, 3.75, 4.125];
  const diagnostic: Diagnostic = {
    id: 'grid-size', entity: null, component: 'NavigationGrid', path: '/costs',
    severity: 'error', message: 'Dimensions do not match the cost count.',
  };
  const show = (dimensions: number[], invalid = false) => <NumberListView
    label="Costs" schema={{ type: 'array', items: { type: 'number' } }}
    hint={{ grid: { size: '/dimensions', blocked: 0 } }} value={values} path={['costs']}
    ctx={{ entity: 'test', component: 'NavigationGrid', diagnostics: invalid ? [diagnostic] : [], value: { dimensions } }}
  />;
  const view = render(show([2, 2]));
  try {
    expect(screen.getByRole('button', { name: 'Show values' }).getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('group', { name: 'Costs' })).toBeNull();
    view.rerender(show([3, 3], true));
    expect(screen.queryByRole('button', { name: 'Show values' })).toBeNull();
    const table = screen.getByRole('group', { name: 'Costs' });
    expect(table.getAttribute('aria-invalid')).toBe('true');
    expect(screen.getByText(diagnostic.message)).toBeTruthy();
    values.forEach((value, index) => expect(within(table).getByTitle(`[${index}] ${value}`).textContent).toBe(String(value)));
    // Returning to a valid grid preserves the user's original collapsed choice.
    view.rerender(show([2, 2]));
    expect(screen.getByRole('button', { name: 'Show values' }).getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('group', { name: 'Costs' })).toBeNull();
    expect(screen.queryByText(diagnostic.message)).toBeNull();
  } finally {
    view.unmount();
    canvas.mockRestore();
  }
});
