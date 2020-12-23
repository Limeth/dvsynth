use iced::Rectangle;
use iced_graphics::Backend;
use iced_native::event::Status;
use iced_native::layout::{Layout, Limits, Node};
use iced_native::widget::Widget;
use iced_native::widget::{Column, Row, Space};
use iced_native::{overlay, Element};
use iced_native::{Clipboard, Event, Hasher, Length, Point};

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

impl<'a, M: 'a, R: WidgetRenderer + 'a> Widget<M, R> for Margin<'a, M, R> {
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
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) -> R::Output {
        self.child.draw(renderer, defaults, layout, cursor_position, viewport)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        self.child.hash_layout(state)
    }

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
    iced_native::Renderer
    + iced_native::space::Renderer
    + iced_native::column::Renderer
    + iced_native::row::Renderer
    + Sized
{
}

impl<B> WidgetRenderer for iced_graphics::Renderer<B> where B: Backend + iced_graphics::backend::Text {}
