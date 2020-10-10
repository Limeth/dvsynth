use iced_native::{Length, Point, Hasher, Event, Clipboard};
use iced_native::{mouse, overlay, Element};
use iced_native::widget::Widget;
// use iced_native::renderer::Renderer;
use iced_native::layout::{Layout, Limits, Node};
use iced_native::widget::{Column, Row, Space};
use iced_graphics::{Backend, Defaults, Primitive, Renderer};
// use iced_wgpu::{Backend, Defaults, Primitive, Renderer};

pub struct Spacing {
    pub right: u16,
    pub up: u16,
    pub left: u16,
    pub down: u16,
}

pub struct Margin<'a, M, B: Backend + 'a> {
    child: Column<'a, M, Renderer<B>>,
}

impl<'a, M: 'a, B: Backend + 'a> Margin<'a, M, B> {
    pub fn new(element: Element<'a, M, Renderer<B>>, spacing: Spacing) -> Self {
        Self {
            child: Column::new()
                .push(Space::with_height(Length::Units(spacing.up)))
                .push(
                    Row::new()
                        .push(Space::with_width(Length::Units(spacing.left)))
                        .push(element)
                        .push(Space::with_width(Length::Units(spacing.right)))
                )
                .push(Space::with_height(Length::Units(spacing.down)))
        }
    }
}

impl<'a, M: 'a, B: Backend + 'a> Widget<M, Renderer<B>> for Margin<'a, M, B> {
    fn width(&self) -> Length {
        Widget::width(&self.child)
    }

    fn height(&self) -> Length {
        Widget::height(&self.child)
    }

    fn layout(&self, renderer: &Renderer<B>, limits: &Limits) -> Node {
        self.child.layout(renderer, limits)
    }

    fn draw(
        &self,
        renderer: &mut Renderer<B>,
        defaults: &Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> (Primitive, mouse::Interaction) {
        self.child.draw(renderer, defaults, layout, cursor_position)
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
        renderer: &Renderer<B>,
        clipboard: Option<&dyn Clipboard>
    ) {
        self.child.on_event(
            event, layout, cursor_position, messages, renderer, clipboard,
        )
    }

    fn overlay(
        &mut self, 
        layout: Layout<'_>
    ) -> Option<overlay::Element<'_, M, Renderer<B>>> {
        self.child.overlay(layout)
    }
}

impl<'a, M: 'a, B: Backend + 'a> From<Margin<'a, M, B>> for Element<'a, M, Renderer<B>> {
    fn from(other: Margin<'a, M, B>) -> Self {
        Element::new(other)
    }
}
