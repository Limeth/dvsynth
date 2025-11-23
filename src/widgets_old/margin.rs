use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::{Column, Row, Space};
use iced::{Element, Event, Length, Rectangle, Vector, overlay};
use iced_core::layout::{Limits, Node};
use iced_core::widget::Tree;
use iced_core::widget::tree::Tag;
use iced_core::{Clipboard, Layout, Renderer, Shell, Widget};

#[derive(Default, PartialEq, Eq, Clone)]
pub struct Spacing {
    // TODO: Consider using Length instead
    pub right: u16,
    pub up: u16,
    pub left: u16,
    pub down: u16,
}

impl Spacing {
    pub const fn from_axes(horizontal: u16, vertical: u16) -> Self {
        Self { right: horizontal, up: vertical, left: horizontal, down: vertical }
    }

    pub const fn uniform(spacing: u16) -> Self {
        Self { right: spacing, up: spacing, left: spacing, down: spacing }
    }
}

pub fn margin<'a, M: 'a, T: 'a, R: 'a + iced_core::Renderer>(
    element: impl Into<Element<'a, M, T, R>>,
    spacing: Spacing,
) -> Element<'a, M, T, R> {
    Column::<M, T, R>::new()
        .push(Space::with_height(Length::Fixed(spacing.up as f32)))
        .push(
            Row::new()
                .push(Space::with_width(Length::Fixed(spacing.left as f32)))
                .push(element)
                .push(Space::with_width(Length::Fixed(spacing.right as f32))),
        )
        .push(Space::with_height(Length::Fixed(spacing.down as f32)))
        .into()
}
