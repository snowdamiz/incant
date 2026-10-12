//! Desired committed audio declarations, not actual mixer cursors or voice state.
//! Rows are copied before callback commands; retained snapshots do not change when
//! a later transport request is queued. The offline/device audio host owns playback.
use crate::{EntityId, Error, Result};
use incant_wasm_abi as abi;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioBus {
    pub parent: Option<EntityId>,
    pub gain_db: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioSpatial {
    pub world_position: [f64; 3],
    pub min_distance: f64,
    pub max_distance: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioListener {
    pub world_position: [f64; 3],
    /// Unit XYZW rotation composed without inherited scale/shear.
    pub world_rotation: [f64; 4],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioSource {
    pub clip: EntityId,
    pub bus: Option<EntityId>,
    /// Exact monotonic transport request identity, including values above 2^53.
    /// Stop/restart/seek advance it; the mixer cursor is not represented here.
    pub transport_epoch: u64,
    pub gain_db: f64,
    pub pan: f64,
    pub rate: f64,
    pub start_seconds: f64,
    /// Desired state. An ended one-shot may still have this flag set.
    pub playing: bool,
    pub looping: bool,
    pub streaming: bool,
    pub spatial: Option<AudioSpatial>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioDeclaration {
    pub entity: EntityId,
    pub bus: Option<AudioBus>,
    pub source: Option<AudioSource>,
    pub listener: Option<AudioListener>,
}
/// A bounded, validated start-of-callback frame over caller-owned bytes.
#[derive(Clone, Copy)]
pub struct AudioDeclarations<'a>(abi::audio::Snapshot<'a>);
impl<'a> AudioDeclarations<'a> {
    /// Validates the complete frame, roles, optional references and bus topology
    /// before exposing any row. Unaligned slices work and no allocation occurs.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        abi::audio::Snapshot::parse(bytes)
            .map(Self)
            .map_err(|_| Error::Malformed)
    }
    pub fn scene(&self) -> EntityId {
        EntityId(self.0.scene)
    }
    pub fn tick(&self) -> u64 {
        self.0.tick
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = AudioDeclaration> + '_ {
        self.0.iter().map(declaration)
    }
    pub fn find(&self, entity: EntityId) -> Option<AudioDeclaration> {
        self.0.find(entity.0).map(declaration)
    }
}
fn declaration(r: abi::AudioRow) -> AudioDeclaration {
    AudioDeclaration {
        entity: EntityId(r.entity),
        bus: (r.roles & 1 != 0).then_some(AudioBus {
            parent: (r.bus_parent_present != 0).then_some(EntityId(r.bus_parent)),
            gain_db: r.bus_gain,
        }),
        source: (r.roles & 2 != 0).then_some(AudioSource {
            clip: EntityId(r.clip),
            bus: (r.source_bus_present != 0).then_some(EntityId(r.source_bus)),
            transport_epoch: r.transport_epoch,
            gain_db: r.gain,
            pan: r.pan,
            rate: r.rate,
            start_seconds: r.start_seconds,
            playing: r.source_flags & 1 != 0,
            looping: r.source_flags & 2 != 0,
            streaming: r.source_flags & 4 != 0,
            spatial: (r.source_flags & 8 != 0).then_some(AudioSpatial {
                world_position: r.world_translation,
                min_distance: r.min_distance,
                max_distance: r.max_distance,
            }),
        }),
        listener: (r.roles & 4 != 0).then_some(AudioListener {
            world_position: r.world_translation,
            world_rotation: r.listener_rotation,
        }),
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioControl {
    /// Resume/pause desired state. Repeated true does not restart an ended voice.
    SetPlaying(bool),
    /// Reset to the beginning and stop, even when already stopped/paused.
    Stop,
    /// Reset to the beginning and play, even when already playing/ended.
    Restart,
    /// Finite [0,3600] seconds; preserves desired playing state. The audio host
    /// additionally validates the position against its captured clip duration.
    Seek(f64),
    SourceGain(f64),
    BusGain(f64),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioCommand {
    pub entity: EntityId,
    pub control: AudioControl,
}
impl AudioCommand {
    pub(crate) fn wire(self) -> Result<abi::AudioCommand> {
        use abi::AudioControlKind as K;
        let (kind, value) = match self.control {
            AudioControl::SetPlaying(v) => (K::SetPlaying, f64::from(v)),
            AudioControl::Stop => (K::Stop, 0.),
            AudioControl::Restart => (K::Restart, 0.),
            AudioControl::Seek(v) => (K::Seek, v),
            AudioControl::SourceGain(v) => (K::SourceGain, v),
            AudioControl::BusGain(v) => (K::BusGain, v),
        };
        let row = abi::AudioCommand {
            entity: self.entity.0,
            kind: kind as u32,
            reserved: 0,
            value,
        };
        abi::audio::validate_command(&row).map_err(|_| Error::Bounds)?;
        Ok(row)
    }
}
