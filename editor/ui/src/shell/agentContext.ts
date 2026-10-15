import type { AgentEditorPanel, AgentViewContext, Ulid } from '../bridge/contract';
import type { PanelVisibility } from '../components/Titlebar';
import type { DockTab } from './ShellContext';

/** Names of currently visible work surfaces only; no DOM, pixels or user text. */
export function agentViewContext(
  project: Ulid | undefined,
  panels: PanelVisibility,
  navigator: 'hierarchy' | 'assets',
  output: DockTab,
  agentOpen: boolean,
): AgentViewContext | undefined {
  if (!project) return undefined;
  const open: AgentEditorPanel[] = ['viewport'];
  if (panels.hierarchy) open.push(navigator);
  if (panels.inspector) {
    open.push('inspector');
    if (agentOpen) open.push('agent');
  }
  if (panels.dock) open.push(output);
  return { project_id: project, open_panels: open };
}
