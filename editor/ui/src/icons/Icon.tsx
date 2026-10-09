/**
 * Incant icon set, Phase 0. 16 px grid, 1.5 px stroke, round caps, currentColor.
 * Drawn for this shell; no external icon font or network fetch. Icons are
 * decorative (aria-hidden); every control carries its own text label.
 */
const PATHS = {
  chevron: 'M6 4l4 4-4 4',
  scene: 'M8 2.5l5.5 3L8 8.5l-5.5-3zM2.5 8.25L8 11.25l5.5-3M2.5 10.75L8 13.75l5.5-3',
  group: 'M2 4.5h4.5l1.5 1.5h6v6.5H2z',
  camera: 'M2 5h8.5v6H2zM10.5 7l3.5-2v6l-3.5-2',
  light: 'M8 2v1.5M8 12.5V14M2 8h1.5M12.5 8H14M3.8 3.8l1 1M11.2 11.2l1 1M3.8 12.2l1-1M11.2 4.8l1-1M8 5.5a2.5 2.5 0 110 5 2.5 2.5 0 010-5z',
  mesh: 'M8 2l5.5 3.2v5.6L8 14l-5.5-3.2V5.2zM8 2v12M2.5 5.2L13.5 10.8M13.5 5.2L2.5 10.8',
  prefab: 'M3 3h7l3 3v7H3zM10 3v3h3',
  script: 'M6 4.5L2.5 8 6 11.5M10 4.5l3.5 3.5-3.5 3.5',
  audio: 'M2.5 6h2.5L8.5 3v10L5 10H2.5zM11 5.5a3.5 3.5 0 010 5M12.8 3.8a6 6 0 010 8.4',
  entity: 'M8 3a5 5 0 110 10 5 5 0 010-10z',
  error: 'M8 2a6 6 0 110 12A6 6 0 018 2zM8 5v3.5M8 10.8v.2',
  warning: 'M8 2.2l6 10.8H2zM8 6.5v3M8 11.2v.2',
  info: 'M8 2a6 6 0 110 12A6 6 0 018 2zM8 7.2v4M8 4.8v.2',
  search: 'M7 3a4 4 0 110 8 4 4 0 010-8zM10 10l3.5 3.5',
  undo: 'M5 6.5H10a3 3 0 010 6H6.5M5 6.5l2.5-2.5M5 6.5L7.5 9',
  redo: 'M11 6.5H6a3 3 0 000 6h3.5M11 6.5L8.5 4M11 6.5L8.5 9',
  plug: 'M6 2v3M10 2v3M4.5 5h7v2.5a3.5 3.5 0 01-7 0zM8 11v3',
  spark: 'M8 2l1.4 4.6L14 8l-4.6 1.4L8 14l-1.4-4.6L2 8l4.6-1.4z',
  keyboard: 'M2 4.5h12v7H2zM4.5 7h1M7.5 7h1M10.5 7h1M5 9.5h6',
  close: 'M4 4l8 8M12 4l-8 8',
  check: 'M3.5 8.5l3 3 6-7',
  sidebarLeft: 'M2.5 3.5h11v9h-11zM6.5 3.5v9',
  sidebarRight: 'M2.5 3.5h11v9h-11zM9.5 3.5v9',
  panelBottom: 'M2.5 3.5h11v9h-11zM2.5 9h11',
  minimize: 'M3.5 8h9',
  maximize: 'M3.5 3.5h9v9h-9z',
  restore: 'M5.5 3.5h7v7M3.5 5.5h7v7h-7z',
  lock: 'M4.5 7.5h7v5.5h-7zM6 7.5V5.75a2 2 0 014 0V7.5',
  copy: 'M5.5 5.5h7v7h-7zM3.5 10.5v-7h7',
  collapseAll: 'M5 3l3 3 3-3M5 13l3-3 3 3',
  viewport: 'M2 3.5h12v9H2zM2 6h12',
  history: 'M3 8a5 5 0 105-5 5 5 0 00-4 2M3 3v2.5h2.5M8 5.5V8l2 1.5',
  console: 'M2 3.5h12v9H2zM4.5 6.5L6.5 8l-2 1.5M8 10h3.5',
  problems: 'M3 2.5h10v11H3zM5.5 5.5h5M5.5 8h5M5.5 10.5h3',
  link: 'M6.5 9.5l3-3M7 4.5l1-1a2.5 2.5 0 013.5 3.5l-1 1M9 11.5l-1 1A2.5 2.5 0 014.5 9l1-1',
  send: 'M2.5 8L13.5 2.5 11 13.5 8 9zM8 9l5.5-6.5',
} as const;

export type IconName = keyof typeof PATHS;

const KIND_ICONS: Record<string, IconName> = {
  scene: 'scene',
  group: 'group',
  camera: 'camera',
  light: 'light',
  mesh: 'mesh',
  prefab: 'prefab',
  script: 'script',
  audio: 'audio',
};

export function iconForKind(kind: string): IconName {
  return KIND_ICONS[kind] ?? 'entity';
}

export function Icon({ name, size = 16, className }: { name: IconName; size?: number; className?: string }) {
  return (
    <svg
      className={className ? `icon ${className}` : 'icon'}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
