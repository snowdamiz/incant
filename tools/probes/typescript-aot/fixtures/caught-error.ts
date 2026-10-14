export function probe(): number {
  let value = 0;
  try { throw new Error('bounded test refusal'); } catch (_error) { value = 7; }
  return value;
}
