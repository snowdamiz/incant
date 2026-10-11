use incant_runtime::{FlipbookControl, FrameCommands, StableId, StructuralCommand};
use serde::Deserialize;
use std::{fs::File, io::Read, path::Path};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub fn counts(entities: usize, flipbooks: usize) -> Result<(), Error> {
    if entities > 4096 || flipbooks > 256 {
        return Err("native_frames requires <=4096 entities and <=256 flipbooks".into());
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub width: u32,
    pub height: u32,
    pub ticks: Vec<u64>,
    #[serde(default)]
    pub controls: Vec<Control>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub tick: u64,
    pub entity: String,
    pub action: Action,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Play,
    Pause,
    Restart,
    Seek { frame: u16 },
}
impl Request {
    pub fn validate(&self) -> Result<(), Error> {
        if !(16..=1024).contains(&self.width)
            || !(16..=1024).contains(&self.height)
            || self.ticks.is_empty()
            || self.ticks.len() > 64
            || self.ticks.windows(2).any(|w| w[0] >= w[1])
            || self.ticks.last().is_none_or(|&tick| tick > 36_000)
            || self.controls.len() > 256
            || self.controls.windows(2).any(|w| w[0].tick > w[1].tick)
            || u64::from(self.width) * u64::from(self.height) * 4 * self.ticks.len() as u64
                > 256 * 1024 * 1024
        {
            return Err("invalid capture dimensions, clock, ordering or limits".into());
        }
        for control in &self.controls {
            if control.tick > *self.ticks.last().unwrap() {
                return Err("control is after the last capture".into());
            }
            id(&control.entity)?;
        }
        Ok(())
    }
    pub fn commands(&self, tick: u64) -> Result<FrameCommands, Error> {
        let mut commands = FrameCommands::default();
        for c in self.controls.iter().filter(|c| c.tick == tick) {
            commands.push(StructuralCommand::ControlFlipbook {
                entity: id(&c.entity)?,
                control: match c.action {
                    Action::Play => FlipbookControl::Play,
                    Action::Pause => FlipbookControl::Pause,
                    Action::Restart => FlipbookControl::Restart,
                    Action::Seek { frame } => FlipbookControl::Seek(frame),
                },
            })?;
        }
        Ok(commands)
    }
}
pub fn id(text: &str) -> Result<StableId, Error> {
    if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("ID must be exactly 32 hexadecimal digits of the cooked stable ID".into());
    }
    Ok(StableId(u128::from_str_radix(text, 16)?.to_be_bytes()))
}
pub fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("capture input size limit".into());
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request(ticks: Vec<u64>) -> Request {
        Request {
            width: 64,
            height: 64,
            ticks,
            controls: vec![],
        }
    }
    #[test]
    fn rejects_bad_clocks_ids_controls_and_pixel_budgets() {
        counts(4096, 256).unwrap();
        assert!(counts(4097, 256).is_err());
        assert!(counts(4096, 257).is_err());
        for ticks in [vec![], vec![1, 1], vec![2, 1], vec![36_001], vec![0; 65]] {
            assert!(request(ticks).validate().is_err());
        }
        let mut r = request(vec![0, 36_000]);
        r.validate().unwrap();
        r.controls.push(Control {
            tick: 36_001,
            entity: "0".repeat(32),
            action: Action::Pause,
        });
        assert!(r.validate().is_err());
        r.controls[0].tick = 0;
        r.controls[0].entity = "é".repeat(16);
        assert!(r.validate().is_err());
        r.controls.clear();
        r.width = 1025;
        assert!(r.validate().is_err());
        r.width = 1024;
        r.height = 1024;
        r.ticks = (0..64).collect();
        r.validate().unwrap(); // Exactly 256 MiB decoded output bound.
        r.ticks.push(64);
        assert!(r.validate().is_err());
        assert!(
            serde_json::from_str::<Request>(
                r#"{"width":64,"height":64,"ticks":[0],"unexpected":0}"#
            )
            .is_err()
        );
    }
    #[test]
    fn bounded_read_rejects_excess_before_deserialization() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("input");
        std::fs::write(&path, b"12345").unwrap();
        assert!(read(&path, 4).is_err());
        assert_eq!(read(&path, 5).unwrap(), b"12345");
    }
}
