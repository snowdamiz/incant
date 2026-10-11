//! Bulk committed playback copied before numeric loans. Reusable native scratch
//! is bounded by the scene's sprite limit; all published bytes are VM-owned.
use incant_runtime::{FlipbookSnapshot, NativeWorld};
use rquickjs::{Array, Ctx, Object, TypedArray, object::Property};

#[derive(Default)]
pub(crate) struct Scratch {
    rows: Vec<FlipbookSnapshot>,
    frames: Vec<u32>,
    flags: Vec<u32>,
}
impl Scratch {
    pub fn capture(&mut self, world: &mut NativeWorld) -> Result<(), incant_runtime::RuntimeError> {
        world.snapshot_flipbooks(&mut self.rows)?;
        self.frames.clear();
        self.flags.clear();
        for row in &self.rows {
            self.frames.push(u32::from(row.frame));
            self.flags
                .push(u32::from(row.playing) | (u32::from(row.completed) << 1));
        }
        Ok(())
    }
    pub fn expose<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let result = Object::new_proto(ctx.clone(), None)?;
        let ids = Array::new(ctx.clone())?;
        for (index, row) in self.rows.iter().enumerate() {
            // Own data properties cannot invoke Array.prototype setters.
            ids.as_object().prop(
                index as u32,
                Property::from(ulid::Ulid::from_bytes(row.id.0).to_string()).enumerable(),
            )?;
        }
        result.prop("ids", Property::from(ids).enumerable())?;
        // new_copy uses the VM allocator, unlike an externally owned Vec-backed
        // buffer. Retained snapshots count against its memory limit.
        result.prop(
            "frames",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.frames)?).enumerable(),
        )?;
        result.prop(
            "flags",
            Property::from(TypedArray::new_copy(ctx.clone(), &self.flags)?).enumerable(),
        )?;
        Ok(result)
    }
}

#[cfg(all(test, feature = "compiler"))]
mod tests {
    use super::*;
    #[test]
    fn publishing_bulk_ids_and_numeric_copies_never_calls_guest_prototype_setters() {
        let runtime = rquickjs::Runtime::new().unwrap();
        let context = rquickjs::Context::full(&runtime).unwrap();
        let scratch = Scratch {
            rows: vec![FlipbookSnapshot {
                id: incant_runtime::StableId([0; 16]),
                frame: 2,
                playing: false,
                completed: true,
            }],
            frames: vec![2],
            flags: vec![2],
        };
        context.with(|ctx|{
            ctx.eval::<(),_>("Object.defineProperty(Array.prototype,'0',{set(){throw Error('array setter')}});Object.defineProperty(Object.prototype,'frames',{set(){throw Error('object setter')}});").unwrap();
            ctx.globals().set("snapshot",scratch.expose(&ctx).unwrap()).unwrap();
            ctx.eval::<(),_>("if(snapshot.ids[0]!=='00000000000000000000000000'||snapshot.frames[0]!==2||snapshot.flags[0]!==2)throw Error('snapshot');snapshot.frames[0]=99;").unwrap();
        });
        assert_eq!(scratch.frames, [2]);
    }
}
