//! Graph-v2 adds captured outgoing clocks. Parameters remain current committed
//! values shared by both graph states, as in the native evaluator.
use crate::{AnimationGraphKey, AnimationGraphSnapshot, EntityId, Error, Result};
use incant_wasm_abi as abi;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationGraphFade {
    pub entity: EntityId,
    pub state: AnimationGraphKey,
    pub seconds: f64,
    pub speed: f64,
    pub playing: bool,
    pub elapsed: f64,
    pub duration: f64,
}
#[derive(Clone, Copy)]
pub struct AnimationGraphSnapshotV2<'a> {
    graphs: AnimationGraphSnapshot<'a>,
    fades: &'a [u8],
}
impl<'a> AnimationGraphSnapshotV2<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let h = abi::AnimationGraphHeaderV2::decode(bytes).ok_or(Error::Malformed)?;
        if &h.magic != abi::ANIMATION_GRAPH_V2_MAGIC
            || h.reserved != 0
            || h.stride as usize != abi::AnimationGraphFade::SIZE
            || h.count as usize > abi::ANIMATION_GRAPH_MAX_ROWS
            || h.graph_bytes as usize > abi::ANIMATION_GRAPH_MAX_BYTES
        {
            return Err(Error::Malformed);
        }
        let end = abi::AnimationGraphHeaderV2::SIZE + h.graph_bytes as usize;
        if bytes.len() != end + h.count as usize * abi::AnimationGraphFade::SIZE {
            return Err(Error::Malformed);
        }
        let graphs = AnimationGraphSnapshot::parse(&bytes[abi::AnimationGraphHeaderV2::SIZE..end])?;
        let fades = &bytes[end..];
        let mut owners = graphs.iter().peekable();
        let mut previous = None;
        for bytes in fades.as_chunks::<{ abi::AnimationGraphFade::SIZE }>().0 {
            let r = abi::AnimationGraphFade::decode(bytes).ok_or(Error::Malformed)?;
            if !r.is_finite()
                || r.reserved != 0
                || r.flags & !1 != 0
                || r.seconds < 0.
                || r.speed.abs() > abi::ANIMATION_MAX_SPEED as f64
                || r.duration <= 0.
                || r.duration > abi::ANIMATION_MAX_TRANSITION_SECONDS as f64
                || r.elapsed < 0.
                || r.elapsed >= r.duration
                || previous.is_some_and(|id| id >= r.entity)
            {
                return Err(Error::Malformed);
            }
            while owners.peek().is_some_and(|g| g.entity.0 < r.entity) {
                owners.next();
            }
            let g = owners
                .peek()
                .filter(|g| g.entity.0 == r.entity)
                .ok_or(Error::Malformed)?;
            if g.state.0 == r.state || g.elapsed != r.elapsed {
                return Err(Error::Malformed);
            }
            previous = Some(r.entity);
        }
        Ok(Self { graphs, fades })
    }
    pub fn graphs(&self) -> AnimationGraphSnapshot<'a> {
        self.graphs
    }
    pub fn fades(&self) -> impl ExactSizeIterator<Item = AnimationGraphFade> + 'a {
        self.fades
            .as_chunks::<{ abi::AnimationGraphFade::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let r = abi::AnimationGraphFade::decode(bytes).expect("validated graph fade");
                AnimationGraphFade {
                    entity: EntityId(r.entity),
                    state: AnimationGraphKey(r.state),
                    seconds: r.seconds,
                    speed: r.speed,
                    playing: r.flags & 1 != 0,
                    elapsed: r.elapsed,
                    duration: r.duration,
                }
            })
    }
}
