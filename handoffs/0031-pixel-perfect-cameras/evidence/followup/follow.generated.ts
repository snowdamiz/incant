import type { ScriptApi, Transform } from "/Users/andreyyurlov/.codex/worktrees/physics-character/incant/.worktrees/0031-pixel-perfect-cameras/sdk/ts/src/index";
type Config = { rig: string; a: string; b: string; z: number;
  m: [number, number, number, number]; q: [number, number, number, number] };
const SCENE_ID = "01M4KC1E0941GEBEW7M5CKB7KE";
const PPU = 32;
const TICKS_PER_LEG = 8;
const LEGS: [number, number][] = [[1, 0], [-1, 0], [-1, 0], [1, 0], [0, 1], [0, -1], [0, -1], [0, 1], [1, 1], [-1, -1], [-1, -1], [1, 1], [1, -1], [-1, 1], [-1, 1], [1, -1]];
const FOLLOW_A: [number, number] = [-40, -30];
const FOLLOW_B: [number, number] = [37, 17];
const CONFIGS: Config[] = [{"rig": "01JA31FF000000000000000002", "a": "01JA31FF000000000000000004", "b": "01JA31FF000000000000000005", "z": -0.0, "m": [1.0, -0.0, 0.0, 1.0], "q": [0.0, 0.0, 0.0, 1.0]}, {"rig": "01JA31FF000000000000000007", "a": "01JA31FF000000000000000009", "b": "01JA31FF00000000000000000A", "z": -100.0, "m": [2.220446049250313e-16, -1.0, 1.0, 2.220446049250313e-16], "q": [0.0, 0.0, 0.7071067811865475, 0.7071067811865476]}, {"rig": "01JA31FF00000000000000000C", "a": "01JA31FF00000000000000000E", "b": "01JA31FF00000000000000000F", "z": -200.0, "m": [0.9323273456060344, -0.361615431964962, 0.361615431964962, 0.9323273456060344], "q": [0.0, 0.0, 0.18394653352804122, 0.9829362506302315]}, {"rig": "01JA31FF00000000000000000H", "a": "01JA31FF00000000000000000K", "b": "01JA31FF00000000000000000M", "z": -300.0, "m": [1.0, -0.0, 0.0, 1.0], "q": [0.0, 0.0, 0.0, 1.0]}, {"rig": "01JA31FF00000000000000000P", "a": "01JA31FF00000000000000000R", "b": "01JA31FF00000000000000000S", "z": -400.0, "m": [2.220446049250313e-16, -1.0, 1.0, 2.220446049250313e-16], "q": [0.0, 0.0, 0.7071067811865475, 0.7071067811865476]}, {"rig": "01JA31FF00000000000000000V", "a": "01JA31FF00000000000000000X", "b": "01JA31FF00000000000000000Y", "z": -500.0, "m": [0.9323273456060344, -0.361615431964962, 0.361615431964962, 0.9323273456060344], "q": [0.0, 0.0, 0.18394653352804122, 0.9829362506302315]}, {"rig": "01JA31FF000000000000000010", "a": "01JA31FF000000000000000012", "b": "01JA31FF000000000000000013", "z": -600.0, "m": [1.0, -0.0, 0.0, 1.0], "q": [0.0, 0.0, 0.0, 1.0]}, {"rig": "01JA31FF000000000000000015", "a": "01JA31FF000000000000000017", "b": "01JA31FF000000000000000018", "z": -700.0, "m": [2.220446049250313e-16, -1.0, 1.0, 2.220446049250313e-16], "q": [0.0, 0.0, 0.7071067811865475, 0.7071067811865476]}, {"rig": "01JA31FF00000000000000001A", "a": "01JA31FF00000000000000001C", "b": "01JA31FF00000000000000001D", "z": -800.0, "m": [0.9323273456060344, -0.361615431964962, 0.361615431964962, 0.9323273456060344], "q": [0.0, 0.0, 0.18394653352804122, 0.9829362506302315]}, {"rig": "01JA31FF00000000000000001F", "a": "01JA31FF00000000000000001H", "b": "01JA31FF00000000000000001J", "z": -900.0, "m": [1.0, -0.0, 0.0, 1.0], "q": [0.0, 0.0, 0.0, 1.0]}, {"rig": "01JA31FF00000000000000001M", "a": "01JA31FF00000000000000001P", "b": "01JA31FF00000000000000001Q", "z": -1000.0, "m": [2.220446049250313e-16, -1.0, 1.0, 2.220446049250313e-16], "q": [0.0, 0.0, 0.7071067811865475, 0.7071067811865476]}, {"rig": "01JA31FF00000000000000001S", "a": "01JA31FF00000000000000001V", "b": "01JA31FF00000000000000001W", "z": -1100.0, "m": [0.9323273456060344, -0.361615431964962, 0.361615431964962, 0.9323273456060344], "q": [0.0, 0.0, 0.18394653352804122, 0.9829362506302315]}];

function eighths(k: number): [number, number] {
  let r = 0;
  let u = 0;
  for (let i = 0; i < k; i++) {
    const [dr, du] = LEGS[Math.floor(i / TICKS_PER_LEG)]!;
    r += dr;
    u += du;
  }
  return [r, u];
}

function place(c: Config, right: number, up: number, z: number): Transform {
  const x = right / PPU;
  const y = up / PPU;
  return { translation: [c.m[0] * x + c.m[1] * y, c.m[2] * x + c.m[3] * y, z], rotation: c.q, scale: [1, 1, 1] };
}

export default defineBehavior<{ tick: number }>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt, state) {
    const k = state.tick + 1; // commanded now, visible from the next frame
    state.tick++;
    if (k > LEGS.length * TICKS_PER_LEG) return;
    const [er, eu] = eighths(k);
    const right = er / 8;
    const up = eu / 8;
    for (const c of CONFIGS) {
      const set = (entity_id: string, value: Transform) =>
        api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id, component: 'Transform', value });
      set(c.rig, place(c, right, up, c.z));
      set(c.a, place(c, right + FOLLOW_A[0], up + FOLLOW_A[1], c.z + 0.02));
      set(c.b, place(c, right + FOLLOW_B[0], up + FOLLOW_B[1], c.z + 0.02));
    }
  },
});
