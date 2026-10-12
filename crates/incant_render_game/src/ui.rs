//! CPU screen-space ownership attached to the exact native world. Preparation
//! never changes the accepted widget frame, even if spatial consumption refuses.
use incant_game_runtime::RoutedUiInput;
use incant_input::InputEvent;
use incant_render_native::NativeRenderInput;
use incant_runtime::{NativeWorld, UiView, WorldIdentity};
use incant_ui::{RoutedUi, UiError, UiEvent, UiFonts, UiPending, UiRouter, UiRuntime, Widget};
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
    ) -> Result<Self, UiError> {
        let view = world
            .ui_view()
            .map_err(|_| UiError::Invalid("unusable native world"))?;
        let mut runtime = UiRuntime::new(widgets(&view))?;
        let mut fonts = UiFonts::new(input.font_sources().cloned().collect(), runtime.widgets())?;
        let first = runtime.prepare(&[], settings.viewport, &mut fonts)?;
        let paint =
            incant_ui::UiPaint::prepare(&runtime, &first, &mut fonts, settings.atlas, None)?;
        runtime.commit(first)?;
        Ok(Self {
            paint,
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
        Ok(Prepared {
            paint,
            viewport,
            generation: view.generation(),
            fonts,
            state,
        })
    }
    pub fn commit(&mut self, prepared: Prepared) -> Result<(), UiError> {
        match prepared.state {
            State::Input(pending) => self.router.commit(&mut self.runtime, *pending)?,
            State::Refresh(pending) => self.runtime.commit(*pending)?,
        }
        if let Some(fonts) = prepared.fonts {
            self.fonts = fonts;
        }
        self.paint = prepared.paint;
        self.viewport = prepared.viewport;
        self.generation = prepared.generation;
        Ok(())
    }
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
