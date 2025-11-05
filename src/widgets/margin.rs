use iced::event::Status;
use iced::mouse::Cursor;
use iced::widget::{Column, Row, Space};
use iced::{Element, Event, Length, Point, Rectangle, Vector, overlay};
use iced_core::layout::{Limits, Node};
use iced_core::widget::Tree;
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

pub struct Margin<'a, M, T, R: WidgetRenderer + 'a> {
    child: Element<'a, M, T, R>,
}

impl<'a, M: 'a, T: 'a, R: WidgetRenderer + 'a> Margin<'a, M, T, R> {
    pub fn new(element: impl Into<Element<'a, M, T, R>>, spacing: Spacing) -> Self {
        if spacing == Spacing::default() {
            return Self { child: element.into() };
        }

        Self {
            child: Column::<M, T, R>::new()
                .push(Space::with_height(Length::Fixed(spacing.up as f32)))
                .push(
                    Row::new()
                        .push(Space::with_width(Length::Fixed(spacing.left as f32)))
                        .push(element)
                        .push(Space::with_width(Length::Fixed(spacing.right as f32))),
                )
                .push(Space::with_height(Length::Fixed(spacing.down as f32)))
                .into(),
        }
    }
}

impl<'a, M: 'a, T: 'a, R: WidgetRenderer + 'a> Widget<M, T, R> for Margin<'a, M, T, R> {
    fn size(&self) -> iced::Size<iced::Length> {
        self.child.size()
    }

    fn layout(&self, state: &mut Tree, renderer: &R, limits: &Limits) -> Node {
        self.child.layout(renderer, limits)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.child.as_widget().draw(state, renderer, theme, style, layout, cursor, viewport)
    }

    // fn hash_layout(&self, state: &mut Hasher) {
    //     self.child.hash_layout(state)
    // }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        self.child.on_event(event, layout, cursor, renderer, clipboard, shell)
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &R,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>> {
        self.child.overlay(layout, renderer)
    }
}

impl<'a, M: 'a, T, R: WidgetRenderer + 'a> From<Margin<'a, M, T, R>> for Element<'a, M, T, R> {
    fn from(other: Margin<'a, M, T, R>) -> Self {
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
