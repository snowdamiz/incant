import type { Command } from '../src/index';

const copy = {
  op: 'duplicate_subtree',
  scene_id: 'scene',
  root_entity_id: 'source',
  entities: { source: 'copy', child: 'copied-child' },
} satisfies Command;
// @ts-expect-error The host must supply explicit identities for every copied node.
const missing = { op: 'duplicate_subtree', scene_id: 'scene', root_entity_id: 'source' } satisfies Command;
// @ts-expect-error IDs are strings; an ordinal is not an entity identity.
const ordinal = { op: 'duplicate_subtree', scene_id: 'scene', root_entity_id: 'source', entities: { source: 3 } } satisfies Command;
// @ts-expect-error Duplication cannot silently detach linked instances.
const detach = { op: 'duplicate_subtree', scene_id: 'scene', root_entity_id: 'source', entities: { source: 'copy' }, detach: true } satisfies Command;
// @ts-expect-error This operation is confined to the source scene.
const crossScene = { op: 'duplicate_subtree', scene_id: 'scene', root_entity_id: 'source', entities: { source: 'copy' }, destination_scene_id: 'other' } satisfies Command;
void [copy, missing, ordinal, detach, crossScene];
