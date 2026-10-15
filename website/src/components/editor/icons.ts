/* Small original 16 px glyphs for the editor recreation, drawn to match the real editor's icon weight. */
export type EdIconName =
  | 'scene'
  | 'group'
  | 'mesh'
  | 'light'
  | 'camera'
  | 'script'
  | 'chevron'
  | 'undo'
  | 'redo'
  | 'panel-left'
  | 'panel-bottom'
  | 'panel-right'
  | 'play'
  | 'step'
  | 'stop'
  | 'search'
  | 'check'
  | 'send'
  | 'move'
  | 'lock'
  | 'copy'
  | 'agent'
  | 'collapse'

export const edIconPaths: Record<EdIconName, string> = {
  scene: 'M8 2.5 14 5.5 8 8.5 2 5.5Z M2 8.2 8 11.2 14 8.2 M2 10.9 8 13.9 14 10.9',
  group: 'M8 3.2a4.8 4.8 0 1 0 0 9.6 4.8 4.8 0 1 0 0-9.6Z',
  mesh: 'M8 1.8 13.5 5v6L8 14.2 2.5 11V5Z M2.5 5 8 8.2 13.5 5 M8 8.2v6 M5.2 3.4l5.5 3.2v6.4 M10.8 3.4 5.3 6.6v6.4',
  light: 'M8 2a4 4 0 0 0-2.4 7.2c.5.4.8 1 .8 1.6v.7h3.2v-.7c0-.6.3-1.2.8-1.6A4 4 0 0 0 8 2Z M6.4 13.6h3.2',
  camera: 'M2 5h8.5v6H2Z M10.5 7.2 14 5.4v5.2l-3.5-1.8',
  script: 'M5.5 4 2 8l3.5 4 M10.5 4 14 8l-3.5 4',
  chevron: 'M5.5 6.5 8 9l2.5-2.5',
  undo: 'M5.5 3.5 2.5 6.5l3 3 M2.8 6.5h6.7a3.5 3.5 0 0 1 0 7H7',
  redo: 'M10.5 3.5l3 3-3 3 M13.2 6.5H6.5a3.5 3.5 0 0 0 0 7H9',
  'panel-left': 'M2.5 3h11v10h-11Z M6.5 3v10',
  'panel-bottom': 'M2.5 3h11v10h-11Z M2.5 9.5h11',
  'panel-right': 'M2.5 3h11v10h-11Z M9.5 3v10',
  play: 'M5 3.5v9l7-4.5Z',
  step: 'M4 3.5v9l6-4.5Z M12 3.5v9',
  stop: 'M4 4h8v8H4Z',
  search: 'M7 2.8a4.2 4.2 0 1 0 0 8.4 4.2 4.2 0 1 0 0-8.4Z M10 10l3.5 3.5',
  check: 'M3.5 8.5 6.5 11.5 12.5 4.5',
  send: 'M2.5 8 13.5 2.5 10 13.5 7.5 9Z M7.5 9l6-6.5',
  move: 'M8 1.5v13 M1.5 8h13 M6 3.5 8 1.5l2 2 M6 12.5l2 2 2-2 M3.5 6 1.5 8l2 2 M12.5 6l2 2-2 2',
  lock: 'M4 7.5h8v6H4Z M5.5 7.5V5.5a2.5 2.5 0 0 1 5 0v2',
  copy: 'M5.5 5.5h7.5v8H5.5Z M3 10.5V2.5h7.5',
  agent: 'M11.2 2.2c1.6.4 3 1.8 3 3.2 0 1.3-1.3 1.9-2.1 2.6-.8.7-.4 1.4.4 2.1 1 1 1.7 1.9 1.6 3.2-.1 1.8-2.1 3-4.4 3.3.5-.4.6-1-.3-1.3-1.8-.2-4.3-.5-5.4-2.2-1-1.5-.8-3.7.7-5.3 1.6-1.6 4.2-2.1 5.9-2.7 1.1-.4 1.4-1.3 1-2.1Z',
  collapse: 'M4 4l8 8M12 4l-8 8',
}
