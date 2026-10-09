import type { BridgeSnapshot, EditorBridge, ProjectAsset } from '../bridge/contract';
import { unavailableMessage } from '../bridge/resolve';
import type { CapabilitySet } from '../bridge/resolve';

export type ImportAvailability = { readonly ok: true } | { readonly ok: false; readonly reason: string };

/** Whether this host can import now, and if not, the host's own reason. */
export function importAvailability(
  snapshot: BridgeSnapshot | null,
  bridge: EditorBridge | null,
  capabilities: CapabilitySet,
): ImportAvailability {
  if (!capabilities.has('asset.import')) return { ok: false, reason: unavailableMessage('asset.import', bridge) };
  const state = snapshot?.assetImport;
  if (!state) return { ok: false, reason: 'The connected engine did not report whether importing is available.' };
  if (!state.available) return { ok: false, reason: state.reason ?? 'The connected engine cannot import assets right now.' };
  return { ok: true };
}

export const assetRowId = (id: string) => `asset-row-${id}`;
export const countFiles = (count: number) => `${count} ${count === 1 ? 'file' : 'files'}`;

export function matchesFilter(asset: ProjectAsset, filter: string): boolean {
  const needle = filter.trim().toLowerCase();
  return !needle || asset.name.toLowerCase().includes(needle) || asset.path.toLowerCase().includes(needle);
}

export interface AssetFolder {
  /** Folder path without the trailing slash; '' for files at the project root. */
  readonly folder: string;
  readonly assets: readonly ProjectAsset[];
}

/**
 * Groups assets by their source folder, folders and files in path order. Grouping by
 * folder keeps the list short and readable in a narrow column: the folder is shown
 * once, so each row only needs its name.
 */
export function groupByFolder(assets: readonly ProjectAsset[]): AssetFolder[] {
  const sorted = [...assets].sort((a, b) => a.path.localeCompare(b.path) || a.id.localeCompare(b.id));
  const groups = new Map<string, ProjectAsset[]>();
  for (const asset of sorted) {
    const slash = asset.path.lastIndexOf('/');
    const folder = slash < 0 ? '' : asset.path.slice(0, slash);
    const group = groups.get(folder);
    if (group) group.push(asset);
    else groups.set(folder, [asset]);
  }
  return [...groups].map(([folder, list]) => ({ folder, assets: list }));
}

/**
 * A folder label for a narrow column: short paths as they are, long ones as their last
 * folders after an ellipsis, so the most specific part stays readable. The full path is
 * kept for the tooltip and the Inspector.
 */
export function folderLabel(folder: string, max = 26): string {
  if (!folder) return 'Project folder';
  if (folder.length + 1 <= max) return `${folder}/`;
  const parts = folder.split('/');
  let tail = parts.pop() ?? '';
  while (parts.length > 0 && parts[parts.length - 1]!.length + tail.length + 4 <= max) tail = `${parts.pop()}/${tail}`;
  return `…/${tail}/`;
}
