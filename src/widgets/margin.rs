use iced::Rectangle;
use iced_graphics::Backend;
use iced_runtime::event::Status;
use iced_runtime::layout::{Layout, Limits, Node};
use iced_runtime::widget::Widget;
use iced_runtime::widget::{Column, Row, Space};
use iced_runtime::{overlay, Element};
use iced_runtime::{Clipboard, Event, Hasher, Length, Point};

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

pub struct Margin<'a, M, R: WidgetRenderer + 'a> {
    child: Element<'a, M, R>,
}

impl<'a, M: 'a, R: WidgetRenderer + 'a> Margin<'a, M, R> {
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

impl<'a, M: 'a, T: Theme, R: WidgetRenderer + 'a> Widget<M, T, R> for Margin<'a, M, R> {
    fn width(&self) -> Length {
        self.child.width()
    }

    fn height(&self) -> Length {
        self.child.height()
    }

    fn layout(&self, renderer: &R, limits: &Limits) -> Node {
        self.child.layout(renderer, limits)
    }

    fn draw(
        &self,
        state: &iced_runtime::widget::Tree,
        renderer: &mut R,
        theme: &<R as iced_runtime::Renderer>::Theme,
        style: &iced_runtime::renderer::Style,
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

    fn overlay(&mut self, layout: Layout<'_>) -> Option<overlay::Element<'_, M, R>> {
        self.child.overlay(layout)
    }
}

impl<'a, M: 'a, R: WidgetRenderer + 'a> From<Margin<'a, M, R>> for Element<'a, M, R> {
    fn from(other: Margin<'a, M, R>) -> Self {
        Element::new(other)
    }
}

pub trait WidgetRenderer:
    iced_runtime::Renderer
    // + iced_runtime::space::Renderer
    // + iced_runtime::column::Renderer
    // + iced_runtime::row::Renderer
    + Sized
{
}

impl WidgetRenderer for R
where
    R: iced_core::Renderer + iced_core::text::Renderer + iced_core::
    iced_graphics::Renderer<B>: iced_runtime::Renderer,
    B: Backend, // + iced_graphics::backend::Text,
{
}
