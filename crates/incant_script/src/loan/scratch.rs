//! Reusable native copies only: no world pointer, JavaScript root or view may
//! enter this cache. A lease returns storage after its owner has detached views
//! and either committed or restored numeric writes.
use incant_runtime::StableId;

pub(super) struct SavedChunk {
    pub ids: Box<[StableId]>,
    pub transforms: Box<[f64]>,
    pub velocities: Option<Box<[f64]>>,
}

#[derive(Default)]
pub(crate) struct Scratch(pub(super) Vec<SavedChunk>);

pub(super) struct Lease<'cache> {
    cache: &'cache mut Scratch,
    pub chunks: Vec<SavedChunk>,
    captured: usize,
}

impl Scratch {
    pub(super) fn lease(&mut self) -> Lease<'_> {
        Lease {
            chunks: std::mem::take(&mut self.0),
            cache: self,
            captured: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn allocations(&self) -> Vec<(usize, usize, Option<usize>)> {
        self.0
            .iter()
            .map(|chunk| {
                (
                    chunk.ids.as_ptr().addr(),
                    chunk.transforms.as_ptr().addr(),
                    chunk.velocities.as_ref().map(|v| v.as_ptr().addr()),
                )
            })
            .collect()
    }
}

impl Lease<'_> {
    pub(super) fn capture(
        &mut self,
        ids: &[StableId],
        transforms: &[f64],
        velocities: Option<&[f64]>,
    ) {
        let index = self.captured;
        let matching = self.chunks.get(index).is_some_and(|old| {
            old.ids.len() == ids.len()
                && old.transforms.len() == transforms.len()
                && old.velocities.as_ref().map(|v| v.len()) == velocities.map(<[f64]>::len)
        });
        if matching {
            let old = &mut self.chunks[index];
            // Every row identity and value is captured again, including an
            // unchanged layout. Retained values never become an undo baseline.
            old.ids.copy_from_slice(ids);
            old.transforms.copy_from_slice(transforms);
            if let (Some(old), Some(values)) = (&mut old.velocities, velocities) {
                old.copy_from_slice(values);
            }
        } else {
            // Evict the unused tail BEFORE allocating a changed shape. Matching
            // prefixes retain exactly their active sizes; the caller admits the
            // whole next chunk within the existing numeric-copy byte limit.
            self.chunks.truncate(index);
            self.chunks.push(SavedChunk {
                ids: ids.into(),
                transforms: transforms.into(),
                velocities: velocities.map(Into::into),
            });
        }
        self.captured += 1;
    }
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        // Constructor refusal can leave a complete captured prefix. Discard
        // uncaptured old rows as well as container capacity after topology
        // shrink, so neither IDs nor cache tails accumulate across churn.
        self.chunks.truncate(self.captured);
        self.chunks.shrink_to_fit();
        self.cache.0 = std::mem::take(&mut self.chunks);
    }
}
