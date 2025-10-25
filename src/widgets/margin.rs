use iced::event::Status;
use iced::widget::{Column, Row, Space};
use iced::{Element, Event, Length, Point, Rectangle};
use iced_core::layout::{Limits, Node};
use iced_core::widget::Tree;
use iced_core::{Clipboard, Layout, Renderer, Widget};

#[derive(Default, PartialEq, Eq, Clone)]
pub struct Spacing {
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

pub struct Margin<'a, M, T, R: WidgetRenderer + 'a> {
    child: Element<'a, M, T, R>,
}

impl<'a, M: 'a, T, R: WidgetRenderer + 'a> Margin<'a, M, T, R> {
    pub fn new(element: impl Into<Element<'a, M, R>>, spacing: Spacing) -> Self {
        if spacing == Spacing::default() {
            return Self { child: element.into() };
        }

        Self {
            child: Column::new()
                .push(Space::with_height(Length::Units(spacing.up)))
                .push(
                    Row::new()
                        .push(Space::with_width(Length::Units(spacing.left)))
                        .push(element)
                        .push(Space::with_width(Length::Units(spacing.right))),
                )
                .push(Space::with_height(Length::Units(spacing.down)))
                .into(),
        }
    }
}

impl<'a, M: 'a, T, R: WidgetRenderer + 'a> Widget<M, T, R> for Margin<'a, M, T, R> {
    fn size(&self) -> iced::Size<iced::Length> {
        self.child.size()
    }

    fn layout(&self, renderer: &R, limits: &Limits) -> Node {
        self.child.layout(renderer, limits)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &R::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        self.child.as_widget().draw(state, renderer, theme, style, layout, cursor_position, viewport)
    }

    // fn hash_layout(&self, state: &mut Hasher) {
    //     self.child.hash_layout(state)
    // }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<M>,
        renderer: &R,
        clipboard: Option<&dyn Clipboard>,
    ) -> Status {
        self.child.on_event(event, layout, cursor_position, messages, renderer, clipboard)
    }

    fn overlay(&mut self, layout: Layout<'_>) -> Option<Element<'_, M, R>> {
        self.child.overlay(layout)
    }
}

impl<'a, M: 'a, T, R: WidgetRenderer + 'a> From<Margin<'a, M, T, R>> for Element<'a, M, R> {
    fn from(other: Margin<'a, M, R>) -> Self {
        Element::new(other)
    }
}

// TODO: Is this necessary?
pub trait WidgetRenderer:
    Renderer
    // + iced_runtime::space::Renderer
    // + iced_runtime::column::Renderer
    // + iced_runtime::row::Renderer
    + Sized
{
}

// impl WidgetRenderer for R
// where
//     R: iced_core::Renderer + iced_core::text::Renderer + iced_core::
//     iced_graphics::Renderer<B>: iced_runtime::Renderer,
//     B: Backend, // + iced_graphics::backend::Text,
// {
// }
