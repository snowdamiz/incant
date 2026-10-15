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
  duplicate: 'M3 10.5V3h7.5M5.5 5.5h8v8h-8zM9.5 8v3M8 9.5h3',
  collapseAll: 'M5 3l3 3 3-3M5 13l3-3 3 3',
  viewport: 'M2 3.5h12v9H2zM2 6h12',
  history: 'M3 8a5 5 0 105-5 5 5 0 00-4 2M3 3v2.5h2.5M8 5.5V8l2 1.5',
  console: 'M2 3.5h12v9H2zM4.5 6.5L6.5 8l-2 1.5M8 10h3.5',
  problems: 'M3 2.5h10v11H3zM5.5 5.5h5M5.5 8h5M5.5 10.5h3',
  link: 'M6.5 9.5l3-3M7 4.5l1-1a2.5 2.5 0 013.5 3.5l-1 1M9 11.5l-1 1A2.5 2.5 0 014.5 9l1-1',
  send: 'M2.5 8L13.5 2.5 11 13.5 8 9zM8 9l5.5-6.5',
  external: 'M9 2.5h4.5V7M13.5 2.5L7.5 8.5M11.5 9.5v4h-9v-9h4',
  plus: 'M8 3.5v9M3.5 8h9',
  signOut: 'M6.5 2.5h-4v11h4M10 5l3 3-3 3M13 8H6',
  model: 'M8 2l5.5 3v6L8 14l-5.5-3V5zM2.5 5L8 8l5.5-3M8 8v6',
  texture: 'M2.5 2.5h11v11h-11zM2.5 11l3.5-3.5 3 3 1.5-1.5 3 3M10.5 5.5v.01',
  import: 'M8 2.5v7M5 6.5l3 3 3-3M2.5 10v3.5h11V10',
  reimport: 'M13 8a5 5 0 01-8.6 3.5M3 8a5 5 0 018.6-3.5M11.5 2v2.6H8.9M4.5 14v-2.6h2.6',
  back: 'M13 8H3.5M7 4.5L3.5 8 7 11.5',
  // Pager: first, previous, next and last page.
  pageFirst: 'M11 4L7 8l4 4M4.5 4v8',
  pagePrev: 'M10 4L6 8l4 4',
  pageNext: 'M6 4l4 4-4 4',
  pageLast: 'M5 4l4 4-4 4M11.5 4v8',
  // Play transport: play (and resume), pause, one-tick step, stop.
  play: 'M5 3.25l7.5 4.75L5 12.75z',
  pause: 'M5.75 3.5v9M10.25 3.5v9',
  step: 'M3.75 3.25l6.5 4.75-6.5 4.75zM12.25 3.5v9',
  stop: 'M4 4h8v8H4z',
  refresh: 'M13 8a5 5 0 01-8.6 3.5M3 8a5 5 0 018.6-3.5M11.5 2v2.6H8.9M4.5 14v-2.6h2.6',
  pencil: 'M10.25 3.25l2.5 2.5L6 12.5H3.5V10zM8.75 4.75l2.5 2.5',
  // Project file menu: a disk for Save, a pushpin for pinning the engine version.
  save: 'M2.5 2.5h8.25l2.75 2.75v8.25h-11zM5 2.5v3h5v-3M5 13.5v-4h6v4',
  pin: 'M5.5 2.5h5M6.5 2.5V6L4.5 9h7l-2-3V2.5M8 9v4.5',
  trash: 'M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.75 9h5.5l.75-9M6.75 7v4.25M9.25 7v4.25',
  // Component marks for Inspector section headers.
  move: 'M8 2v12M2 8h12M6.25 3.75L8 2l1.75 1.75M6.25 12.25L8 14l1.75-1.75M3.75 6.25L2 8l1.75 1.75M12.25 6.25L14 8l-1.75 1.75',
  // A moving ball with speed lines: a body the solver moves.
  body: 'M10 4a4 4 0 110 8 4 4 0 010-8zM2 6h3M2.75 8.5h2.5M2 11h3',
  collider: 'M2.5 5.5v-3h3M10.5 2.5h3v3M13.5 10.5v3h-3M5.5 13.5h-3v-3M6 6h4v4H6z',
  bulb: 'M8 2.25a4 4 0 00-2.4 7.2c.5.4.9 1 .9 1.55v.5h3v-.5c0-.55.4-1.15.9-1.55A4 4 0 008 2.25zM6.5 13.75h3',
  spot: 'M6 2.5h4v2.25L13 13H3l3-8.25zM6 4.75h4',
  globe: 'M8 2a6 6 0 110 12A6 6 0 018 2zM2 8h12M8 2c-1.9 1.7-2.6 3.7-2.6 6s.7 4.3 2.6 6M8 2c1.9 1.7 2.6 3.7 2.6 6s-.7 4.3-2.6 6',
  mixer: 'M4 2.5v11M8 2.5v11M12 2.5v11M2.5 10h3M6.5 5.5h3M10.5 8h3',
  velocity: 'M5.5 8h8M10.5 5l3 3-3 3M2.5 5.5h3M2.5 10.5h3',
  spin: 'M13.2 8.6A5.25 5.25 0 113 6.7M13.5 3v3.25h-3.25M13.4 6.1A5.25 5.25 0 008 2.75',
  grid: 'M2.5 2.5h11v11h-11zM2.5 6.17h11M2.5 9.83h11M6.17 2.5v11M9.83 2.5v11',
  navmesh: 'M2.5 12.5L5.25 3.5l8.25 3-1.75 6.5zM5.25 3.5l6.5 9.5M2.5 12.5l11-6',
  // An arrow pointer: nothing is selected yet.
  pointer: 'M4 2.75l8.5 5.25-3.9.9 2.15 4.1-1.5.8-2.15-4.1L4 12.25z',
  // A head and shoulders: the account button while no account is signed in.
  profile: 'M8 2.75a2.75 2.75 0 110 5.5 2.75 2.75 0 010-5.5zM3 13.25c.6-2.3 2.6-3.75 5-3.75s4.4 1.45 5 3.75',
  // A puzzle piece: a component type the Inspector has no profile for.
  component: 'M3 4.5h3a1.5 1.5 0 013 0h3v3a1.5 1.5 0 010 3v3H3z',
  // Collider primitives, for shape rows and compound part lists.
  shapeBox: 'M3 5.25L8 2.75l5 2.5v5.5l-5 2.5-5-2.5zM3 5.25L8 7.75l5-2.5M8 7.75v5.5',
  shapeSphere: 'M8 2.75a5.25 5.25 0 110 10.5 5.25 5.25 0 010-10.5zM2.75 8c1.3 1.05 3.1 1.6 5.25 1.6s3.95-.55 5.25-1.6',
  // Wide, with the top cap's rim drawn like the sphere's equator, so it reads as a solid
  // and never as a zero beside a part's index.
  shapeCapsule: 'M4.75 6a3.25 3.25 0 016.5 0v4a3.25 3.25 0 01-6.5 0zM4.75 6c.9.7 2 1 3.25 1s2.35-.3 3.25-1',
  shapeCompound: 'M2.5 9.5h5v4h-5zM9.75 4.25a2.25 2.25 0 110 4.5 2.25 2.25 0 010-4.5zM4 7.5V5a2 2 0 014 0',
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
  model: 'model',
  texture: 'texture',
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
