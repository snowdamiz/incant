//! CPU screen-space ownership attached to the exact native world. Preparation
//! never changes the accepted widget frame, even if spatial consumption refuses.
use incant_game_runtime::RoutedUiInput;
use incant_input::InputEvent;
use incant_render_data::{UiEdit, UiInput, UiRect, UiRole, UiWidget};
use incant_render_native::NativeRenderInput;
use incant_runtime::{NativeWorld, UiView, WorldIdentity};
use incant_ui::{
    PendingUiCheckpoint, Rect, RoutedUi, UiCheckpointError, UiError, UiEvent, UiFonts, UiKind,
    UiPaint, UiPending, UiRouter, UiRuntime, UiVisual, Widget, WidgetVisual,
};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct UiSettings {
    /// Logical screen pixels. A host resize changes layout, never authored data.
    pub viewport: [f32; 2],
    pub atlas: incant_ui::AtlasConfig,
}
impl UiSettings {
    pub fn new(viewport: [f32; 2]) -> Self {
        Self {
            viewport,
            atlas: incant_ui::AtlasConfig {
                width: 2048,
                height: 2048,
                max_glyphs: 8192,
            },
        }
    }
}
pub(crate) struct Binding {
    pub paint: Arc<incant_ui::UiPaint>,
    /// Draw input of the accepted paint and widget state.
    pub input: Arc<UiInput>,
    atlas: incant_ui::AtlasConfig,
    identity: WorldIdentity,
    generation: u64,
    viewport: [f32; 2],
    pub runtime: UiRuntime,
    fonts: UiFonts,
    router: UiRouter,
    pending: Option<RoutedUi>,
}
pub(crate) struct Prepared {
    paint: Arc<incant_ui::UiPaint>,
    input: Arc<UiInput>,
    viewport: [f32; 2],
    generation: u64,
    fonts: Option<UiFonts>,
    state: State,
}
enum State {
    Input(Box<RoutedUi>),
    Refresh(Box<UiPending>),
}
impl Binding {
    pub fn load(
        world: &NativeWorld,
        input: &NativeRenderInput,
        settings: UiSettings,
        checkpoint: Option<&PendingUiCheckpoint>,
    ) -> Result<Self, UiCheckpointError> {
        let view = world
            .ui_view()
            .map_err(|_| UiError::Invalid("unusable native world"))?;
        let mut runtime = UiRuntime::for_world(world)?;
        let mut fonts = UiFonts::new(input.font_sources().cloned().collect(), runtime.widgets())?;
        if let Some(checkpoint) = checkpoint {
            runtime = checkpoint.restore(world, settings.viewport, &mut fonts)?;
        }
        let first = runtime.prepare_refresh(settings.viewport, &mut fonts)?;
        let paint =
            incant_ui::UiPaint::prepare(&runtime, &first, &mut fonts, settings.atlas, None)?;
        let input = draw_input(&first.visual(&runtime)?, &paint, None)?;
        runtime.commit_for_world(first, world)?;
        Ok(Self {
            paint,
            input,
            atlas: settings.atlas,
            identity: view.identity().clone(),
            generation: view.generation(),
            viewport: settings.viewport,
            runtime,
            fonts,
            router: UiRouter::default(),
            pending: None,
        })
    }
    pub fn viewport(&self) -> [f32; 2] {
        self.viewport
    }
    pub fn stage(
        &mut self,
        world: &NativeWorld,
        events: &[InputEvent],
        receipts: &[UiEvent],
    ) -> Result<RoutedUiInput, UiError> {
        let view = world
            .ui_view()
            .map_err(|_| UiError::Invalid("unusable native world"))?;
        if view.identity() != &self.identity
            || view.generation() != self.generation
            || self.pending.is_some()
        {
            return Err(UiError::Stale);
        }
        let mut pending = self.router.prepare(
            &self.runtime,
            events,
            receipts,
            1.0 / f64::from(world.tick_rate()),
            self.viewport,
            &mut self.fonts,
        )?;
        let (events, snapshot) = pending.take_input();
        self.pending = Some(pending);
        Ok(RoutedUiInput { events, snapshot })
    }
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    pub fn prepare(
        &mut self,
        world: &NativeWorld,
        viewport: [f32; 2],
    ) -> Result<Prepared, UiError> {
        let view = world
            .ui_view()
            .map_err(|_| UiError::Invalid("unusable native world"))?;
        if view.identity() != &self.identity {
            return Err(UiError::Stale);
        }
        if self.pending.is_some() && viewport != self.viewport {
            return Err(UiError::Invalid("resize during an accepted input tick"));
        }
        let mut state = if let Some(pending) = self.pending.take() {
            State::Input(Box::new(pending))
        } else {
            State::Refresh(Box::new(
                self.runtime.prepare_refresh(viewport, &mut self.fonts)?,
            ))
        };
        let mut fonts = if view.generation() != self.generation {
            let widgets = widgets(&view);
            // Both old/new bounded shaping engines can coexist. Raw immutable
            // font bytes retain the same Arc; no project/cache I/O occurs here.
            let mut fonts = UiFonts::new(self.fonts.sources().cloned().collect(), &widgets)?;
            state = match state {
                State::Input(pending) => State::Input(Box::new((*pending).reconcile(
                    &self.runtime,
                    widgets,
                    &mut fonts,
                )?)),
                State::Refresh(pending) => State::Refresh(Box::new(
                    self.runtime.reconcile(*pending, widgets, &mut fonts)?,
                )),
            };
            Some(fonts)
        } else {
            None
        };
        let pending = match &state {
            State::Input(routed) => routed.pending(),
            State::Refresh(pending) => pending,
        };
        let paint = incant_ui::UiPaint::prepare(
            &self.runtime,
            pending,
            fonts.as_mut().unwrap_or(&mut self.fonts),
            self.atlas,
            Some(&self.paint),
        )?;
        let input = draw_input(
            &pending.visual(&self.runtime)?,
            &paint,
            Some((&self.paint, &self.input)),
        )?;
        Ok(Prepared {
            paint,
            input,
            viewport,
            generation: view.generation(),
            fonts,
            state,
        })
    }
    pub fn commit(
        &mut self,
        prepared: Prepared,
        world: &NativeWorld,
    ) -> Result<(), UiCheckpointError> {
        match prepared.state {
            State::Input(pending) => {
                self.router
                    .commit_for_world(&mut self.runtime, *pending, world)?
            }
            State::Refresh(pending) => self.runtime.commit_for_world(*pending, world)?,
        }
        if let Some(fonts) = prepared.fonts {
            self.fonts = fonts;
        }
        self.paint = prepared.paint;
        self.input = prepared.input;
        self.viewport = prepared.viewport;
        self.generation = prepared.generation;
        Ok(())
    }
}
impl Prepared {
    pub fn input(&self) -> &Arc<UiInput> {
        &self.input
    }
}

