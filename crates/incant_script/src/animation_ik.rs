//! Owned bounded IK columns and a primitive command on the common tick queue.
use incant_runtime::{IkControl, IkSnapshot, NativeWorld, StructuralCommand};
use rquickjs::{Array, Ctx, Exception, Object, TypedArray, Value, object::Property};

#[derive(Default)]
pub(crate) struct Scratch {
    rows: Vec<IkSnapshot>,
    chains: Vec<u16>,
    targets: Vec<f64>,
    poles: Vec<f64>,
    weights: Vec<f64>,
}
impl Scratch {
    pub fn capture(&mut self, world: &NativeWorld) -> Result<(), incant_runtime::RuntimeError> {
        world.snapshot_ik(&mut self.rows)?;
        self.chains.clear();
        self.targets.clear();
        self.poles.clear();
        self.weights.clear();
        for row in &self.rows {
            self.chains.extend(row.sample.chain);
            self.targets.extend(row.sample.target);
            self.poles.extend(row.sample.pole);
            self.weights.push(row.sample.weight);
        }
        Ok(())
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let result = Object::new_proto(ctx.clone(), None)?;
        let entities = Array::new(ctx.clone())?;
        let constraints = Array::new(ctx.clone())?;
        for (i, row) in self.rows.iter().enumerate() {
            for (array, id) in [(&entities, row.entity), (&constraints, row.sample.id)] {
                array.as_object().prop(
                    i as u32,
                    Property::from(ulid::Ulid::from_bytes(id.0).to_string()).enumerable(),
                )?;
            }
        }
        result.prop("ids", Property::from(entities).enumerable())?;
        result.prop("constraintIds", Property::from(constraints).enumerable())?;
        macro_rules! column {
            ($name:literal, $field:ident) => {
                result.prop(
                    $name,
                    Property::from(TypedArray::new_copy(ctx.clone(), &self.$field)?).enumerable(),
                )?;
            };
        }
        column!("chains", chains);
        column!("targets", targets);
        column!("poles", poles);
        column!("weights", weights);
        Ok(result)
    }
}
pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    let invalid = || Exception::throw_type(ctx, "unsupported IK control arguments");
    // Primitive lanes avoid user getters while holding command/native state.
    let [entity, constraint, tx, ty, tz, px, py, pz, weight] = args else {
        return Err(invalid());
    };
    let id = |value: &Value<'js>| {
        let string = value.as_string().ok_or_else(invalid)?;
        let bytes = string.clone().to_cstring()?;
        if bytes.len() != 26 {
            return Err(invalid());
        }
        drop(bytes);
        crate::commands::id(ctx, value)
    };
    let coordinate = |v: &Value<'js>| {
        v.as_number()
            .filter(|n| n.is_finite() && n.abs() <= incant_types::MAX_IK_COORDINATE)
            .ok_or_else(invalid)
    };
    Ok(StructuralCommand::ControlIk {
        entity: id(entity)?,
        control: IkControl {
            constraint: id(constraint)?,
            target: [coordinate(tx)?, coordinate(ty)?, coordinate(tz)?],
            pole: [coordinate(px)?, coordinate(py)?, coordinate(pz)?],
            weight: weight
                .as_number()
                .filter(|n| n.is_finite() && (0.0..=1.0).contains(n))
                .ok_or_else(invalid)?,
        },
    })
}
