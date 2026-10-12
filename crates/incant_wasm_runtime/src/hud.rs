//! Callback-owned copies of declarations and post-routing UI notifications.
use crate::BoundaryError as Error;
use incant_input::{UiNotification, UiSnapshot};
use incant_runtime::{NativeWorld, StableId, StructuralCommand, TextControl, UiControl};
use incant_types::UiKind;
use incant_wasm_abi as abi;

const _: () = {
    assert!(abi::HUD_MAX_WIDGETS == incant_types::MAX_UI_NODES);
    assert!(abi::HUD_MAX_TEXTS == incant_runtime::MAX_TEXT_NODES);
    assert!(abi::HUD_MAX_TEXT_BYTES == incant_runtime::MAX_TEXT_BYTES);
    assert!(abi::HUD_MAX_DECLARATION_BYTES == incant_runtime::MAX_SCENE_TEXT_BYTES);
    assert!(abi::HUD_MAX_EVENTS == incant_input::MAX_UI_NOTIFICATIONS);
    assert!(abi::HUD_MAX_EVENT_BYTES == incant_input::MAX_UI_NOTIFICATION_BYTES);
    assert!(abi::HUD_MAX_COMMANDS == incant_runtime::MAX_HUD_COMMANDS);
    assert!(abi::HUD_MAX_COMMAND_BYTES == incant_runtime::MAX_HUD_COMMAND_BYTES);
    assert!(abi::HUD_MAX_BYTES < abi::MEMORY);
};
fn id(text: &str) -> Result<[u8; 16], Error> {
    // Already validated by UiSnapshot, but retain checked canonical conversion.
    if text.len() != 26 || text.as_bytes()[0] > b'7' {
        return Err(Error::Abi("HUD identity"));
    }
    let mut value = 0u128;
    for byte in text.bytes() {
        let digit = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"
            .iter()
            .position(|&b| b == byte)
            .ok_or(Error::Abi("HUD identity"))?;
        value = value
            .checked_mul(32)
            .and_then(|n| n.checked_add(digit as u128))
            .ok_or(Error::Abi("HUD identity"))?;
    }
    Ok(value.to_be_bytes())
}
fn notification(event: &UiNotification) -> (u32, &str, &str) {
    match event {
        UiNotification::Activated { entity_id } => (0, entity_id, ""),
        UiNotification::ValueChanged { entity_id, value } => (1, entity_id, value),
        UiNotification::Submitted { entity_id, value } => (2, entity_id, value),
    }
}
pub(crate) fn encode(
    world: &NativeWorld,
    input: &UiSnapshot,
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    input.validate().map_err(|_| Error::Abi("HUD input"))?;
    let ui = world.ui_view()?;
    let text = world.text_view()?;
    let widgets = ui.nodes().len();
    let texts = text.nodes().len();
    if widgets > abi::HUD_MAX_WIDGETS || texts > abi::HUD_MAX_TEXTS {
        return Err(Error::Limit("HUD owners"));
    }
    let mut bytes = 0usize;
    for content in ui
        .nodes()
        .filter_map(|n| n.node.kind.text().map(|t| t.content.as_str()))
        .chain(text.nodes().map(|(_, t)| t.content.as_ref()))
    {
        if content.len() > abi::HUD_MAX_TEXT_BYTES {
            return Err(Error::Limit("HUD text"));
        }
        bytes = bytes
            .checked_add(content.len())
            .filter(|&n| n <= abi::HUD_MAX_DECLARATION_BYTES)
            .ok_or(Error::Limit("HUD declarations"))?;
    }
    for e in &input.events {
        bytes += notification(e).2.len();
    }
    let size = abi::HudHeader::SIZE
        + widgets * abi::HudWidget::SIZE
        + texts * abi::HudText::SIZE
        + input.events.len() * abi::HudEvent::SIZE
        + bytes;
    if size > abi::HUD_MAX_BYTES {
        return Err(Error::Limit("HUD frame"));
    }
    let header = abi::HudHeader {
        magic: *abi::HUD_MAGIC,
        scene: world.scene_id().0,
        tick: world.tick(),
        widgets: widgets as u32,
        texts: texts as u32,
        events: input.events.len() as u32,
        bytes: bytes as u32,
        focused: input
            .focused
            .as_deref()
            .map(id)
            .transpose()?
            .unwrap_or([0; 16]),
        flags: u32::from(input.focused.is_some()) | u32::from(input.text_input) << 1,
        discarded: u32::from(input.discarded_events),
        widget_stride: abi::HudWidget::SIZE as u32,
        text_stride: abi::HudText::SIZE as u32,
        event_stride: abi::HudEvent::SIZE as u32,
        reserved: 0,
    };
    output.clear();
    output.reserve_exact(size);
    output.extend(header.encode());
    let mut offset = 0;
    for n in ui.nodes() {
        let len = n.node.kind.text().map_or(0, |t| t.content.len());
        let role = match n.node.kind {
            UiKind::Root => 0,
            UiKind::Container => 1,
            UiKind::Label { .. } => 2,
            UiKind::Button { .. } => 3,
            UiKind::TextInput { .. } => 4,
        };
        output.extend(
            abi::HudWidget {
                entity: n.id.0,
                role,
                disabled: u32::from(n.node.disabled),
                offset: offset as u32,
                len: len as u32,
            }
            .encode(),
        );
        offset += len;
    }
    for (id, t) in text.nodes() {
        output.extend(
            abi::HudText {
                entity: id.0,
                offset: offset as u32,
                len: t.content.len() as u32,
            }
            .encode(),
        );
        offset += t.content.len();
    }
    for event in &input.events {
        let (kind, owner, content) = notification(event);
        output.extend(
            abi::HudEvent {
                entity: id(owner)?,
                kind,
                offset: offset as u32,
                len: content.len() as u32,
                reserved: 0,
            }
            .encode(),
        );
        offset += content.len();
    }
    for content in ui
        .nodes()
        .filter_map(|n| n.node.kind.text().map(|t| t.content.as_str()))
        .chain(text.nodes().map(|(_, t)| t.content.as_ref()))
    {
        output.extend_from_slice(content.as_bytes());
    }
    for e in &input.events {
        output.extend_from_slice(notification(e).2.as_bytes());
    }
    debug_assert_eq!(output.len(), size);
    abi::hud::Frame::parse(output).map_err(|_| Error::Abi("HUD frame association"))?;
    Ok(())
}
pub(crate) fn command(bytes: &[u8]) -> Result<StructuralCommand, Error> {
    let (entity, control) = abi::hud::command(bytes).map_err(|_| Error::Abi("HUD command"))?;
    let entity = StableId(entity);
    Ok(match control {
        abi::hud::Control::UiText(text) => StructuralCommand::ControlUi {
            entity,
            control: UiControl::Text(text.into()),
        },
        abi::hud::Control::UiDisabled(disabled) => StructuralCommand::ControlUi {
            entity,
            control: UiControl::Disabled(disabled),
        },
        abi::hud::Control::Text(text) => StructuralCommand::ControlText {
            entity,
            control: TextControl::Content(text.into()),
        },
    })
}
