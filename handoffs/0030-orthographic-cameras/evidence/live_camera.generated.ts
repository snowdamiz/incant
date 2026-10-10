import type { Camera, ScriptApi, Transform } from "<repo>/sdk/ts/src/index";
type CameraProjection_ = NonNullable<Camera['projection']>;
const SCENE_ID = "01M4K6VFS5EW2KS4936EWD53JZ";
const LIVE_ID = "01JA30AA00000000000000000H";
const STEPS: { tick: number; label: string; projection: CameraProjection_ | null; transform: Transform }[] = [{"tick": 0, "label": "t0-d16-legacy", "projection": null, "transform": {"translation": [7.947708238, 8.8, 8.350503679], "rotation": [-0.246840110491, 0.290459497856, 0.077828387884, 0.921219833697], "scale": [1, 1, 1]}}, {"tick": 10, "label": "t10-d16-orthographic-v9", "projection": {"kind": "orthographic", "vertical_size": 9}, "transform": {"translation": [7.947708238, 8.8, 8.350503679], "rotation": [-0.246840110491, 0.290459497856, 0.077828387884, 0.921219833697], "scale": [1, 1, 1]}}, {"tick": 20, "label": "t20-d24-orthographic-v9", "projection": {"kind": "orthographic", "vertical_size": 9}, "transform": {"translation": [11.921562357, 12.8, 14.025755518], "rotation": [-0.246840110491, 0.290459497856, 0.077828387884, 0.921219833697], "scale": [1, 1, 1]}}, {"tick": 30, "label": "t30-d24-orthographic-v6", "projection": {"kind": "orthographic", "vertical_size": 6}, "transform": {"translation": [11.921562357, 12.8, 14.025755518], "rotation": [-0.246840110491, 0.290459497856, 0.077828387884, 0.921219833697], "scale": [1, 1, 1]}}, {"tick": 40, "label": "t40-d24-perspective", "projection": {"kind": "perspective"}, "transform": {"translation": [11.921562357, 12.8, 14.025755518], "rotation": [-0.246840110491, 0.290459497856, 0.077828387884, 0.921219833697], "scale": [1, 1, 1]}}];

export default defineBehavior<{ tick: number }>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt, state) {
    const step = STEPS.find((s) => s.tick === state.tick);
    state.tick++;
    if (!step) return;
    const camera = api.query('Camera').find((e) => e.id === LIVE_ID)!.components.Camera as Camera;
    const value: Camera = { fov_degrees: camera.fov_degrees, near: camera.near, far: camera.far };
    if (step.projection) value.projection = step.projection;
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Camera', value });
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Transform', value: step.transform });
    api.log(JSON.stringify({ t: state.tick - 1, step: step.label }));
  },
});
