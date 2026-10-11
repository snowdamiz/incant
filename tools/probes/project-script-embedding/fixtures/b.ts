import type { NativeBehavior } from '@incant/runtime/native';
import { next, update } from './shared';
const initialized = next();
export default { update(context) { update(context, 2, 1, initialized); return undefined; } } satisfies NativeBehavior;