fn ltrb(rect: Rect) -> UiRect {
    [
        rect.origin[0],
        rect.origin[1],
        rect.origin[0] + rect.size[0],
        rect.origin[1] + rect.size[1],
    ]
}
fn edit(node: &WidgetVisual<'_>) -> Option<UiEdit> {
    node.text.map(|buffer| UiEdit {
        selection: buffer.display_selection(),
        preedit: buffer.preedit().map(|p| p.display_range),
    })
}
/// Text-bearing widgets of `visual` in canonical paint order. Root/Container
/// nodes have no appearance and no paint text.
fn drawn<'a>(visual: &'a UiVisual<'a>) -> impl Iterator<Item = (usize, WidgetVisual<'a>)> + 'a {
    (0..visual.len())
        .filter_map(|position| visual.node(position).map(|node| (position, node)))
        .filter(|(_, node)| node.widget.node.kind.text().is_some())
}
/// The renderer's input for one frame: the exact paint (layout, shaped glyphs,
/// atlas) joined with the same frame's widget state. Nothing is re-shaped or
/// inferred from text; an unchanged paint and state reuse the accepted Arc.
fn draw_input(
    visual: &UiVisual<'_>,
    paint: &Arc<UiPaint>,
    accepted: Option<(&Arc<UiPaint>, &Arc<UiInput>)>,
) -> Result<Arc<UiInput>, UiError> {
    if !std::ptr::eq(visual.layout, &*paint.layout) {
        return Err(UiError::Invalid("paint belongs to another layout"));
    }
    if let Some((previous, input)) = accepted
        && Arc::ptr_eq(previous, paint)
        && drawn(visual).count() == input.widgets.len()
        && drawn(visual)
            .zip(&input.widgets)
            .all(|((_, node), widget)| {
                node.focused == widget.focused
                    && node.pressed == widget.pressed
                    && edit(&node) == widget.edit
            })
    {
        return Ok(Arc::clone(input));
    }
    let mut texts = paint.texts.iter();
    let mut widgets = Vec::with_capacity(paint.texts.len());
    for (position, node) in drawn(visual) {
        let layout = &visual.layout.nodes[position];
        let text = texts
            .next()
            .filter(|t| t.id == node.widget.id && layout.id == node.widget.id)
            .ok_or(UiError::Invalid("paint text order"))?;
        let role = match &node.widget.node.kind {
            UiKind::Label { .. } => UiRole::Label,
            UiKind::Button { .. } => UiRole::Button,
            UiKind::TextInput { .. } => UiRole::TextInput,
            UiKind::Root | UiKind::Container => unreachable!("text-bearing widget"),
        };
        let edit = edit(&node);
        if (role == UiRole::TextInput) != edit.is_some()
            || node
                .text
                .is_some_and(|buffer| buffer.display_text() != text.layout.source())
        {
            return Err(UiError::Invalid("text input state and paint disagree"));
        }
        widgets.push(UiWidget {
            id: node.widget.id,
            role,
            disabled: layout.disabled,
            focused: node.focused,
            pressed: node.pressed,
            bounds: ltrb(layout.bounds),
            visible: ltrb(layout.hit),
            origin: text.origin,
            clip: ltrb(text.clip),
            layout: Arc::clone(&text.layout),
            glyphs: Arc::clone(&text.glyphs),
            edit,
        });
    }
    if texts.next().is_some() {
        return Err(UiError::Invalid("paint text order"));
    }
    Ok(Arc::new(UiInput {
        viewport: visual.layout.viewport,
        atlas: Arc::clone(&paint.atlas),
        widgets,
    }))
}

fn widgets(view: &UiView<'_>) -> Vec<Widget> {
    view.nodes()
        .map(|node| Widget {
            id: node.id,
            incarnation: node.incarnation,
            parent: node.parent,
            node: Arc::clone(node.node),
        })
        .collect()
}
