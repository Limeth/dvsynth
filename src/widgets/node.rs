use std::hash::Hash;
use iced_native::{self, Align, Size, Length, Point, Hasher, Event, Clipboard, Row, Column, Text};
use iced_native::{overlay, Element};
use iced_native::mouse::{self, Event as MouseEvent, Button as MouseButton};
use iced_native::widget::{Widget, Container};
use iced_native::layout::{Node, Layout, Limits};
use iced_graphics::{self, Backend, Defaults, Primitive};
use vek::Vec2;
use ordered_float::OrderedFloat;
use super::*;
use crate::style;

pub struct ChannelSlice<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
}

impl<'a> ChannelSlice<'a> {
    pub fn render<M: 'a, R: 'a + WidgetRenderer>(&self) -> Element<'a, M, R> {
        Text::new(self.title.to_string())
            .size(14) // FIXME: hardcoding bad >:(
            .into()
    }
}

#[derive(Default)]
pub struct NodeElementState;

pub struct NodeElementBuilder<'a, M: 'a, R: 'a + WidgetRenderer> {
    state: &'a mut NodeElementState,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    input_channels: Vec<ChannelSlice<'a>>,
    output_channels: Vec<ChannelSlice<'a>>,
    __marker: std::marker::PhantomData<&'a (M, R)>,
}

pub struct NodeElement<'a, M: 'a, R: 'a + WidgetRenderer> {
    state: &'a mut NodeElementState,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    element_tree: Element<'a, M, R>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> NodeElementBuilder<'a, M, R> {
    pub fn new(state: &'a mut NodeElementState) -> Self {
        Self {
            state,
            width: Length::Shrink,
            height: Length::Shrink,
            extents: [u32::MAX, u32::MAX].into(),
            input_channels: Default::default(),
            output_channels: Default::default(),
            __marker: Default::default(),
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn max_width(mut self, max_width: u32) -> Self {
        self.extents[0] = max_width;
        self
    }

    pub fn max_height(mut self, max_height: u32) -> Self {
        self.extents[1] = max_height;
        self
    }

    pub fn extents(mut self, extents: Vec2<u32>) -> Self {
        self.extents = extents;
        self
    }

    pub fn push_input_channel(mut self, channel: impl Into<ChannelSlice<'a>>) -> Self {
        self.input_channels.push(channel.into());
        self
    }

    pub fn push_output_channel(mut self, channel: impl Into<ChannelSlice<'a>>) -> Self {
        self.output_channels.push(channel.into());
        self
    }

    pub fn build(self) -> NodeElement<'a, M, R> {
        NodeElement {
            state: self.state,
            width: self.width,
            height: self.height,
            extents: self.extents,
            element_tree: {
                Margin::new(
                    Row::new()
                        .spacing(style::consts::SPACING_HORIZONTAL)
                        .push({ // input channels
                            let mut column = Column::new()
                                .spacing(style::consts::SPACING_VERTICAL)
                                .align_items(Align::Start);

                            for input_channel in &self.input_channels {
                                column = column.push(input_channel.render());
                            }

                            column
                        })
                        .push({ // output channels
                            let mut column = Column::new()
                                .spacing(style::consts::SPACING_VERTICAL)
                                .align_items(Align::End);

                            for output_channel in &self.output_channels {
                                column = column.push(output_channel.render());
                            }

                            column
                        }),
                    style::consts::SPACING,
                ).into()
            },
        }
    }
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> NodeElement<'a, M, R> {
    pub fn builder(
        state: &'a mut NodeElementState,
    ) -> NodeElementBuilder<'a, M, R> {
        NodeElementBuilder::new(state)
    }
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> Widget<M, R> for NodeElement<'a, M, R> {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &R, limits: &Limits) -> Node {
        // let limits = limits
        //     .max_width(self.extents[0])
        //     .max_height(self.extents[1])
        //     .width(self.width)
        //     .height(self.height);
        self.element_tree.layout(renderer, limits)
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &<R as iced_native::Renderer>::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> <R as iced_native::Renderer>::Output {
        <R as WidgetRenderer>::draw(renderer, defaults, layout, cursor_position, self)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.element_tree.hash_layout(state);
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<M>,
        renderer: &R,
        clipboard: Option<&dyn Clipboard>
    ) {
        self.element_tree.on_event(event, layout, cursor_position, messages, renderer, clipboard);
    }

    fn overlay(
        &mut self,
        layout: Layout<'_>
    ) -> Option<overlay::Element<'_, M, R>> {
        self.element_tree.overlay(layout)
    }
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> From<NodeElement<'a, M, R>> for Element<'a, M, R> {
    fn from(other: NodeElement<'a, M, R>) -> Self {
        Element::new(other)
    }
}

/// Good practice: Rendering is made to be generic over the backend using this trait, which
/// is to be implemented on the specific `Renderer`.
pub trait WidgetRenderer:
        margin::WidgetRenderer
      + iced_native::Renderer
      + iced_native::text::Renderer
      + iced_native::column::Renderer
      + iced_native::widget::container::Renderer
      + Sized {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &NodeElement<'_, M, Self>,
    ) -> Self::Output;
}

impl<B> WidgetRenderer for iced_graphics::Renderer<B>
where
    B: Backend + iced_graphics::backend::Text,
{
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &NodeElement<'_, M, Self>,
    ) -> Self::Output {
        element.element_tree.draw(self, defaults, layout, cursor_position)
    }
}
