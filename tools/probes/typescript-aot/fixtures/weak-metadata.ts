// NativeChunk.ids are immutable metadata; numeric views are never cached.
const cache = new WeakMap<readonly string[], readonly number[]>();
export function probe(): number {
  const ids = Object.freeze(['01CSEN00000000000000000001']);
  cache.set(ids, Object.freeze([9]));
  return cache.get(ids)![0];
}
