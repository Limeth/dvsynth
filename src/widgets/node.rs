use super::*;
use crate::{style, util, ChannelDirection, ChannelIdentifier, Connection, Message, NodeMessage};
use iced_graphics::canvas::{Fill, FillRule, Frame, LineCap, LineJoin, Path, Stroke};
use iced_graphics::{self, Backend, Defaults, Primitive};
use iced_native::layout::{Layout, Limits, Node};
use iced_native::mouse::{self, Button as MouseButton, Event as MouseEvent};
use iced_native::widget::{Container, Widget};
use iced_native::{
    self, Align, Background, Clipboard, Column, Event, Hasher, Length, Point, Row, Size, Text,
};
use iced_native::{overlay, Element};
use iced_native::{Color, Vector};
use ordered_float::OrderedFloat;
use petgraph::graph::NodeIndex;
use std::hash::Hash;
use vek::Vec2;

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

    pub fn build(
        self, /*, text_input_callback: impl (Fn(NodeIndex<u32>, String) -> M) + 'static*/
    ) -> NodeElement<'a, M, R> {
        NodeElement {
            index: self.index,
            // state: self.state,
            width: self.width,
            height: self.height,
            extents: self.extents,
            element_tree: {
                // Element { Margin { Row [ Column [ .. ], Column [ .. ] ] } }
                Margin::new(
                    Row::new()
                        .spacing(style::consts::SPACING_HORIZONTAL)
                        .push({
                            // input channels
                            let mut column = Column::new()
                                .spacing(style::consts::SPACING_VERTICAL)
                                .align_items(Align::Start);

                            for input_channel in &self.input_channels {
                                column = column.push(input_channel.render());
                            }

                            column
                        })
                        .push({
                            // output channels
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
                )
                .into()
            },
        }
    }
}

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> NodeElement<'a, M, R> {
    pub fn builder(index: NodeIndex<u32>, state: &'a mut NodeElementState) -> NodeElementBuilder<'a, M, R> {
        NodeElementBuilder::new(index, state)
    }
}

