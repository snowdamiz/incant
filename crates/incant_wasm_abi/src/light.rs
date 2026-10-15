//! Canonical all-or-nothing copied punctual-light commands.
use crate::{
    LIGHT_COMMAND_MAGIC, LIGHT_MAX_COMMANDS, LightCommand, LightCommandHeader, LightControlKind,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malformed;
#[derive(Clone, Copy)]
pub struct Commands<'a> {
    rows: &'a [u8],
}
impl<'a> Commands<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Malformed> {
        let h = LightCommandHeader::decode(bytes).ok_or(Malformed)?;
        if &h.magic != LIGHT_COMMAND_MAGIC
            || h.count as usize > LIGHT_MAX_COMMANDS
            || h.stride as usize != LightCommand::SIZE
            || bytes.len() != LightCommandHeader::SIZE + h.count as usize * LightCommand::SIZE
        {
            return Err(Malformed);
        }
        let result = Self {
            rows: &bytes[LightCommandHeader::SIZE..],
        };
        for row in result.iter() {
            validate_command(&row)?;
        }
        Ok(result)
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = LightCommand> + '_ {
        self.rows
            .as_chunks::<{ LightCommand::SIZE }>()
            .0
            .iter()
            .map(|row| LightCommand::decode(row).unwrap())
    }
}
pub fn validate_command(row: &LightCommand) -> Result<LightControlKind, Malformed> {
    let kind = u8::try_from(row.kind)
        .ok()
        .and_then(LightControlKind::from_tag)
        .ok_or(Malformed)?;
    if row.reserved != 0
        || !match kind {
            LightControlKind::Color => row.values.iter().all(|v| (0. ..=1.).contains(v)),
            LightControlKind::Intensity => {
                (0. ..=1_000_000.).contains(&row.values[0])
                    && row.values[1].to_bits() == 0
                    && row.values[2].to_bits() == 0
            }
        }
    {
        return Err(Malformed);
    }
    Ok(kind)
}
