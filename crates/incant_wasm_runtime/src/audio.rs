//! Copied desired audio declarations and native staged transport controls.
use crate::BoundaryError as Error;
use incant_runtime::{AudioControl, NativeWorld, StableId, StructuralCommand};
use incant_wasm_abi as abi;

const _: () = {
    assert!(abi::AUDIO_MAX_BYTES < abi::MEMORY);
    assert!(abi::AUDIO_MAX_COMMAND_BYTES < abi::MEMORY);
    assert!(abi::AUDIO_MAX_NODES == incant_runtime::MAX_AUDIO_NODES);
    assert!(abi::AUDIO_MAX_BUSES == incant_runtime::MAX_AUDIO_BUSES);
    assert!(abi::AUDIO_MAX_SOURCES == incant_runtime::MAX_AUDIO_SOURCES);
    assert!(abi::AUDIO_MAX_COMMANDS == abi::COMMANDS);
};

pub(crate) fn encode(world: &NativeWorld, output: &mut Vec<u8>) -> Result<(), Error> {
    let count = world.audio_nodes().len();
    if count > abi::AUDIO_MAX_NODES {
        return Err(Error::Limit("audio declaration snapshot"));
    }
    output.clear();
    output.extend(
        abi::AudioHeader {
            magic: *abi::AUDIO_MAGIC,
            scene: world.scene_id().0,
            tick: world.tick(),
            count: count as u32,
            stride: abi::AudioRow::SIZE as u32,
        }
        .encode(),
    );
    for (node, matrix) in world.audio_nodes() {
        let mut r = abi::AudioRow::decode(&[0; abi::AudioRow::SIZE]).unwrap();
        r.entity = node.entity.0;
        if let Some(bus) = &node.bus {
            r.roles |= 1;
            r.bus_gain = f64::from(bus.gain_db);
            r.bus_parent_present = u32::from(bus.parent.is_some());
            r.bus_parent = bus.parent.map_or([0; 16], |id| id.0);
        }
        if let Some(source) = &node.source {
            r.roles |= 2;
            r.clip = source.clip.0;
            r.source_bus_present = u32::from(source.bus.is_some());
            r.source_bus = source.bus.map_or([0; 16], |id| id.0);
            r.source_flags = u32::from(source.playing)
                | (u32::from(source.looping) << 1)
                | (u32::from(source.streaming) << 2);
            r.transport_epoch = source.transport_epoch;
            r.gain = f64::from(source.gain_db);
            r.pan = f64::from(source.pan);
            r.rate = source.rate;
            r.start_seconds = source.start_seconds;
            if let Some(spatial) = &source.spatial {
                r.source_flags |= 8;
                r.min_distance = f64::from(spatial.min_distance);
                r.max_distance = f64::from(spatial.max_distance);
            }
        }
        if node.listener {
            r.roles |= 4;
            r.listener_rotation = world
                .world_rotation(node.entity)
                .ok_or(Error::Abi("audio listener rotation absent"))?;
        }
        if node.listener || r.source_flags & 8 != 0 {
            r.world_translation = [matrix[3][0], matrix[3][1], matrix[3][2]];
        }
        output.extend(r.encode());
    }
    Ok(())
}
/// The entire batch has already passed canonical validation before this conversion.
pub(crate) fn command(r: abi::AudioCommand) -> StructuralCommand {
    use abi::AudioControlKind as K;
    let control = match abi::audio::validate_command(&r).expect("validated audio batch") {
        K::SetPlaying => AudioControl::SetPlaying(r.value == 1.),
        K::Stop => AudioControl::Stop,
        K::Restart => AudioControl::Restart,
        K::Seek => AudioControl::Seek(r.value),
        K::SourceGain => AudioControl::SourceGain(r.value as f32),
        K::BusGain => AudioControl::BusGain(r.value as f32),
    };
    StructuralCommand::ControlAudio {
        entity: StableId(r.entity),
        control,
    }
}