fn get_connection_point(layout: Layout, direction: ChannelDirection) -> Vec2<f32> {
    let field_position: Vec2<f32> = Into::<[f32; 2]>::into(layout.position()).into();
    let field_size: Vec2<f32> = Into::<[f32; 2]>::into(layout.bounds().size()).into();

    match direction {
        ChannelDirection::In => {
            field_position + field_size * Vec2::new(0.0, 0.5)
                - Vec2::new(style::consts::SPACING_HORIZONTAL as f32, 0.0)
        }
        ChannelDirection::Out => {
            field_position
                + field_size * Vec2::new(1.0, 0.5)
                + Vec2::new(style::consts::SPACING_HORIZONTAL as f32, 0.0)
        }
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
    ) -> <R as iced_native::Renderer>::Output
    {
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
        clipboard: Option<&dyn Clipboard>,
    )
    {
        self.element_tree.on_event(event, layout, cursor_position, messages, renderer, clipboard);
    }

    fn overlay(&mut self, layout: Layout<'_>) -> Option<overlay::Element<'_, M, R>> {
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
    + Sized
{
    fn draw<M: Clone>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &NodeElement<'_, M, Self>,
    ) -> Self::Output;
}

impl<B> WidgetRenderer for iced_graphics::Renderer<B>
where B: Backend + iced_graphics::backend::Text
{
    fn draw<M: Clone>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &NodeElement<'_, M, Self>,
    ) -> Self::Output
    {
        const CONNECTION_POINT_RADIUS: f32 = 3.0;
        const CONNECTION_POINT_CENTER: f32 = CONNECTION_POINT_RADIUS + 1.0; // extra pixel for anti aliasing
        const FRAME_SIZE: f32 = CONNECTION_POINT_CENTER * 2.0;

        let mut primitives = Vec::new();
        let mut frame = Frame::new([FRAME_SIZE, FRAME_SIZE].into());
        let path = Path::new(|builder| {
            builder
                .circle([CONNECTION_POINT_CENTER, CONNECTION_POINT_CENTER].into(), CONNECTION_POINT_RADIUS);
        });

        frame.fill(&path, Fill { color: Color::WHITE, rule: FillRule::NonZero });

        let primitive_connection_point = frame.into_geometry().into_primitive();
        let (primitive, interaction) = element.element_tree.draw(self, defaults, layout, cursor_position);

        primitives.push(primitive);

        // Element { Margin { Row [ Column [ .. ], Column [ .. ] ] } }
        let row_layout = layout.children().nth(1).unwrap(); // Margin Column
        let row_layout = row_layout.children().nth(1).unwrap(); // Margin Row
        let inputs_layout = row_layout.children().nth(0).unwrap();
        let outputs_layout = row_layout.children().nth(1).unwrap();
        let channel_layouts = inputs_layout
            .children()
            .map(|layout| (layout, ChannelDirection::In))
            .chain(outputs_layout.children().map(|layout| (layout, ChannelDirection::Out)));

        for (channel_layout, channel_direction) in channel_layouts {
            let translation = get_connection_point(channel_layout, channel_direction)
                - Vec2::new(CONNECTION_POINT_CENTER, CONNECTION_POINT_CENTER);

            primitives.push(Primitive::Translate {
                translation: Vector::new(translation.x, translation.y),
                content: Box::new(primitive_connection_point.clone()),
            });
        }

        (Primitive::Group { primitives }, interaction)
    }
}

fn draw_point(position: Vector, color: Color) -> Primitive {
    const CONNECTION_POINT_RADIUS: f32 = 2.0;
    const CONNECTION_POINT_CENTER: f32 = CONNECTION_POINT_RADIUS + 1.0; // extra pixel for anti aliasing
    const FRAME_SIZE: f32 = CONNECTION_POINT_CENTER * 2.0;

    let mut frame = Frame::new([FRAME_SIZE, FRAME_SIZE].into());
    let path = Path::new(|builder| {
        builder.circle([CONNECTION_POINT_CENTER, CONNECTION_POINT_CENTER].into(), CONNECTION_POINT_RADIUS);
    });

    frame.fill(&path, Fill { color, rule: FillRule::NonZero });

    Primitive::Translate {
        translation: position - Vector::new(CONNECTION_POINT_CENTER, CONNECTION_POINT_CENTER),
        content: Box::new(frame.into_geometry().into_primitive()),
    }
}

fn draw_bounds(layout: Layout<'_>, color: Color) -> Primitive {
    // let layout_position = Vector::new(layout.position().x, layout.position().y);
    // let layout_size = Vector::new(layout.bounds().size().width, layout.bounds().size().height);

    // Primitive::Group {
    //     primitives: vec![
    //         draw_point(
    //             layout_position,
    //             color,
    //         ),
    //         draw_point(
    //             layout_position + layout_size,
    //             color,
    //         ),
    //     ],
    // }
    Primitive::Quad {
        bounds: layout.bounds(),
        background: Background::Color(Color::TRANSPARENT),
        border_radius: 0,
        border_width: 1,
        border_color: color,
    }
}

impl<'a, B: 'a + Backend + iced_graphics::backend::Text>
    FloatingPaneContent<'a, Message, iced_graphics::Renderer<B>>
    for NodeElement<'a, Message, iced_graphics::Renderer<B>>
{
    type FloatingPaneIndex = NodeIndex<u32>;
    type FloatingPaneContentState = FloatingPaneContentState;
    type FloatingPanesContentState = FloatingPanesContentState;

    fn create_element(self) -> Element<'a, Message, iced_graphics::Renderer<B>> {
        self.into()
    }

    fn draw_content(
        panes: &FloatingPanes<'a, Message, iced_graphics::Renderer<B>, Self>,
        renderer: &mut iced_graphics::Renderer<B>,
        defaults: &<iced_graphics::Renderer<B> as iced_native::Renderer>::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> <iced_graphics::Renderer<B> as iced_native::Renderer>::Output
    {
        let mut mouse_interaction = mouse::Interaction::default();
        let mut primitives = Vec::new();

        primitives.extend(panes.children.iter().zip(layout.children()).map(
            |((child_index, child), layout)| {
                let (primitive, new_mouse_interaction) =
                    child.element_tree.draw(renderer, defaults, layout, cursor_position);

                if new_mouse_interaction > mouse_interaction {
                    mouse_interaction = new_mouse_interaction;
                }

                primitive
            },
        ));

        fn draw_connection(frame: &mut Frame, from: Vec2<f32>, to: Vec2<f32>, stroke: Stroke) {
            const CONTROL_POINT_DISTANCE_SLOPE: f32 = 1.0 / 3.0;
            const CONTROL_POINT_DISTANCE_ABS_SOFTNESS: f32 = 32.0;
            const CONTROL_POINT_DISTANCE_MAX_SHARPNESS: f32 = 0.01;
            const CONTROL_POINT_DISTANCE_MAX: f32 = 64.0;

            let mid = (from + to) / 2.0;
            let control_point_distance = (to - from)
                .map(|coord_delta| {
                    util::softminabs(
                        CONTROL_POINT_DISTANCE_ABS_SOFTNESS,
                        CONTROL_POINT_DISTANCE_MAX_SHARPNESS,
                        CONTROL_POINT_DISTANCE_MAX,
                        coord_delta * CONTROL_POINT_DISTANCE_SLOPE,
                    )
                })
                .sum();

            let control_from = from + Vec2::new(control_point_distance, 0.0);
            let control_to = to - Vec2::new(control_point_distance, 0.0);
            let path = Path::new(|builder| {
                builder.move_to(from.into_array().into());
                // builder.line_to(to.into_array().into());
                builder.quadratic_curve_to(control_from.into_array().into(), mid.into_array().into());
                builder.quadratic_curve_to(control_to.into_array().into(), to.into_array().into());
            });

            frame.stroke(&path, stroke);
        }

        // Draw connections
        let mut frame = Frame::new(layout.bounds().size());

        // Draw existing connections
        for connection in &panes.content_state.connections {
            // let pane_from = &panes.children[&connection.from.node_index];
            // let pane_to = &panes.children[&connection.to.node_index];
            // FIXME: Replace with O(1)
            let (layout_from, (index_from, pane_from)) = layout
                .children()
                .zip(&panes.children)
                .find(|(child_layout, (child_index, child))| **child_index == connection.from().node_index)
                .unwrap();
            let (layout_to, (index_to, pane_to)) = layout
                .children()
                .zip(&panes.children)
                .find(|(child_layout, (child_index, child))| **child_index == connection.to().node_index)
                .unwrap();

            let layout_outputs = layout_from;
            let layout_outputs = layout_outputs.children().nth(0).unwrap();
            let layout_outputs = layout_outputs.children().nth(1).unwrap();
            let layout_outputs = layout_outputs.children().nth(0).unwrap();
            let layout_outputs = layout_outputs.children().nth(1).unwrap();
            let layout_outputs = layout_outputs.children().nth(1).unwrap();
            let layout_outputs = layout_outputs.children().nth(1).unwrap();

            let layout_inputs = layout_to;
            let layout_inputs = layout_inputs.children().nth(0).unwrap();
            let layout_inputs = layout_inputs.children().nth(1).unwrap();
            let layout_inputs = layout_inputs.children().nth(0).unwrap();
            let layout_inputs = layout_inputs.children().nth(1).unwrap();
            let layout_inputs = layout_inputs.children().nth(1).unwrap();
            let layout_inputs = layout_inputs.children().nth(0).unwrap();

            let layout_output = layout_outputs.children().nth(connection.from().channel_index).unwrap();
            let layout_input = layout_inputs.children().nth(connection.to().channel_index).unwrap();

            // primitives.push(
            //     draw_bounds(layout_output, Color::from_rgb(1.0, 0.0, 0.0))
            // );
            // primitives.push(
            //     draw_bounds(layout_input, Color::from_rgb(0.0, 0.0, 1.0))
            // );

            let from = get_connection_point(layout_output, ChannelDirection::Out);
            let to = get_connection_point(layout_input, ChannelDirection::In);

            // primitives.push(draw_point(from.into_array().into(), Color::from_rgb(1.0, 0.0, 0.0)));
            // primitives.push(draw_point(to.into_array().into(), Color::from_rgb(0.0, 0.0, 1.0)));

            draw_connection(
                &mut frame,
                from,
                to,
                Stroke {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.7),
                    width: 1.5,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Round,
                },
            );
        }

        // Draw pending connection
        if let Some(selected_channel) = panes.content_state.selected_channel.as_ref() {
            let pane_index = panes
                .children
                .keys()
                .enumerate()
                .find(|(_, key)| **key == selected_channel.node_index)
                .unwrap()
                .0;

            let pane_layout = layout.children().nth(pane_index).unwrap();

            let layout_channels = pane_layout;
            let layout_channels = layout_channels.children().nth(0).unwrap();
            let layout_channels = layout_channels.children().nth(1).unwrap();
            let layout_channels = layout_channels.children().nth(0).unwrap();
            let layout_channels = layout_channels.children().nth(1).unwrap();
            let layout_channels = layout_channels.children().nth(1).unwrap();
            let layout_channels = layout_channels
                .children()
                .nth({
                    match selected_channel.channel_direction {
                        ChannelDirection::In => 0,
                        ChannelDirection::Out => 1,
                    }
                })
                .unwrap();
            let layout_channel = layout_channels.children().nth(selected_channel.channel_index).unwrap();

            let connection_position =
                get_connection_point(layout_channel, selected_channel.channel_direction);
            let cursor_position = panes.state.cursor_position;

            let (from, to) = match selected_channel.channel_direction {
                ChannelDirection::In => (cursor_position, connection_position),
                ChannelDirection::Out => (connection_position, cursor_position),
            };

            draw_connection(
                &mut frame,
                from,
                to,
                Stroke {
                    color: Color::from_rgba(1.0, 0.6, 0.0, 1.0),
                    width: 3.0,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Round,
                },
            );
        }

        primitives.push(frame.into_geometry().into_primitive());

        (Primitive::Group { primitives }, mouse_interaction)
    }

    fn hash_content(
        panes: &FloatingPanes<'a, Message, iced_graphics::Renderer<B>, Self>,
        state: &mut Hasher,
    )
    {
        // TODO
    }

    fn on_event(
        panes: &mut FloatingPanes<'a, Message, iced_graphics::Renderer<B>, Self>,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<Message>,
        renderer: &iced_graphics::Renderer<B>,
        clipboard: Option<&dyn Clipboard>,
    ) -> bool
    {
        match event {
            Event::Mouse(MouseEvent::ButtonPressed(MouseButton::Left)) => {
                for (layout, node_index) in layout.children().zip(panes.children.keys()) {
                    let row_layout = layout;
                    let row_layout = row_layout.children().nth(0).unwrap();
                    let row_layout = row_layout.children().nth(1).unwrap();
                    let row_layout = row_layout.children().nth(0).unwrap();
                    let row_layout = row_layout.children().nth(1).unwrap(); // Margin Column
                    let row_layout = row_layout.children().nth(1).unwrap(); // Margin Row
                    let inputs_layout = row_layout.children().nth(0).unwrap();
                    let outputs_layout = row_layout.children().nth(1).unwrap();
                    let channels = inputs_layout
                        .children()
                        .enumerate()
                        .map(|(index, layout)| (index, layout, ChannelDirection::In))
                        .chain(
                            outputs_layout
                                .children()
                                .enumerate()
                                .map(|(index, layout)| (index, layout, ChannelDirection::Out)),
                        );

                    for (channel_index, channel_layout, channel_direction) in channels {
                        let grab_radius = channel_layout.bounds().size().height / 2.0;
                        let connection_point = get_connection_point(channel_layout, channel_direction);
                        let distance_squared = panes.state.cursor_position.distance_squared(connection_point);

                        if distance_squared <= grab_radius * grab_radius {
                            let channel = ChannelIdentifier {
                                node_index: *node_index,
                                channel_direction,
                                channel_index,
                            };

                            let disconnect = match channel_direction {
                                ChannelDirection::In => panes.content_state.is_connected(channel),
                                ChannelDirection::Out => false,
                            };

                            // Is connection pending?
                            if let Some(selected_channel) = panes.content_state.selected_channel.clone() {
                                if panes.content_state.can_connect(selected_channel, channel) {
                                    if disconnect {
                                        messages.push(Message::DisconnectChannel { channel });
                                    }

                                    let channels = match selected_channel.channel_direction {
                                        ChannelDirection::In => [channel, selected_channel],
                                        ChannelDirection::Out => [selected_channel, channel],
                                    };

                                    messages.push(Message::InsertConnection {
                                        connection: Connection::try_from_identifiers(channels).unwrap(),
                                    });
                                    panes.content_state.selected_channel = None;
                                }
                            } else {
                                if disconnect {
                                    let connection = panes
                                        .content_state
                                        .connections
                                        .iter()
                                        .find(|connection| connection.contains_channel(channel));
                                    if let Some(connection) = connection {
                                        let other_channel =
                                            connection.channel(channel.channel_direction.inverse());
                                        panes.content_state.selected_channel = Some(other_channel);

                                        messages.push(Message::DisconnectChannel { channel });
                                    }
                                } else {
                                    panes.content_state.selected_channel = Some(channel);
                                }
                            }

                            return true;
                        }
                    }
                }

                panes.content_state.selected_channel = None;
            }
            _ => (),
        }

        false
    }
}

#[derive(Default)]
pub struct FloatingPaneContentState {
    pub node_index: Option<NodeIndex<u32>>,
}

#[derive(Default)]
pub struct FloatingPanesContentState {
    pub connections: Vec<Connection>,
    pub selected_channel: Option<ChannelIdentifier>,
}

impl FloatingPanesContentState {
    fn can_connect(&self, from: ChannelIdentifier, to: ChannelIdentifier) -> bool {
        // TODO: Add borrow checking and type checking
        from.node_index != to.node_index && from.channel_direction != to.channel_direction
        // Allow, but disconnect previous connection
        // && self.connections.iter().any(|connection| connection.to() == to)
    }

    fn is_connected(&self, channel: ChannelIdentifier) -> bool {
        self.connections.iter().any(|connection| connection.channel(channel.channel_direction) == channel)
    }
}
