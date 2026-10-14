//! Immutable row labels only. Numeric views and their backing storage are never
//! cached: every loan still creates fresh, revocable foreign buffers.
use incant_runtime::StableId;
use rquickjs::{Array, Ctx, Function, Persistent, object::Property};

struct Entry {
    rows: Vec<StableId>,
    array: Persistent<Array<'static>>,
}

#[derive(Default)]
pub(crate) struct IdCache(Vec<Entry>);

impl IdCache {
    pub(super) fn truncate(&mut self, count: usize) {
        self.0.truncate(count);
    }

    pub(super) fn get<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        freeze: &Function<'js>,
        index: usize,
        rows: &[StableId],
    ) -> rquickjs::Result<Array<'js>> {
        // Compare complete ordered values, not a table index, hash or generation.
        // Despawn, swap-removal and archetype movement cannot stale the labels.
        if let Some(entry) = self.0.get(index)
            && entry.rows == rows
        {
            return entry.array.clone().restore(ctx);
        }
        let array = Array::new(ctx.clone())?;
        for (index, id) in rows.iter().enumerate() {
            array.as_object().prop(
                index as u32,
                Property::from(ulid::Ulid::from_bytes(id.0).to_string()).enumerable(),
            )?;
        }
        // The SDK's readonly IDs must stay immutable if a guest retains them.
        // Publish a complete frozen array; retained older arrays are untouched.
        freeze.call::<_, ()>((array.clone(),))?;
        let entry = Entry {
            rows: rows.to_vec(),
            array: Persistent::save(ctx, array.clone()),
        };
        if index < self.0.len() {
            self.0[index] = entry;
        } else {
            debug_assert_eq!(index, self.0.len());
            self.0.push(entry);
        }
        Ok(array)
    }
}
