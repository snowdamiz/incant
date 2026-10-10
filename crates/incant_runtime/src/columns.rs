//! Audited numeric loans from Bevy table storage. Keep casts confined here.
use crate::world::{Identity, Local, Motion};
use crate::{NativeWorld, StableId};
use incant_types::{Transform, Velocity};
use std::mem::{align_of, offset_of, size_of};

/// Number of f64 values per Transform: translation xyz, rotation xyzw, scale xyz.
pub const TRANSFORM_STRIDE: usize = 10;
/// Number of f64 values per Velocity: linear xyz.
pub const VELOCITY_STRIDE: usize = 3;

// These assertions compile on every target. repr(C) fixes field order; the
// offsets, sizes and alignment prove there is no interior or trailing padding.
// Transparent ECS wrappers add no bytes. No struct with other fields is cast.
const _: () = {
    assert!(offset_of!(Transform, translation) == 0);
    assert!(offset_of!(Transform, rotation) == 3 * size_of::<f64>());
    assert!(offset_of!(Transform, scale) == 7 * size_of::<f64>());
    assert!(size_of::<Transform>() == TRANSFORM_STRIDE * size_of::<f64>());
    assert!(align_of::<Transform>() == align_of::<f64>());
    assert!(offset_of!(Velocity, linear) == 0);
    assert!(size_of::<Velocity>() == VELOCITY_STRIDE * size_of::<f64>());
    assert!(align_of::<Velocity>() == align_of::<f64>());
    assert!(size_of::<Local>() == size_of::<Transform>());
    assert!(align_of::<Local>() == align_of::<f64>());
    assert!(size_of::<Motion>() == size_of::<Velocity>());
    assert!(align_of::<Motion>() == align_of::<f64>());
    assert!(size_of::<Identity>() == size_of::<StableId>());
    assert!(align_of::<Identity>() == align_of::<StableId>());
};

/// One nonempty ECS table, borrowed for one synchronous visit.
///
/// Row `i` is identified by `ids[i]`. Its Transform occupies
/// `transforms[i * TRANSFORM_STRIDE..(i + 1) * TRANSFORM_STRIDE]`; Velocity uses
/// the corresponding `VELOCITY_STRIDE` range when present. These are direct
/// mutable slices of initialized component storage, with no padding, IDs,
/// pointers, allocation capacity or derived matrices in the numeric slices.
/// Rows/table order are unspecified and may change after structural commands.
pub struct NumericChunk<'a> {
    pub ids: &'a [StableId],
    pub transforms: &'a mut [f64],
    pub velocities: Option<&'a mut [f64]>,
}

/// Restricted access during [`NativeWorld::with_numeric_columns`].
///
/// This handle cannot run systems, inspect derived state or change topology.
/// Repeated visits within a scope use the same table/row order and allocations,
/// allowing a trusted adapter to restore bounded undo copies before returning.
pub struct NumericColumns<'a> {
    owner: &'a mut NativeWorld,
    // One flag per table, not per entity. Tables cannot change during this loan.
    touched: Vec<bool>,
}

