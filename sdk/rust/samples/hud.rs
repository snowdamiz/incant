//! Bounded HUD declarations, routed outcomes and ordered gameplay commands.
#![no_std]
use core::fmt::Write;
use incant_module::{EntityId, Error, HudControl, HudEventKind, Key, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn id(n: u128) -> EntityId {
    EntityId::from_u128(n)
}
struct Message {
    bytes: [u8; 1024],
    len: usize,
}
impl Message {
    fn new() -> Self {
        Self {
            bytes: [0; 1024],
            len: 0,
        }
    }
    fn text(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
}
impl Write for Message {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(core::fmt::Error)?;
        if end > self.bytes.len() {
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn simulate(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    let mut scratch = [0; 8192];
    let before = tick.hud(&mut scratch)?;
    let button = before
        .widgets()
        .find(|w| {
            matches!(
                w.role,
                incant_module::HudRole::Label | incant_module::HudRole::Button
            )
        })
        .ok_or(Error::Callback)?
        .entity;
    let root = before
        .widgets()
        .find(|w| w.role == incant_module::HudRole::Root)
        .ok_or(Error::Callback)?
        .entity;
    let world_text = before.texts().next().ok_or(Error::Callback)?.entity;
    let mut physical = [0; 16384];
    let input = tick.input(&mut physical)?;
    // The composed router owns Enter/Tab while the field is active. A physical
    // key not owned by UI outside active text editing remains gameplay input.
    if before.text_input()
        && (input.keyboard.held(Key::Enter as u8) || input.keyboard.pressed(Key::Enter as u8))
    {
        return Err(Error::Callback);
    }
    let mut message = Message::new();
    write!(
        &mut message,
        "{}|focus={}|text={}|discard={}",
        before.tick(),
        before.focused().map_or(0, |v| u128::from_be_bytes(v.0)),
        before.text_input(),
        before.discarded_events()
    )
    .map_err(|_| Error::Capacity)?;
    for w in before.widgets() {
        write!(
            &mut message,
            "|w{}:{}:{}",
            u128::from_be_bytes(w.entity.0),
            w.disabled,
            w.text.unwrap_or("-")
        )
        .map_err(|_| Error::Capacity)?;
    }
    for t in before.texts() {
        write!(
            &mut message,
            "|t{}:{}",
            u128::from_be_bytes(t.entity.0),
            t.text
        )
        .map_err(|_| Error::Capacity)?;
    }
    for e in before.events() {
        write!(
            &mut message,
            "|e{}:{}:{}",
            u128::from_be_bytes(e.entity.0),
            e.kind as u8,
            e.value.unwrap_or("-")
        )
        .map_err(|_| Error::Capacity)?;
    }
    let mut command = [0; 512];
    let next = if before.events().any(|e| e.kind == HudEventKind::Submitted) {
        "Submitted"
    } else if before
        .events()
        .any(|e| e.kind == HudEventKind::ValueChanged)
    {
        "Edited 中文"
    } else {
        "Health 8"
    };
    tick.control_hud(button, HudControl::UiText(next), &mut command)?;
    tick.control_hud(world_text, HudControl::Text("Pickup 世界"), &mut command)?;
    if before.tick() == 4 {
        tick.control_hud(button, HudControl::UiDisabled(true), &mut command)?;
    }
    if before.tick() == 5 {
        tick.control_hud(button, HudControl::UiDisabled(false), &mut command)?;
    }
    let mut after_bytes = [0; 8192];
    let after = tick.hud(&mut after_bytes)?;
    if before.widgets().ne(after.widgets())
        || before.texts().ne(after.texts())
        || before.events().ne(after.events())
    {
        return Err(Error::Callback);
    }
    // Same prefix as failures below: numeric+HUD+logs must commit together.
    for n in 0..tick.chunk_count()? {
        let chunk = tick.chunk(n)?;
        let mut ids = [id(0); 16];
        let mut values = [[0.; 10]; 16];
        tick.read_ids(chunk, &mut ids)?;
        tick.read_transforms(chunk, &mut values)?;
        for (owner, t) in ids.iter().zip(&mut values).take(chunk.len()) {
            if *owner == world_text {
                t[0] += 6. * dt;
            }
        }
        tick.write_transforms(chunk, &values)?;
    }
    tick.log(message.text(), LogLevel::Info)?;
    if input.keyboard.held(Key::F1 as u8) {
        tick.control_hud(root, HudControl::UiText("bad role"), &mut command)?;
    }
    if input.keyboard.held(Key::F2 as u8) {
        return Err(Error::Callback);
    }
    if input.keyboard.held(Key::F3 as u8) {
        for _ in 0..257 {
            let _ = tick.control_hud(button, HudControl::UiDisabled(false), &mut command);
        }
    }
    if input.keyboard.held(Key::F4 as u8) {
        let _ = tick.hud(&mut []);
    }
    if input.keyboard.held(Key::F5 as u8) {
        tick.despawn(&button)?;
        tick.control_hud(button, HudControl::UiText("dead"), &mut command)?;
    }
    Ok(())
}
