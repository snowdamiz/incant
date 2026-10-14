// Portable arithmetic control, NOT an Incant host adapter or the complete Core workload.
const rows = 9999;
const values = new Float64Array(rows * 3);
export function update(tick: number): number {
  const phase = (tick % 64 - 32) / 64;
  let sum = 0.0;
  for (let row = 0; row < rows; row++) {
    const offset = row * 3;
    values[offset] = row % 97 - 48 + phase;
    values[offset + 1] = Math.floor(row / 97) % 97 - 48;
    values[offset + 2] = row % 13 - 6 - phase;
    sum += values[offset] + values[offset + 2];
  }
  return sum;
}
export function first(): number { return values[0]; }
export function last(): number { return values[(rows - 1) * 3 + 2]; }