impl NativeWorld {
    /// Lend actual numeric component columns exclusively and synchronously.
    ///
    /// For trusted native code: writes are immediate, with no numeric validation
    /// or automatic rollback. Every row of each visited table is conservatively
    /// marked changed, and derived transforms are refreshed once when the scope
    /// exits. This also happens on early return, `Err`, or Rust unwinding, retaining
    /// partial writes; process abort does not run cleanup. The simulation tick is
    /// unchanged. An unused scope does no change tracking or propagation work.
    ///
    /// A script adapter must validate/restore its own bounded undo data *inside*
    /// this scope and revoke every foreign view before Rust reads/restores the
    /// component data, the current chunk visit returns, or this scope exits.
    /// This API supplies no foreign pointers,
    /// detachment, rollback or script host. Native slices cannot outlive a visit.
    ///
    /// ```
    /// # use incant_runtime::{CookedScene, NativeWorld, StableId, TRANSFORM_STRIDE};
    /// # let mut world = NativeWorld::new(CookedScene::new(StableId([0; 16]), 60, vec![]).unwrap());
    /// world.with_numeric_columns(|columns| {
    ///     columns.for_each_chunk(|chunk| {
    ///         for transform in chunk.transforms.as_chunks_mut::<TRANSFORM_STRIDE>().0 {
    ///             transform[0] += 1.0;
    ///         }
    ///     });
    /// });
    /// ```
    ///
    /// Observation and structural mutation require the exclusive loan to end:
    /// ```compile_fail
    /// # use incant_runtime::NativeWorld;
    /// fn overlapping(world: &mut NativeWorld) {
    ///     world.with_numeric_columns(|columns| {
    ///         world.step().unwrap();
    ///         columns.for_each_chunk(|_| {});
    ///     });
    /// }
    /// ```
    /// ```compile_fail
    /// # use incant_runtime::{NativeWorld, StableId};
    /// fn observe(world: &mut NativeWorld, id: StableId) {
    ///     world.with_numeric_columns(|_| world.inspect(id));
    /// }
    /// ```
    /// ```compile_fail
    /// # use incant_runtime::{FrameCommands, NativeWorld};
    /// fn structural(world: &mut NativeWorld, commands: &mut FrameCommands) {
    ///     world.with_numeric_columns(|_| world.apply(commands));
    /// }
    /// ```
    /// ```compile_fail
    /// # use incant_runtime::NativeWorld;
    /// fn escaping(world: &mut NativeWorld) {
    ///     let mut retained = None;
    ///     world.with_numeric_columns(|columns| {
    ///         columns.for_each_chunk(|chunk| retained = Some(chunk.transforms));
    ///     });
    ///     retained.unwrap()[0] = 1.0;
    /// }
    /// ```
    pub fn with_numeric_columns<R>(
        &mut self,
        operation: impl FnOnce(&mut NumericColumns<'_>) -> R,
    ) -> R {
        let touched = vec![false; self.world.storages().tables.len()];
        let mut columns = NumericColumns {
            owner: self,
            touched,
        };
        operation(&mut columns)
    }
}

impl NumericColumns<'_> {
    /// Visit each nonempty Transform table once, including tables without
    /// Velocity. A slice loan ends before the next callback begins.
    pub fn for_each_chunk(&mut self, mut visit: impl FnMut(NumericChunk<'_>)) {
        let result: Result<(), std::convert::Infallible> = self.try_for_each_chunk(|chunk| {
            visit(chunk);
            Ok(())
        });
        match result {
            Ok(()) => (),
            Err(never) => match never {},
        }
    }

    /// Stop at the first callback error. Writes in visited chunks remain;
    /// unvisited numeric columns are untouched. The caller can restore undo data
    /// in another visit before leaving the outer scope. No component arrays are
    /// copied here.
    pub fn try_for_each_chunk<E>(
        &mut self,
        mut visit: impl FnMut(NumericChunk<'_>) -> Result<(), E>,
    ) -> Result<(), E> {
        let world = &mut self.owner.world;
        let (Some(identity_id), Some(local_id)) = (
            world.component_id::<Identity>(),
            world.component_id::<Local>(),
        ) else {
            return Ok(());
        };
        let motion_id = world.component_id::<Motion>();
        // Iterate tables, not archetypes: sparse-set archetypes may share a table.
        for (index, table) in world.storages().tables.iter().enumerate() {
            if table.is_empty() || table.get_column(local_id).is_none() {
                continue;
            }
            self.touched[index] = true;
            // SAFETY: this scope holds the only mutable World borrow. It cannot
            // run systems or structural changes. IDs were obtained for these
            // exact component types from this World; every Local has Identity.
            // Bevy returns only initialized rows, and components occupy distinct
            // allocations. No other view of these cells exists during `visit`.
            // repr(C)/transparent + the compile-time assertions above establish
            // contiguous, aligned f64 values without padding or invalid bit
            // patterns. Identity is borrowed read-only through its transparent
            // wrapper. The callback cannot retain any slice beyond this visit.
            let chunk = unsafe {
                let ids = table
                    .get_data_slice_for::<Identity>(identity_id)
                    .expect("identity");
                let locals = table.get_data_slice_for::<Local>(local_id).expect("local");
                let motions = motion_id.and_then(|id| table.get_data_slice_for::<Motion>(id));
                let ids = std::slice::from_raw_parts(ids[0].get().cast::<StableId>(), ids.len());
                let transforms = std::slice::from_raw_parts_mut(
                    locals[0].get().cast::<f64>(),
                    locals.len() * TRANSFORM_STRIDE,
                );
                let velocities = motions.map(|motions| {
                    std::slice::from_raw_parts_mut(
                        motions[0].get().cast::<f64>(),
                        motions.len() * VELOCITY_STRIDE,
                    )
                });
                NumericChunk {
                    ids,
                    transforms,
                    velocities,
                }
            };
            visit(chunk)?;
        }
        Ok(())
    }
}

impl Drop for NumericColumns<'_> {
    fn drop(&mut self) {
        if !self.touched.iter().any(|touched| *touched) {
            return;
        }
        let world = &mut self.owner.world;
        let tick = world.change_tick();
        let components = [
            world.component_id::<Local>(),
            world.component_id::<Motion>(),
        ];
        for (index, table) in world.storages().tables.iter().enumerate() {
            if !self.touched[index] {
                continue;
            }
            for id in components.into_iter().flatten() {
                if let Some(ticks) = table.get_changed_ticks_slice_for(id) {
                    for changed in ticks {
                        // SAFETY: the exclusive world loan is still held and all
                        // callback slices have ended. These per-component cells
                        // are distinct from each other and from numeric storage.
                        unsafe { *changed.get() = tick };
                    }
                }
            }
        }
        self.owner.refresh_globals();
        self.owner.world.increment_change_tick();
    }
}

#[cfg(test)]
mod tests;
