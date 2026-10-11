//! Bounded numeric transfers, followed by the existing checked native sync.
//! No foreign pointer or module memory reference enters this file.
use crate::{ModuleError, bounds};
use incant_runtime::{NumericColumns, StableId};

pub struct Chunk {
    pub ids: Vec<StableId>,
    pub transforms: Vec<f64>,
    pub velocities: Option<Vec<f64>>,
    pub dirty_transform: bool,
    pub dirty_velocity: bool,
}
pub fn capture(columns: &mut NumericColumns<'_>) -> Result<Vec<Chunk>, ModuleError> {
    let mut chunks = Vec::new();
    let mut bytes = 0usize;
    columns.try_for_each_chunk(|chunk| {
        bytes = bytes
            .checked_add(
                chunk.ids.len() * 16
                    + chunk.transforms.len() * 8
                    + chunk.velocities.as_ref().map_or(0, |v| v.len() * 8),
            )
            .filter(|size| *size <= bounds::COLUMNS)
            .ok_or(ModuleError::Limit("numeric transfer storage"))?;
        if chunks.len() == bounds::CHUNKS {
            return Err(ModuleError::Limit("numeric chunk count"));
        }
        chunks.push(Chunk {
            ids: chunk.ids.to_vec(),
            transforms: chunk.transforms.to_vec(),
            velocities: chunk.velocities.map(|v| v.to_vec()),
            dirty_transform: false,
            dirty_velocity: false,
        });
        Ok(())
    })?;
    Ok(chunks)
}
struct Saved {
    index: usize,
    ids: Vec<StableId>,
    transforms: Option<Vec<f64>>,
    velocities: Option<Vec<f64>>,
}
pub struct Undo(Vec<Saved>);
impl Undo {
    pub fn restore(&self, columns: &mut NumericColumns<'_>) {
        let mut index = 0;
        let mut saved = self.0.iter().peekable();
        // Check every saved identity/layout before restoring any component.
        columns.for_each_chunk(|chunk| {
            if let Some(old) = saved.peek().filter(|old| old.index == index) {
                assert_eq!(
                    chunk.ids, old.ids,
                    "rejected batch preserves row identities"
                );
                if let Some(values) = &old.transforms {
                    assert_eq!(chunk.transforms.len(), values.len());
                }
                if let Some(values) = &old.velocities {
                    assert_eq!(
                        chunk.velocities.as_ref().map(|v| v.len()),
                        Some(values.len())
                    );
                }
                saved.next();
            }
            index += 1;
        });
        assert!(saved.next().is_none(), "rejected batch preserves tables");
        let mut index = 0;
        let mut saved = self.0.iter().peekable();
        columns.for_each_chunk(|chunk| {
            if let Some(old) = saved.peek().filter(|old| old.index == index) {
                if let Some(values) = &old.transforms {
                    chunk.transforms.copy_from_slice(values);
                }
                if let Some(values) = &old.velocities {
                    chunk.velocities.unwrap().copy_from_slice(values);
                }
                saved.next();
            }
            index += 1;
        });
    }
}
struct Guard<'a, 'w> {
    columns: &'a mut NumericColumns<'w>,
    undo: Option<Undo>,
}
impl Drop for Guard<'_, '_> {
    fn drop(&mut self) {
        if let Some(undo) = &self.undo {
            undo.restore(self.columns);
        }
    }
}
pub fn commit(columns: &mut NumericColumns<'_>, chunks: &[Chunk]) -> Result<Undo, ModuleError> {
    let mut index = 0;
    let mut saved = Vec::new();
    columns.for_each_chunk(|chunk| {
        let new = &chunks[index];
        assert_eq!(chunk.ids, new.ids, "exclusive scope preserves table layout");
        if new.dirty_transform || new.dirty_velocity {
            saved.push(Saved {
                index,
                ids: chunk.ids.to_vec(),
                transforms: new.dirty_transform.then(|| chunk.transforms.to_vec()),
                velocities: new
                    .dirty_velocity
                    .then(|| chunk.velocities.as_ref().unwrap().to_vec()),
            });
        }
        index += 1;
    });
    assert_eq!(index, chunks.len());
    let mut guard = Guard {
        columns,
        undo: Some(Undo(saved)),
    };
    let mut index = 0;
    guard.columns.for_each_chunk(|chunk| {
        let new = &chunks[index];
        if new.dirty_transform {
            chunk.transforms.copy_from_slice(&new.transforms);
        }
        if new.dirty_velocity {
            chunk
                .velocities
                .unwrap()
                .copy_from_slice(new.velocities.as_ref().unwrap());
        }
        index += 1;
    });
    guard.columns.validate()?;
    Ok(guard.undo.take().unwrap())
}
