export function probe(): number {
  const values = new Float64Array(3);
  values[0] = 1; values[1] = 2; values[2] = 3;
  let sum = 0;
  for (const value of values) sum += value;
  return sum;
}
