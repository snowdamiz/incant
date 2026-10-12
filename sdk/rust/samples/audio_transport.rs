//! Real bounded audio gameplay: copied declarations and ordered native transport.
#![no_std]
use core::fmt::Write;
use incant_module::{
    AudioCommand, AudioControl as A, AudioDeclarations, EntityId, Error, IDENTITY, LogLevel,
    Result, Tick,
};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn id(n: u128) -> EntityId {
    EntityId::from_u128(n)
}
fn words(target: &mut [f64], value: EntityId) {
    for (out, bytes) in target.iter_mut().zip(value.0.as_chunks::<4>().0) {
        *out = f64::from(u32::from_be_bytes(*bytes));
    }
}
fn values(s: AudioDeclarations<'_>) -> Result<[f64; 186]> {
    if s.len() != 5 {
        return Err(Error::Malformed);
    }
    let mut values = [0.; 186];
    for (i, node) in s.iter().enumerate() {
        let r = &mut values[i * 36..][..36];
        words(&mut r[..4], node.entity);
        if let Some(bus) = node.bus {
            r[4] += 1.;
            r[5] = f64::from(bus.parent.is_some());
            if let Some(parent) = bus.parent {
                words(&mut r[6..10], parent);
            }
            r[10] = bus.gain_db;
        }
        if let Some(source) = node.source {
            r[4] += 2.;
            words(&mut r[11..15], source.clip);
            r[15] = f64::from(source.bus.is_some());
            r[16] = f64::from(
                u32::from(source.playing)
                    | (u32::from(source.looping) << 1)
                    | (u32::from(source.streaming) << 2)
                    | (u32::from(source.spatial.is_some()) << 3),
            );
            if let Some(bus) = source.bus {
                words(&mut r[17..21], bus);
            }
            r[21] = f64::from((source.transport_epoch >> 32) as u32);
            r[22] = f64::from(source.transport_epoch as u32);
            r[23..27].copy_from_slice(&[
                source.gain_db,
                source.pan,
                source.rate,
                source.start_seconds,
            ]);
            if let Some(spatial) = source.spatial {
                r[27] = spatial.min_distance;
                r[28] = spatial.max_distance;
                r[29..32].copy_from_slice(&spatial.world_position);
            }
        }
        if let Some(listener) = node.listener {
            r[4] += 4.;
            r[29..32].copy_from_slice(&listener.world_position);
            r[32..36].copy_from_slice(&listener.world_rotation);
        }
    }
    words(&mut values[180..184], s.scene());
    values[184] = f64::from((s.tick() >> 32) as u32);
    values[185] = f64::from(s.tick() as u32);
    Ok(values)
}
struct Buffer {
    bytes: [u8; 1024],
    len: usize,
}
impl Write for Buffer {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let dst = self
            .bytes
            .get_mut(self.len..self.len + text.len())
            .ok_or(core::fmt::Error)?;
        dst.copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut before = [0; 4096];
    let snapshot = tick.audio(&mut before)?;
    let turn = u32::try_from(snapshot.tick().checked_sub(1).ok_or(Error::Malformed)?)
        .map_err(|_| Error::Bounds)?;
    let values = values(snapshot)?;
    for (part, row) in values.as_chunks::<31>().0.iter().enumerate() {
        let mut buffer = Buffer {
            bytes: [0; 1024],
            len: 0,
        };
        write!(
            buffer,
            "{{\"tick\":{},\"part\":{},\"values\":[",
            snapshot.tick(),
            part
        )
        .map_err(|_| Error::Capacity)?;
        for (i, value) in row.iter().enumerate() {
            if i != 0 {
                buffer.write_str(",").map_err(|_| Error::Capacity)?;
            }
            write!(buffer, "{value:?}").map_err(|_| Error::Capacity)?;
        }
        buffer.write_str("]}").map_err(|_| Error::Capacity)?;
        let text =
            core::str::from_utf8(&buffer.bytes[..buffer.len]).map_err(|_| Error::Malformed)?;
        tick.log(text, LogLevel::Info)?;
    }
    for n in 0..tick.chunk_count()? {
        let chunk = tick.chunk(n)?;
        let mut ids = [id(0); 256];
        let ids = tick.read_ids(chunk, &mut ids)?;
        let mut transforms = [IDENTITY; 256];
        let transforms = tick.read_transforms(chunk, &mut transforms)?;
        for (entity, transform) in ids.iter().zip(transforms.iter_mut()) {
            let n = u128::from_be_bytes(entity.0);
            if (100..162).contains(&n) {
                let at = (n as usize - 100) * 3;
                transform[..3].copy_from_slice(&values[at..at + 3]);
            } else if n == 3 {
                transform[0] = f64::from(turn % 13) / 16.;
            } else if n == 5 {
                transform[2] = -f64::from(turn % 7) / 32.;
            }
        }
        tick.write_transforms(chunk, transforms)?;
    }
    let controls: &[(u128, A)] = match turn {
        0 => &[
            (2, A::Restart),
            (3, A::Restart),
            (2, A::Seek(0.0013)),
            (3, A::Seek(0.0021)),
        ],
        2 => &[(2, A::Stop), (2, A::Seek(0.0023)), (3, A::Seek(0.01))],
        4 => &[
            (2, A::SourceGain(-8.)),
            (1, A::BusGain(-6.)),
            (4, A::BusGain(-3.)),
        ],
        5 => &[(2, A::SetPlaying(true))],
        8 => &[(2, A::Restart), (2, A::Restart)],
        9 => &[(3, A::SetPlaying(false)), (3, A::Seek(0.0123))],
        11 => &[(3, A::SetPlaying(true))],
        13 => &[(2, A::Seek(0.01))],
        15 => &[(2, A::SetPlaying(true))],
        17 | 30 | 36 => &[(2, A::Restart)],
        19 => &[(3, A::Stop), (2, A::Stop)],
        23 => &[(3, A::Restart), (3, A::Seek(0.031))],
        32 => &[(2, A::Stop)],
        41 => &[(3, A::SetPlaying(false))],
        43 => &[(3, A::SetPlaying(true))],
        _ => &[],
    };
    let mut commands = [AudioCommand {
        entity: id(0),
        control: A::Stop,
    }; 4];
    for (dst, &(n, control)) in commands.iter_mut().zip(controls) {
        *dst = AudioCommand {
            entity: id(n),
            control,
        };
    }
    let mut scratch = [0; 256];
    tick.audio_commands(&commands[..controls.len()], &mut scratch)?;
    let mut after = [0; 4096];
    tick.audio(&mut after)?;
    if before != after {
        return Err(Error::Malformed);
    }
    Ok(())
}
