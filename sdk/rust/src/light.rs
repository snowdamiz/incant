//! Existing punctual-light controls, using the shared accepted tick transaction.
use crate::{EntityId, Error, Result};
use incant_wasm_abi as abi;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightControl {
    Color([f64; 3]),
    Intensity(f64),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightCommand {
    pub entity: EntityId,
    pub control: LightControl,
}
impl LightCommand {
    pub(crate) fn wire(self) -> Result<abi::LightCommand> {
        let (kind, values) = match self.control {
            LightControl::Color(v) => (abi::LightControlKind::Color, v),
            LightControl::Intensity(v) => (abi::LightControlKind::Intensity, [v, 0., 0.]),
        };
        let row = abi::LightCommand {
            entity: self.entity.0,
            kind: kind as u32,
            reserved: 0,
            values,
        };
        abi::light::validate_command(&row).map_err(|_| Error::Bounds)?;
        Ok(row)
    }
}
