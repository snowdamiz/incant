//! Owned bounded graph snapshots and strict commands on the existing tick queue.
use incant_runtime::{
    AnimationGraphControl, AnimationGraphSnapshot, NativeWorld, StructuralCommand,
};
use rquickjs::{Array, Ctx, Exception, IntoJs, Object, TypedArray, Value, object::Property};

#[derive(Default)]
pub(crate) struct Scratch {
    rows: Vec<AnimationGraphSnapshot>,
    seconds: Vec<f64>,
    paused: Vec<u8>,
    elapsed: Vec<f64>,
    offsets: Vec<u32>,
    values: Vec<f64>,
    source_clips: Vec<i32>,
    source_seconds: Vec<f64>,
    weights: Vec<f64>,
    blending: Vec<u8>,
    weighted: Vec<u8>,
    fade_flags: Vec<u8>,
    fade_seconds: Vec<f64>,
    fade_speeds: Vec<f64>,
    fade_elapsed: Vec<f64>,
    fade_duration: Vec<f64>,
}
impl Scratch {
    pub fn capture(&mut self, world: &mut NativeWorld) -> Result<(), incant_runtime::RuntimeError> {
        world.snapshot_animation_graphs(&mut self.rows)?;
        self.seconds.clear();
        self.paused.clear();
        self.elapsed.clear();
        self.offsets.clear();
        self.values.clear();
        self.source_clips.clear();
        self.source_seconds.clear();
        self.weights.clear();
        self.blending.clear();
        self.weighted.clear();
        self.fade_flags.clear();
        self.fade_seconds.clear();
        self.fade_speeds.clear();
        self.fade_elapsed.clear();
        self.fade_duration.clear();
        self.offsets.push(0);
        for row in &self.rows {
            self.seconds.push(row.seconds);
            self.paused.push(u8::from(row.paused));
            self.elapsed.push(row.elapsed);
            self.values
                .extend_from_slice(&row.parameters[..usize::from(row.parameter_count)]);
            self.offsets.push(self.values.len() as u32);
            self.weighted
                .push(u8::from(row.pose.is_some_and(|p| p.is_weighted())));
            self.fade_flags
                .push(row.fade.map_or(0, |f| 1 | (u8::from(f.playing) << 1)));
            self.fade_seconds.push(row.fade.map_or(0.0, |f| f.seconds));
            self.fade_speeds.push(row.fade.map_or(0.0, |f| f.speed));
            self.fade_elapsed.push(row.fade.map_or(0.0, |f| f.elapsed));
            self.fade_duration
                .push(row.fade.map_or(0.0, |f| f.duration));
            self.blending.push(u8::from(row.blend.is_some()));
            self.source_clips
                .push(row.blend.and_then(|b| b.sample.clip).map_or(-1, i32::from));
            self.source_seconds
                .push(row.blend.map_or(0.0, |b| b.sample.seconds));
            self.weights.push(row.blend.map_or(0.0, |b| b.weight));
        }
        Ok(())
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let result = Object::new_proto(ctx.clone(), None)?;
        let ids = Array::new(ctx.clone())?;
        let states = Array::new(ctx.clone())?;
        let fade_states = Array::new(ctx.clone())?;
        let parameters = Array::new(ctx.clone())?;
        let text = |id: incant_runtime::StableId| ulid::Ulid::from_bytes(id.0).to_string();
        for (index, row) in self.rows.iter().enumerate() {
            fade_states.as_object().prop(
                index as u32,
                Property::from(match row.fade {
                    Some(fade) => text(fade.state).into_js(ctx)?,
                    None => Value::new_null(ctx.clone()),
                })
                .enumerable(),
            )?;
            ids.as_object()
                .prop(index as u32, Property::from(text(row.entity)).enumerable())?;
            states
                .as_object()
                .prop(index as u32, Property::from(text(row.state)).enumerable())?;
            for (slot, id) in row.parameter_ids[..usize::from(row.parameter_count)]
                .iter()
                .enumerate()
            {
                parameters.as_object().prop(
                    self.offsets[index] + slot as u32,
                    Property::from(text(*id)).enumerable(),
                )?;
            }
        }
        result.prop("ids", Property::from(ids).enumerable())?;
        result.prop("states", Property::from(states).enumerable())?;
        result.prop("fadeStates", Property::from(fade_states).enumerable())?;
        result.prop("parameterIds", Property::from(parameters).enumerable())?;
        macro_rules! column {
            ($key:literal,$field:ident) => {
                result.prop(
                    $key,
                    Property::from(TypedArray::new_copy(ctx.clone(), &self.$field)?).enumerable(),
                )?;
            };
        }
        column!("seconds", seconds);
        column!("paused", paused);
        column!("elapsed", elapsed);
        column!("parameterOffsets", offsets);
        column!("parameterValues", values);
        column!("sourceClips", source_clips);
        column!("sourceSeconds", source_seconds);
        column!("weights", weights);
        column!("blending", blending);
        column!("weighted", weighted);
        column!("fadeFlags", fade_flags);
        column!("fadeSeconds", fade_seconds);
        column!("fadeSpeeds", fade_speeds);
        column!("fadeElapsed", fade_elapsed);
        column!("fadeDuration", fade_duration);
        Ok(result)
    }
}
pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    let invalid = || Exception::throw_type(ctx, "unsupported animation graph control arguments");
    let control_id = |value: &Value<'js>| {
        let string = value.as_string().ok_or_else(invalid)?;
        let bytes = string.clone().to_cstring()?;
        if bytes.len() != 26 {
            return Err(invalid());
        }
        drop(bytes);
        crate::commands::id(ctx, value)
    };
    let (entity, control) = match (name, args) {
        ("setAnimationParameter", [entity, parameter, value]) => (
            control_id(entity)?,
            AnimationGraphControl::SetParameter {
                parameter: control_id(parameter)?,
                value: value
                    .as_number()
                    .filter(|n| n.is_finite() && n.abs() <= incant_types::MAX_ANIMATION_GRAPH_VALUE)
                    .ok_or_else(invalid)?,
            },
        ),
        ("selectAnimationState", [entity, state]) => (
            control_id(entity)?,
            AnimationGraphControl::SelectState(control_id(state)?),
        ),
        _ => return Err(invalid()),
    };
    Ok(StructuralCommand::ControlAnimationGraph { entity, control })
}
