//! Copied pre-command declarations plus post-routing focus and UI outcomes.
//! Declaration text is distinct from a TextInput's live edit buffer; notifications
//! carry accepted edits/submissions. All strings borrow caller-owned scratch.
use crate::{EntityId, Error, Result};
pub use abi::hud::Control as HudControl;
pub use abi::{HudEventKind, HudRole};
use incant_wasm_abi as abi;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudWidget<'a> {
    pub entity: EntityId,
    pub role: HudRole,
    pub disabled: bool,
    pub text: Option<&'a str>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudText<'a> {
    pub entity: EntityId,
    pub text: &'a str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudEvent<'a> {
    pub entity: EntityId,
    pub kind: HudEventKind,
    pub value: Option<&'a str>,
}
#[derive(Clone, Copy)]
pub struct HudSnapshot<'a>(abi::hud::Frame<'a>);
impl<'a> HudSnapshot<'a> {
    /// Validate every count, UTF-8 range, sorted owner and focus/event association
    /// before exposing any view. Canonical offsets reject aliases/gaps/trailing data.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        abi::hud::Frame::parse(bytes)
            .map(Self)
            .map_err(|_| Error::Malformed)
    }
    pub fn scene(&self) -> EntityId {
        EntityId(self.0.header.scene)
    }
    pub fn tick(&self) -> u64 {
        self.0.header.tick
    }
    pub fn focused(&self) -> Option<EntityId> {
        (self.0.header.flags & 1 != 0).then_some(EntityId(self.0.header.focused))
    }
    pub fn text_input(&self) -> bool {
        self.0.header.flags & 2 != 0
    }
    pub fn discarded_events(&self) -> u16 {
        self.0.header.discarded as u16
    }
    pub fn widgets(&self) -> impl ExactSizeIterator<Item = HudWidget<'a>> + 'a {
        self.0.widgets().map(|(r, t)| HudWidget {
            entity: EntityId(r.entity),
            role: HudRole::from_tag(r.role as u8).expect("validated role"),
            disabled: r.disabled != 0,
            text: (r.role >= 2).then_some(t),
        })
    }
    pub fn texts(&self) -> impl ExactSizeIterator<Item = HudText<'a>> + 'a {
        self.0.texts().map(|(r, t)| HudText {
            entity: EntityId(r.entity),
            text: t,
        })
    }
    pub fn events(&self) -> impl ExactSizeIterator<Item = HudEvent<'a>> + 'a {
        self.0.events().map(|(r, t)| HudEvent {
            entity: EntityId(r.entity),
            kind: HudEventKind::from_tag(r.kind as u8).expect("validated event"),
            value: (r.kind != 0).then_some(t),
        })
    }
}
