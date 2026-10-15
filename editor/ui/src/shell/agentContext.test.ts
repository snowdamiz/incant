import { describe, expect, it } from 'vitest';
import type { Ulid } from '../bridge/contract';
import { agentViewContext } from './agentContext';

describe('agent view observations', () => {
  const project = '01J00000000000000000000000' as Ulid;
  it('reports the visible navigator and output tab, never every available tab', () => {
    const panels = { hierarchy: true, inspector: true, dock: true };
    expect(agentViewContext(project, panels, 'assets', 'console', true)).toEqual({
      project_id: project, open_panels: ['viewport', 'assets', 'inspector', 'agent', 'console'],
    });
    expect(agentViewContext(project, panels, 'hierarchy', 'history', false)?.open_panels)
      .toEqual(['viewport', 'hierarchy', 'inspector', 'history']);
  });
  it('omits hidden panels and never borrows a previous project identity', () => {
    const panels = { hierarchy: false, inspector: false, dock: false };
    expect(agentViewContext(project, panels, 'assets', 'problems', true)?.open_panels).toEqual(['viewport']);
    expect(agentViewContext(undefined, panels, 'assets', 'problems', true)).toBeUndefined();
  });
});
