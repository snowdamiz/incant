//! Copied state-machine parameters and transactional controls. Graph keys are
//! stable authored IDs, never native indices or handles.
use crate::{EntityId, Error, Result};
use incant_wasm_abi as abi;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnimationGraphKey(pub [u8; abi::ID_BYTES]);
impl AnimationGraphKey {
    pub const fn from_u128(value: u128) -> Self {
        Self(value.to_be_bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationGraphControl {
    SetParameter {
        parameter: AnimationGraphKey,
        value: f64,
    },
    SelectState(AnimationGraphKey),
}
impl AnimationGraphControl {
    pub(crate) fn encode(self) -> Result<[u8; abi::AnimationGraphCommand::SIZE]> {
        let (kind, id, value) = match self {
            Self::SetParameter { parameter, value } => {
                if !value.is_finite() || value.abs() > abi::ANIMATION_GRAPH_MAX_VALUE as f64 {
                    return Err(Error::Malformed);
                }
                (0, parameter.0, value)
            }
            Self::SelectState(state) => (1, state.0, 0.0),
        };
        Ok(abi::AnimationGraphCommand {
            kind,
            reserved: 0,
            id,
            value,
        }
        .encode())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationGraphParameter {
    pub id: AnimationGraphKey,
    pub value: f64,
}

#[derive(Clone, Copy)]
pub struct AnimationGraphPlayback<'a> {
    pub entity: EntityId,
    pub state: AnimationGraphKey,
    /// Explicit master pause, distinct from natural clip completion.
    pub paused: bool,
    pub seconds: f64,
    pub elapsed: f64,
    parameters: &'a [u8],
}
impl<'a> AnimationGraphPlayback<'a> {
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = AnimationGraphParameter> + 'a {
        self.parameters
            .as_chunks::<{ abi::AnimationGraphParameter::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let row =
                    abi::AnimationGraphParameter::decode(bytes).expect("validated graph parameter");
                AnimationGraphParameter {
                    id: AnimationGraphKey(row.id),
                    value: row.value,
                }
            })
    }
}

#[derive(Clone, Copy)]
pub struct AnimationGraphSnapshot<'a> {
    rows: &'a [u8],
    parameters: &'a [u8],
}
impl<'a> AnimationGraphSnapshot<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = abi::AnimationGraphHeader::decode(bytes).ok_or(Error::Malformed)?;
        if &header.magic != abi::ANIMATION_GRAPH_MAGIC
            || header.stride as usize != abi::AnimationGraphRow::SIZE
            || header.parameter_stride as usize != abi::AnimationGraphParameter::SIZE
            || header.count as usize > abi::ANIMATION_GRAPH_MAX_ROWS
            || header.parameter_count as usize > abi::ANIMATION_GRAPH_MAX_PARAMETER_ROWS
        {
            return Err(Error::Malformed);
        }
        let row_bytes = header.count as usize * abi::AnimationGraphRow::SIZE;
        let parameter_bytes = header.parameter_count as usize * abi::AnimationGraphParameter::SIZE;
        if bytes.len() != abi::AnimationGraphHeader::SIZE + row_bytes + parameter_bytes {
            return Err(Error::Malformed);
        }
        let (rows, parameters) = bytes[abi::AnimationGraphHeader::SIZE..].split_at(row_bytes);
        let mut previous = None;
        let mut next_parameter = 0usize;
        for bytes in rows
            .as_chunks::<{ abi::AnimationGraphRow::SIZE }>()
            .0
            .iter()
        {
            let row = abi::AnimationGraphRow::decode(bytes).ok_or(Error::Malformed)?;
            if !row.is_finite()
                || row.seconds < 0.0
                || !(0.0..=abi::ANIMATION_GRAPH_MAX_SECONDS as f64).contains(&row.elapsed)
                || row.flags & !1 != 0
                || row.reserved != 0
                || previous.is_some_and(|id| id >= row.id)
                || row.parameter_offset as usize != next_parameter
                || row.parameter_count as usize > abi::ANIMATION_GRAPH_MAX_PARAMETERS
            {
                return Err(Error::Malformed);
            }
            previous = Some(row.id);
            let end = next_parameter + row.parameter_count as usize;
            if end > header.parameter_count as usize {
                return Err(Error::Malformed);
            }
            let mut previous_parameter = None;
            for bytes in parameters[next_parameter * abi::AnimationGraphParameter::SIZE
                ..end * abi::AnimationGraphParameter::SIZE]
                .as_chunks::<{ abi::AnimationGraphParameter::SIZE }>()
                .0
                .iter()
            {
                let parameter =
                    abi::AnimationGraphParameter::decode(bytes).ok_or(Error::Malformed)?;
                if !parameter.value.is_finite()
                    || parameter.value.abs() > abi::ANIMATION_GRAPH_MAX_VALUE as f64
                    || previous_parameter.is_some_and(|id| id >= parameter.id)
                {
                    return Err(Error::Malformed);
                }
                previous_parameter = Some(parameter.id);
            }
            next_parameter = end;
        }
        if next_parameter != header.parameter_count as usize {
            return Err(Error::Malformed);
        }
        Ok(Self { rows, parameters })
    }
    pub fn len(&self) -> usize {
        self.rows.len() / abi::AnimationGraphRow::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = AnimationGraphPlayback<'a>> + 'a {
        let parameters = self.parameters;
        self.rows
            .as_chunks::<{ abi::AnimationGraphRow::SIZE }>()
            .0
            .iter()
            .map(move |bytes| {
                let row = abi::AnimationGraphRow::decode(bytes).expect("validated graph row");
                let start = row.parameter_offset as usize * abi::AnimationGraphParameter::SIZE;
                let end = start + row.parameter_count as usize * abi::AnimationGraphParameter::SIZE;
                AnimationGraphPlayback {
                    entity: EntityId(row.id),
                    state: AnimationGraphKey(row.state),
                    paused: row.flags & 1 != 0,
                    seconds: row.seconds,
                    elapsed: row.elapsed,
                    parameters: &parameters[start..end],
                }
            })
    }
}
