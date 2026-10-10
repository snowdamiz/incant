// Character look-dev course behavior for handoff 0023.
//
// Every entity whose name starts with "Character" walks its lane with the
// engine's read-only api.computeCharacterMotion sweep. The behavior owns gravity
// and the one scripted jump; the engine only resolves collisions. The returned
// displacement is applied as an ordinary Velocity command (translation / dt).
// Compile with: node tools/build_script.mjs character_course.ts character_course.js
import type { CharacterQuery, Entity, Json, ScriptApi } from '../../../sdk/ts/src/index';

const WALK_SPEED = 1.6; // m/s along the lane
const GRAVITY = 9.81; // m/s², behavior-owned
const JUMP_SPEED = 4.0; // m/s initial upward velocity
const JUMP_AT_X = -0.65; // jump lane takes off once its center passes this x
const SLIDE_DIR: [number, number] = [1, -0.6]; // wall lane pushes diagonally into the wall

// One option set for every lane so differences come from geometry, not tuning.
const OPTIONS: NonNullable<CharacterQuery['options']> = {
  offset: 0.02,
  slide: true,
  max_slope_climb_angle: Math.PI / 4,
  min_slope_slide_angle: Math.PI / 4,
  snap_to_ground: 0.3,
  autostep: { max_height: 0.25, min_width: 0.15, include_dynamic_bodies: false },
};

type Lane = { vy: number; jumped: boolean; grounded: boolean; contacts: number };
type State = { tick: number; lanes: { [id: string]: Lane } };

const round = (v: number) => Math.round(v * 1000) / 1000;

function position(entity: Entity): [number, number, number] {
  const transform = entity.components['Transform'] as { translation: [number, number, number] };
  return transform.translation;
}

export default defineBehavior<State & Record<string, Json>>({
  initialState: { tick: 0, lanes: {} },
  update(api: ScriptApi, dt: number, state) {
    state.tick += 1;
    const rows: Json[] = [];
    const characters = api
      .query('RigidBody')
      .filter((e) => e.name.startsWith('Character'))
      .sort((a, b) => (a.id < b.id ? -1 : 1));
    for (const entity of characters) {
      const lane: Lane = state.lanes[entity.id] ?? { vy: 0, jumped: false, grounded: false, contacts: 0 };
      const [x, y, z] = position(entity);
      let dx = WALK_SPEED;
      let dz = 0;
      if (entity.name.includes('wall')) {
        const length = Math.hypot(SLIDE_DIR[0], SLIDE_DIR[1]);
        dx = (WALK_SPEED * SLIDE_DIR[0]) / length;
        dz = (WALK_SPEED * SLIDE_DIR[1]) / length;
      }
      if (entity.name.includes('jump') && !lane.jumped && lane.grounded && x >= JUMP_AT_X) {
        lane.vy = JUMP_SPEED;
        lane.jumped = true;
        api.log(`jump ${entity.name} tick=${state.tick} x=${round(x)} y=${round(y)}`);
      }
      lane.vy -= GRAVITY * dt;
      const movement = api.computeCharacterMotion({
        scene_id: entity.scene_id,
        entity_id: entity.id,
        translation: [dx * dt, lane.vy * dt, dz * dt],
        options: OPTIONS,
      });
      if (movement.grounded && !lane.grounded && state.tick > 1) {
        api.log(`land ${entity.name} tick=${state.tick} x=${round(x)} y=${round(y)} vy=${round(lane.vy)}`);
      }
      if (movement.grounded && lane.vy < 0) lane.vy = 0;
      if (movement.collisions.length !== lane.contacts) {
        api.log(`contacts ${entity.name} tick=${state.tick} ${lane.contacts}->${movement.collisions.length}`);
      }
      lane.grounded = movement.grounded;
      lane.contacts = movement.collisions.length;
      state.lanes[entity.id] = lane;
      api.command({
        op: 'set_component',
        scene_id: entity.scene_id,
        entity_id: entity.id,
        component: 'Velocity',
        value: { linear: movement.translation.map((v) => v / dt) as [number, number, number] },
      });
      rows.push([entity.name, round(x), round(y), round(z), movement.grounded, movement.sliding_down_slope,
        movement.translation.map(round), movement.collisions.length]);
    }
    api.log(JSON.stringify({ t: state.tick, c: rows }), 'debug');
  },
});
