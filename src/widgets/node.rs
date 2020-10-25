use std::hash::Hash;
use iced_native::{self, Align, Size, Length, Point, Hasher, Event, Clipboard, Row, Column, Text};
use iced_native::{overlay, Element};
use iced_native::mouse::{self, Event as MouseEvent, Button as MouseButton};
use iced_native::widget::{Widget, Container};
use iced_native::layout::{Node, Layout, Limits};
use iced_graphics::{self, Backend, Defaults, Primitive};
use iced_graphics::canvas::{Fill, FillRule, Frame, LineCap, Path, Stroke};
use iced_native::{Color, Vector};
use vek::Vec2;
use ordered_float::OrderedFloat;
use petgraph::graph::NodeIndex;
use super::*;
use crate::{style, Message, NodeMessage, ChannelDirection};

pub struct ChannelSlice<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
}

impl<'a> ChannelSlice<'a> {
    pub fn render<M: 'a + Clone, R: 'a + WidgetRenderer>(&self) -> Element<'a, M, R> {
        Text::new(self.title.to_string())
            .size(14) // FIXME: hardcoding bad >:(
            .into()
    }
}

#[derive(Default)]
pub struct NodeElementState {
    // pub text_input_state: iced::widget::text_input::State,
    // pub text_input_value: String,
}

pub struct NodeElementBuilder<'a, M: 'a + Clone, R: 'a + WidgetRenderer> {
    index: NodeIndex<u32>,
    state: &'a mut NodeElementState,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    input_channels: Vec<ChannelSlice<'a>>,
    output_channels: Vec<ChannelSlice<'a>>,
    __marker: std::marker::PhantomData<&'a (M, R)>,
}

pub struct NodeElement<'a, M: 'a + Clone, R: 'a + WidgetRenderer> {
    index: NodeIndex<u32>,
    // state: &'a mut NodeElementState,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    element_tree: Element<'a, M, R>,
}

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> NodeElementBuilder<'a, M, R> {
    pub fn new(index: NodeIndex<u32>, state: &'a mut NodeElementState) -> Self {
        Self {
            index,
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

    pub fn build(self/*, text_input_callback: impl (Fn(NodeIndex<u32>, String) -> M) + 'static*/) -> NodeElement<'a, M, R> {
        NodeElement {
            index: self.index,
            // state: self.state,
            width: self.width,
            height: self.height,
            extents: self.extents,
            element_tree: { // Element { Margin { Row [ Column [ .. ], Column [ .. ] ] } }
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

                            // let text_input = iced_native::widget::TextInput::<M, R>::new(
                            //     &mut self.state.text_input_state,
                            //     "",
                            //     &self.state.text_input_value,
                            //     {
                            //         let index = self.index;
                            //         move |new_value| {
                            //             (text_input_callback)(index, new_value)
                            //         }
                            //     },
                            // );

                            // column = column.push(text_input);

                            column
                        }),
                    style::consts::SPACING,
                ).into()
            },
        }
    }
}

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> NodeElement<'a, M, R> {
    pub fn builder(
        index: NodeIndex<u32>,
        state: &'a mut NodeElementState,
    ) -> NodeElementBuilder<'a, M, R> {
        NodeElementBuilder::new(index, state)
    }
}

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> Widget<M, R> for NodeElement<'a, M, R> {
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

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> From<NodeElement<'a, M, R>> for Element<'a, M, R> {
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
      + iced_native::widget::text_input::Renderer
      + Sized {
    fn draw<M: Clone>(
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
    fn draw<M: Clone>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &NodeElement<'_, M, Self>,
    ) -> Self::Output {
        const CONNECTION_POINT_RADIUS: f32 = 3.0;
        const CONNECTION_POINT_CENTER: f32 = CONNECTION_POINT_RADIUS + 1.0; // extra pixel for anti aliasing
        const FRAME_SIZE: f32 = CONNECTION_POINT_CENTER * 2.0;

        let mut primitives = Vec::new();
        let mut frame = Frame::new([FRAME_SIZE, FRAME_SIZE].into());
        let path = Path::new(|builder| {
            builder.circle([CONNECTION_POINT_CENTER, CONNECTION_POINT_CENTER].into(), CONNECTION_POINT_RADIUS);
        });

        frame.fill(&path, Fill {
            color: Color::WHITE,
            rule: FillRule::NonZero,
        });

        let primitive_connection_point = frame.into_geometry().into_primitive();
        let (primitive, interaction) = element.element_tree.draw(self, defaults, layout, cursor_position);

        primitives.push(primitive);

        // Element { Margin { Row [ Column [ .. ], Column [ .. ] ] } }
        let row_layout = layout.children().nth(1).unwrap(); // Margin Column
        let row_layout = row_layout.children().nth(1).unwrap(); // Margin Row
        let inputs_layout = row_layout.children().nth(0).unwrap();
        let outputs_layout = row_layout.children().nth(1).unwrap();
        let channel_layouts = inputs_layout.children().map(|layout| (layout, ChannelDirection::In))
            .chain(outputs_layout.children().map(|layout| (layout, ChannelDirection::Out)));

        for (channel_layout, channel_direction) in channel_layouts {
            let position = channel_layout.position();
            let mut translation: Vec2<f32> = Vec2::new(0.0, position.y)
                - Vec2::new(CONNECTION_POINT_CENTER, CONNECTION_POINT_CENTER)
                // + Vec2::new(0.0 * crate::style::consts::SPACING_HORIZONTAL as f32, 0.0)
                + Vec2::new(layout.position().x, 0.0)
                + Vec2::new(0.0, channel_layout.bounds().height / 2.0);

            if channel_direction == ChannelDirection::Out {
                translation += Vec2::new(layout.bounds().width, 0.0)
            }

            primitives.push(Primitive::Translate {
                translation: Vector::new(translation.x, translation.y),
                content: Box::new(primitive_connection_point.clone()),
            });
        }

        (
            Primitive::Group {
                primitives,
            },
            interaction,
        )
    }
}
