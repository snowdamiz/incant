import type { Command } from '../src/index';
import type { ProjectSchema } from '../src/generated';

// Consumers retain legacy projects and can author either supported solver family.
const legacy = { tick_rate: 60 } satisfies ProjectSchema.ProjectSettings;
const fast = { tick_rate: 60, physics_mode: 'fast' } satisfies ProjectSchema.ProjectSettings;
const deterministic = { tick_rate: 60, physics_mode: 'deterministic' } satisfies ProjectSchema.ProjectSettings;
const selectFast = { op: 'set_physics_mode', mode: 'fast' } satisfies Command;
const selectDeterministic = { op: 'set_physics_mode', mode: 'deterministic' } satisfies Command;

// @ts-expect-error SIMD width is a target optimization, not an authored mode.
const targetVariant = { tick_rate: 60, physics_mode: 'fast-simd8' } satisfies ProjectSchema.ProjectSettings;
// @ts-expect-error A mode change must name its new solver family.
const missingMode = { op: 'set_physics_mode' } satisfies Command;
// @ts-expect-error Unrecognized authored modes must not enter the command contract.
const unknownMode = { op: 'set_physics_mode', mode: 'automatic' } satisfies Command;

void [legacy, fast, deterministic, selectFast, selectDeterministic, targetVariant, missingMode, unknownMode];
