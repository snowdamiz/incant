use crate::{UiAlign, UiDirection, UiJustify, UiLength, UiStyle};
use taffy::prelude::TaffyAuto;
use taffy::{
    AlignItems, Dimension, FlexDirection, FlexWrap, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Point, Rect, Size, Style,
};

pub fn convert(value: &UiStyle) -> Style {
    Style {
        size: Size {
            width: dimension(value.size[0]),
            height: dimension(value.size[1]),
        },
        min_size: Size {
            width: LengthPercentageAuto::length(value.min_size[0]),
            height: LengthPercentageAuto::length(value.min_size[1]),
        },
        max_size: Size {
            width: value.max_size[0]
                .map_or(LengthPercentageAuto::AUTO, LengthPercentageAuto::length),
            height: value.max_size[1]
                .map_or(LengthPercentageAuto::AUTO, LengthPercentageAuto::length),
        },
        padding: Rect {
            left: LengthPercentage::length(value.padding[0]),
            top: LengthPercentage::length(value.padding[1]),
            right: LengthPercentage::length(value.padding[2]),
            bottom: LengthPercentage::length(value.padding[3]),
        },
        gap: Size {
            width: LengthPercentage::length(value.gap[0]),
            height: LengthPercentage::length(value.gap[1]),
        },
        flex_direction: match value.direction {
            UiDirection::Row => FlexDirection::Row,
            UiDirection::RowReverse => FlexDirection::RowReverse,
            UiDirection::Column => FlexDirection::Column,
            UiDirection::ColumnReverse => FlexDirection::ColumnReverse,
        },
        flex_wrap: if value.wrap {
            FlexWrap::Wrap
        } else {
            FlexWrap::NoWrap
        },
        align_items: Some(match value.align {
            UiAlign::Start => AlignItems::START,
            UiAlign::End => AlignItems::END,
            UiAlign::Center => AlignItems::CENTER,
            UiAlign::Stretch => AlignItems::STRETCH,
        }),
        align_content: Some(taffy::AlignContent::START),
        justify_content: Some(match value.justify {
            UiJustify::Start => JustifyContent::START,
            UiJustify::End => JustifyContent::END,
            UiJustify::Center => JustifyContent::CENTER,
            UiJustify::SpaceBetween => JustifyContent::SPACE_BETWEEN,
            UiJustify::SpaceAround => JustifyContent::SPACE_AROUND,
            UiJustify::SpaceEvenly => JustifyContent::SPACE_EVENLY,
        }),
        flex_grow: value.grow,
        flex_shrink: value.shrink,
        flex_basis: dimension(value.basis),
        // Clipping is explicit in the published layout; never create scrolling
        // or reserve invisible scrollbar width through library defaults.
        overflow: Point {
            x: taffy::Overflow::Clip,
            y: taffy::Overflow::Clip,
        },
        ..Style::default()
    }
}
fn dimension(value: UiLength) -> Dimension {
    match value {
        UiLength::Auto => Dimension::AUTO,
        UiLength::Pixels(n) => Dimension::length(n),
        UiLength::Fraction(n) => Dimension::percent(n),
    }
}
