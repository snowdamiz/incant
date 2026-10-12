//! Strict tick-local audio controls through the shared native command queue.
use crate::commands;
use incant_runtime::{AudioControl, StructuralCommand};
use rquickjs::{Ctx, Exception, Value};

pub(crate) fn parse<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    args: &[Value<'js>],
) -> rquickjs::Result<StructuralCommand> {
    if matches!(name, "stopAudio" | "restartAudio") {
        let [entity] = args else {
            return Err(Exception::throw_type(
                ctx,
                "audio transport expects one entity",
            ));
        };
        return Ok(StructuralCommand::ControlAudio {
            entity: commands::id(ctx, entity)?,
            control: if name == "stopAudio" {
                AudioControl::Stop
            } else {
                AudioControl::Restart
            },
        });
    }
    let [entity, value] = args else {
        return Err(Exception::throw_type(
            ctx,
            "audio control expects an entity and a value",
        ));
    };
    let entity = commands::id(ctx, entity)?;
    let control = if name == "setAudioPlaying" {
        AudioControl::SetPlaying(
            value
                .as_bool()
                .ok_or_else(|| Exception::throw_type(ctx, "audio playing must be boolean"))?,
        )
    } else if name == "seekAudio" {
        AudioControl::Seek(
            value
                .as_number()
                .filter(|s| (0. ..=3600.).contains(s))
                .ok_or_else(|| {
                    Exception::throw_type(ctx, "audio seek must be finite in [0,3600] seconds")
                })?,
        )
    } else {
        // Check the original f64 before conversion: a barely out-of-range value
        // must not become valid by rounding to f32 at the native boundary.
        let db = value
            .as_number()
            .filter(|db| (-80. ..=24.).contains(db))
            .ok_or_else(|| Exception::throw_type(ctx, "audio gain must be finite in [-80,24] dB"))?
            as f32;
        match name {
            "setAudioGain" => AudioControl::SourceGain(db),
            "setAudioBusGain" => AudioControl::BusGain(db),
            _ => return Err(Exception::throw_type(ctx, "unsupported audio control")),
        }
    };
    Ok(StructuralCommand::ControlAudio { entity, control })
}
