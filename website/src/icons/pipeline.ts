/*
 * Pipeline glyphs for the workflow diagram.
 * Source: Lucide 1.53.0 (lucide-static), https://lucide.dev. License: ISC,
 * Copyright (c) 2026 Lucide Icons and Contributors (copy in public/licenses/lucide-ISC.txt).
 * Elements are copied unmodified; they render at viewBox 0 0 24 24 with a 2px round stroke.
 */
export interface GlyphElement {
  tag: 'path' | 'circle' | 'line' | 'polyline'
  attrs: Record<string, string>
}

export const pipelineGlyphs: Record<string, GlyphElement[]> = {
  model: [
    { tag: 'path', attrs: {"d": "M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"} },
    { tag: 'path', attrs: {"d": "m3.3 7 8.7 5 8.7-5"} },
    { tag: 'path', attrs: {"d": "M12 22V12"} },
  ],
  surface: [
    { tag: 'circle', attrs: {"cx": "15", "cy": "9", "r": "7"} },
    { tag: 'circle', attrs: {"cx": "9", "cy": "15", "r": "7"} },
  ],
  script: [
    { tag: 'path', attrs: {"d": "M8 3H7a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2 2 2 0 0 1 2 2v5c0 1.1.9 2 2 2h1"} },
    { tag: 'path', attrs: {"d": "M16 21h1a2 2 0 0 0 2-2v-5c0-1.1.9-2 2-2a2 2 0 0 1-2-2V5a2 2 0 0 0-2-2h-1"} },
  ],
  review: [
    { tag: 'path', attrs: {"d": "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"} },
    { tag: 'path', attrs: {"d": "M9 10h6"} },
    { tag: 'path', attrs: {"d": "M12 13V7"} },
    { tag: 'path', attrs: {"d": "M9 17h6"} },
  ],
  playtest: [
    { tag: 'line', attrs: {"x1": "6", "x2": "10", "y1": "11", "y2": "11"} },
    { tag: 'line', attrs: {"x1": "8", "x2": "8", "y1": "9", "y2": "13"} },
    { tag: 'line', attrs: {"x1": "15", "x2": "15.01", "y1": "12", "y2": "12"} },
    { tag: 'line', attrs: {"x1": "18", "x2": "18.01", "y1": "10", "y2": "10"} },
    { tag: 'path', attrs: {"d": "M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"} },
  ],
  ship: [
    { tag: 'path', attrs: {"d": "M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z"} },
    { tag: 'path', attrs: {"d": "M12 22V12"} },
    { tag: 'polyline', attrs: {"points": "3.29 7 12 12 20.71 7"} },
    { tag: 'path', attrs: {"d": "m7.5 4.27 9 5.15"} },
  ],
}
