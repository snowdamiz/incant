//! Owned root-motion columns; exact signed cycles never pass through JS Number.
use incant_runtime::{NativeWorld, RootMotionControl, RootMotionSnapshot, StructuralCommand};
use incant_types::RootMotionApplication;
use rquickjs::{Array, Ctx, Exception, IntoJs, Object, TypedArray, Value, object::Property};

#[derive(Default)]
pub(crate) struct Scratch {
    rows: Vec<RootMotionSnapshot>,
    fingerprints: Vec<u8>,
    roots: Vec<u16>,
    policies: Vec<u8>,
    flags: Vec<u8>,
    ticks: Vec<u64>,
    clips: Vec<i32>,
    cycles: Vec<i64>,
    seconds: Vec<f64>,
    outgoing_clips: Vec<i32>,
    outgoing_cycles: Vec<i64>,
    outgoing_seconds: Vec<f64>,
    requested: Vec<f64>,
    applied: Vec<f64>,
    accumulated_requested: Vec<f64>,
    accumulated_applied: Vec<f64>,
}
impl Scratch {
    pub fn capture(&mut self, world: &NativeWorld) -> Result<(), incant_runtime::RuntimeError> {
        world.snapshot_root_motion(&mut self.rows)?;
        self.columns();
        Ok(())
    }
    fn columns(&mut self) {
        self.fingerprints.clear();
        self.roots.clear();
        self.policies.clear();
        self.flags.clear();
        self.ticks.clear();
        self.clips.clear();
        self.cycles.clear();
        self.seconds.clear();
        self.outgoing_clips.clear();
        self.outgoing_cycles.clear();
        self.outgoing_seconds.clear();
        self.requested.clear();
        self.applied.clear();
        self.accumulated_requested.clear();
        self.accumulated_applied.clear();
        for row in &self.rows {
            self.fingerprints.extend_from_slice(&row.fingerprint);
            self.roots.push(row.root);
            self.policies.push(match row.application {
                RootMotionApplication::ExtractOnly => 0,
                RootMotionApplication::Transform => 1,
                RootMotionApplication::Kinematic3d => 2,
                RootMotionApplication::Character3d => 3,
            });
            self.flags
                .push(u8::from(row.applying) | (u8::from(row.outgoing.is_some()) << 1));
            self.ticks.push(row.tick);
            self.clips.push(row.current.clip.map_or(-1, i32::from));
            self.cycles.push(row.current.cycle);
            self.seconds.push(row.current.seconds);
            self.outgoing_clips
                .push(row.outgoing.and_then(|c| c.clip).map_or(-1, i32::from));
            self.outgoing_cycles
                .push(row.outgoing.map_or(0, |c| c.cycle));
            self.outgoing_seconds
                .push(row.outgoing.map_or(0., |c| c.seconds));
            for (column, delta) in [
                (&mut self.requested, row.requested),
                (&mut self.applied, row.applied),
                (&mut self.accumulated_requested, row.accumulated_requested),
                (&mut self.accumulated_applied, row.accumulated_applied),
            ] {
                column.extend_from_slice(&delta.translation);
                column.extend_from_slice(&delta.rotation);
            }
        }
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let result = Object::new_proto(ctx.clone(), None)?;
        let ids = Array::new(ctx.clone())?;
        let states = Array::new(ctx.clone())?;
        let outgoing = Array::new(ctx.clone())?;
        let text = |id: incant_runtime::StableId| ulid::Ulid::from_bytes(id.0).to_string();
        for (n, row) in self.rows.iter().enumerate() {
            ids.as_object()
                .prop(n as u32, Property::from(text(row.entity)).enumerable())?;
            for (array, state) in [
                (&states, row.current.state),
                (&outgoing, row.outgoing.and_then(|c| c.state)),
            ] {
                array.as_object().prop(
                    n as u32,
                    Property::from(match state {
                        Some(id) => text(id).into_js(ctx)?,
                        None => Value::new_null(ctx.clone()),
                    })
                    .enumerable(),
                )?;
            }
        }
        result.prop("ids", Property::from(ids).enumerable())?;
        result.prop("states", Property::from(states).enumerable())?;
        result.prop("outgoingStates", Property::from(outgoing).enumerable())?;
        macro_rules! column {
            ($key:literal,$field:ident) => {
                result.prop(
                    $key,
                    Property::from(TypedArray::new_copy(ctx.clone(), &self.$field)?).enumerable(),
                )?;
            };
        }
        column!("fingerprints", fingerprints);
        column!("roots", roots);
        column!("policies", policies);
        column!("flags", flags);
        column!("ticks", ticks);
        column!("clips", clips);
        column!("cycles", cycles);
        column!("seconds", seconds);
        column!("outgoingClips", outgoing_clips);
        column!("outgoingCycles", outgoing_cycles);
        column!("outgoingSeconds", outgoing_seconds);
        column!("requested", requested);
        column!("applied", applied);
        column!("accumulatedRequested", accumulated_requested);
        column!("accumulatedApplied", accumulated_applied);
        Ok(result)
    }
}

pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    let invalid = || Exception::throw_type(ctx, "unsupported root-motion control arguments");
    let [entity, action, rest @ ..] = args else {
        return Err(invalid());
    };
    let entity_text = entity.as_string().ok_or_else(invalid)?;
    let bytes = entity_text.clone().to_cstring()?;
    if bytes.len() != 26 {
        return Err(invalid());
    }
    drop(bytes);
    let entity = crate::commands::id(ctx, entity)?;
    let action = action.as_string().ok_or_else(invalid)?;
    let bytes = action.clone().to_cstring()?;
    if bytes.len() > 9 {
        return Err(invalid());
    }
    drop(bytes);
    let action = action.to_string().map_err(|_| invalid())?;
    let control = match (action.as_str(), rest) {
        ("applying", [value]) => {
            RootMotionControl::SetApplying(value.as_bool().ok_or_else(invalid)?)
        }
        ("reset", []) => RootMotionControl::ResetAccumulated,
        _ => return Err(invalid()),
    };
    Ok(StructuralCommand::ControlRootMotion { entity, control })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cycles_and_ticks_are_exact_owned_bigint_columns() {
        let runtime = rquickjs::Runtime::new().unwrap();
        let context = rquickjs::Context::full(&runtime).unwrap();
        let scratch = Scratch {
            cycles: vec![i64::MIN, i64::MAX, 9_007_199_254_740_993],
            outgoing_cycles: vec![-9_007_199_254_740_993],
            ticks: vec![u64::MAX],
            ..Default::default()
        };
        context.with(|ctx| {
            ctx.eval::<(),_>("for(const key of ['cycles','ticks','outgoingCycles'])Object.defineProperty(Object.prototype,key,{set(){throw Error('prototype')}})").unwrap();
            let first=scratch.expose(&ctx).unwrap();ctx.globals().set("first",first).unwrap();
            ctx.eval::<(),_>("if(!(first.cycles instanceof BigInt64Array)||!(first.ticks instanceof BigUint64Array)||first.cycles[0]!==-9223372036854775808n||first.cycles[1]!==9223372036854775807n||first.cycles[2]!==9007199254740993n||first.outgoingCycles[0]!==-9007199254740993n||first.ticks[0]!==18446744073709551615n)throw Error('precision');first.cycles[0]=0n").unwrap();
            ctx.globals().set("second",scratch.expose(&ctx).unwrap()).unwrap();
            ctx.eval::<(),_>("if(second.cycles[0]!==-9223372036854775808n||first.cycles[0]!==0n)throw Error('ownership')").unwrap();
        });
    }
}
