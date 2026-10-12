//! Bounded retained screen-space layout and widget state, using only cooked
//! declarations and explicitly supplied packaged fonts. No authoring document,
//! filesystem, renderer, OS input method or provider dependency.
#![forbid(unsafe_code)]
mod focus;
mod fonts;
pub use fonts::{MAX_UI_FONT_CHAINS, UiFonts};
mod layout;
mod paint;
pub use paint::{MAX_UI_GLYPHS, MAX_UI_LINES, UiPaint, UiTextPaint};
mod reconcile;
mod routing;
pub use routing::{RoutedUi, UiRouter};
mod style;
pub use focus::{
    Direction, FocusToken, MAX_UI_EVENTS, MAX_UI_PACKET_BYTES, PointerToken, UiAction, UiEvent,
    UiPending, UiRuntime,
};
pub use layout::{
    LayoutCandidate, LayoutEngine, LayoutFrame, MAX_MEASURE_CALLS, MeasureSpace, Rect, TextMeasure,
    Widget, WidgetLayout,
};

pub use incant_text::AtlasConfig;
pub use incant_types::{
    MAX_UI_DEPTH, MAX_UI_EXTENT, MAX_UI_NODES, StableId, UiAlign, UiDirection, UiJustify, UiKind,
    UiLength, UiNode, UiStyle, UiText,
};

#[derive(Debug, thiserror::Error)]
pub enum UiError {
    #[error(transparent)]
    Input(#[from] incant_input::InputError),
    #[error("invalid UI declaration: {0}")]
    Invalid(&'static str),
    #[error("UI resource limit: {0}")]
    Limit(&'static str),
    #[error("UI input belongs to an expired owner, focus or layout")]
    Stale,
    #[error("UI layout failed")]
    Layout,
    #[error(transparent)]
    Text(#[from] incant_text::TextError),
}
