import type { NativeBehavior } from '@incant/runtime/native';
import { next, update } from './shared';
const initialized = next();
export default { update(context) { update(context, 1, 2, initialized); return undefined; } } satisfies NativeBehavior;
