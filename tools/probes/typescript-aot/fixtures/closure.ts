export function probe(): number {
  let count = 0;
  const next = (): number => ++count;
  return next() + next();
}
