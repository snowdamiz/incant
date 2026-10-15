/**
 * DETERMINISTIC DUPLICATE FIXTURE DATA for handoff 0078. Invented content, not engine
 * output. The project is handoff 0033's harbor review double (an invented scene with the
 * real component schemas), renamed, with two additions: a linked prefab instance, which
 * the bus refuses to copy, and a long group for the bounded-rendering case. The host
 * advertises duplication exactly as the bridge requires (`entity_duplicate`), except on
 * the older host.
 */
import { reviewRead, ulid } from '../../0033-editor-design-review/tools/review-double.mjs';

const transform = (translation) => ({ translation, rotation: [0, 0, 0, 1], scale: [1, 1, 1] });

function harbor({ long = false, ...overrides } = {}) {
  const { read } = reviewRead();
  const scene = Object.values(read.project.scenes)[0];
  const props = Object.values(scene.entities).find((entity) => entity.name === 'Props').id;
  const entities = { ...scene.entities };
  let serial = 7000;
  const add = (name, parent, extra = {}) => {
    const id = ulid((serial += 1));
    entities[id] = { id, name, parent, components: { Transform: transform([0, 0, 0]) }, ...extra };
    return id;
  };
  // Only presence matters here: the bus refuses any entity with a prefab link.
  add('Lamp Post (prefab instance)', props, { prefab: { asset: ulid(7900), source: ulid(7901) } });
  if (long) {
    const bollards = add('Bollards', null);
    for (let n = 1; n <= 600; n += 1) add(`Bollard ${String(n).padStart(3, '0')}`, bollards);
  }
  return {
    ...read,
    project: { ...read.project, name: 'Harbor (duplicate fixture)', scenes: { [scene.id]: { ...scene, entities } } },
    revision: 7,
    viewport_error: 'Duplicate fixture: a browser page has no native surface.',
    entity_duplicate: { available: true },
    ...overrides,
  };
}

const WINDOWS = { platform: 'windows', controls: 'custom', maximized: false, fullscreen: false, focused: true, leadingInset: 0, trailingInset: 0 };
const SLOW = 600_000;

export const SCENARIOS = {
  harbor: { engine: harbor() },
  long: { engine: harbor({ long: true }) },
  slow: { engine: harbor(), delay: SLOW },
  unverified: { engine: harbor(), unverified: true },
  'older-host': { engine: harbor({ entity_duplicate: undefined }) },
  windows: { engine: harbor(), chrome: WINDOWS },
};
