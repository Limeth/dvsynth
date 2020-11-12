use super::*;
use crate::node::{ChannelRef, ChannelType, ChannelTypeTrait, NodeBehaviourMessage, NodeConfiguration};
use crate::util::{PathBuilderExt, RectangleExt, Segments};
use crate::{style, util, ChannelDirection, ChannelIdentifier, Connection, Message, NodeMessage};
use iced::widget::Space;
use iced_graphics::canvas::{Fill, FillRule, Frame, LineCap, LineJoin, Path, Stroke};
use iced_graphics::{self, Backend, Defaults, Primitive};
use iced_native::layout::{Layout, Limits, Node};
use iced_native::mouse::{self, Button as MouseButton, Event as MouseEvent};
use iced_native::widget::{Container, Widget};
use iced_native::{
    self, Align, Background, Clipboard, Column, Event, Hasher, Length, Point, Rectangle, Row, Size, Text,
};
use iced_native::{overlay, Element};
use iced_native::{Color, Vector};
use lyon_geom::QuadraticBezierSegment;
use ordered_float::OrderedFloat;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::hash::Hash;
use vek::Vec2;

impl<'a> ChannelRef<'a> {
    pub fn render<M: 'a + Clone, R: 'a + WidgetRenderer>(&self) -> Element<'a, M, R> {
        Text::new(self.title.to_string()).size(style::consts::TEXT_SIZE_REGULAR).into()
    }
}

#[derive(Default)]
pub struct NodeElementState {
    // pub text_input_state: iced::widget::text_input::State,
// pub text_input_value: String,
}

pub struct NodeElementBuilder<'a, M: 'a + Clone, R: 'a + WidgetRenderer> {
    index: NodeIndex,
    state: &'a mut NodeElementState,
    node_behaviour_element: Option<Element<'a, M, R>>,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    input_channels: Vec<ChannelRef<'a>>,
    output_channels: Vec<ChannelRef<'a>>,
    __marker: std::marker::PhantomData<&'a (M, R)>,
}

/// A widget specifically made to be used as the child of the [`FloatingPanes`] widget alongside the
/// custom behaviour [`FloatingPanesBehaviour`] to function as a node graph editor.
pub struct NodeElement<'a, M: 'a + Clone, R: 'a + WidgetRenderer> {
    index: NodeIndex,
    input_channels: Vec<ChannelRef<'a>>,
    output_channels: Vec<ChannelRef<'a>>,
    // state: &'a mut NodeElementState,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    element_tree: Element<'a, M, R>,
}

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> NodeElementBuilder<'a, M, R> {
    pub fn new(index: NodeIndex, state: &'a mut NodeElementState) -> Self {
        Self {
            index,
            state,
            node_behaviour_element: None,
            width: Length::Shrink,
            height: Length::Shrink,
            extents: [u32::MAX, u32::MAX].into(),
            input_channels: Default::default(),
            output_channels: Default::default(),
            __marker: Default::default(),
        }
    }

    pub fn node_behaviour_element(
        mut self,
        node_behaviour_element: impl Into<Option<Element<'a, M, R>>>,
    ) -> Self
    {
        self.node_behaviour_element = node_behaviour_element.into();
        self
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

    pub fn push_input_channel(mut self, channel: impl Into<ChannelRef<'a>>) -> Self {
        self.input_channels.push(channel.into());
        self
    }

    pub fn push_output_channel(mut self, channel: impl Into<ChannelRef<'a>>) -> Self {
        self.output_channels.push(channel.into());
        self
    }

    pub fn build(
        self, /*, text_input_callback: impl (Fn(NodeIndex, String) -> M) + 'static*/
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
                    {
                        let mut column = Column::new().spacing(style::consts::SPACING_VERTICAL);

                        if let Some(node_behaviour_element) = self.node_behaviour_element {
                            column = column.push(node_behaviour_element);
                        } else {
                            // insert space to keep layout indices consistent
                            column = column.push(Space::new(Length::Shrink, Length::Shrink));
                        }

                        column = column.push(
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
                        );

                        column
                    },
                    style::consts::SPACING,
                )
                .into()
            },
            input_channels: self.input_channels,
            output_channels: self.output_channels,
        }
    }
}

impl<'a, M: 'a + Clone, R: 'a + WidgetRenderer> NodeElement<'a, M, R> {
    pub fn builder(index: NodeIndex, state: &'a mut NodeElementState) -> NodeElementBuilder<'a, M, R> {
        NodeElementBuilder::new(index, state)
    }

    fn get_connection_point(layout: ChannelLayout, direction: ChannelDirection) -> Vec2<f32> {
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

    fn is_channel_selected(
        channel_layout: ChannelLayout,
        channel_direction: ChannelDirection,
        cursor_position: Vec2<f32>,
    ) -> bool
    {
        const GRAB_RADIUS: f32 = 6.0;

        let mut bounds = channel_layout.bounds();
        bounds = match channel_direction {
            ChannelDirection::Out => bounds.grow(
                style::consts::SPACING_HORIZONTAL as f32,
                style::consts::SPACING_VERTICAL as f32 * 0.5,
                0.0,
                style::consts::SPACING_VERTICAL as f32 * 0.5,
            ),
            ChannelDirection::In => bounds.grow(
                0.0,
                style::consts::SPACING_VERTICAL as f32 * 0.5,
                style::consts::SPACING_HORIZONTAL as f32,
                style::consts::SPACING_VERTICAL as f32 * 0.5,
            ),
        };

        if bounds.contains(cursor_position.into_array().into()) {
            return true;
        }

        let connection_point = Self::get_connection_point(channel_layout, channel_direction);
        let distance_squared = cursor_position.distance_squared(connection_point);

        distance_squared <= GRAB_RADIUS * GRAB_RADIUS
    }

    pub fn get_layout_index_from_channel(
        panes: &FloatingPanes<'a, M, R, FloatingPanesBehaviour<M>>,
        channel: ChannelIdentifier,
    ) -> Option<usize>
    {
        panes.get_layout_index_from_pane_index(&channel.node_index)
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
        self.element_tree.draw(renderer, defaults, layout, cursor_position)
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

pub struct FloatingPanesBehaviour<M> {
    pub on_channel_disconnect: fn(ChannelIdentifier) -> M,
    pub on_connection_create: fn(Connection) -> M,
    pub connections: Vec<Connection>,
}

impl<M: Clone> FloatingPanesBehaviour<M> {
    /// A reflexive function to check whether two channels can be connected
    fn can_connect<'a, R: 'a + WidgetRenderer>(
        panes: &FloatingPanes<'a, M, R, Self>,
        from: ChannelIdentifier,
        to: ChannelIdentifier,
    ) -> bool
    {
        // TODO: Add borrow checking and type checking
        from.node_index != to.node_index && from.channel_direction != to.channel_direction && {
            let pane_from = panes.children.get(&from.node_index).unwrap();
            let pane_to = panes.children.get(&to.node_index).unwrap();
            let channel_from = pane_from
                .behaviour_data
                .node_configuration
                .channel(from.channel_direction, from.channel_index);
            let channel_to =
                pane_to.behaviour_data.node_configuration.channel(to.channel_direction, to.channel_index);

            ChannelType::is_abi_compatible(&channel_from.ty, &channel_to.ty)
        }
        // Allow, but disconnect previous connection
        // && self.connections.iter().any(|connection| connection.to() == to)
    }

    fn is_connected(&self, channel: ChannelIdentifier) -> bool {
        self.connections.iter().any(|connection| connection.channel(channel.channel_direction) == channel)
    }
}

impl<'a, M: Clone + 'a, R: 'a + WidgetRenderer> floating_panes::FloatingPanesBehaviour<'a, M, R>
    for FloatingPanesBehaviour<M>
{
    type FloatingPaneIndex = NodeIndex;
    type FloatingPaneBehaviourData = FloatingPaneBehaviourData;
    type FloatingPaneBehaviourState = FloatingPaneBehaviourState;
    type FloatingPanesBehaviourState = FloatingPanesBehaviourState;

    fn draw_panes(
        panes: &FloatingPanes<'a, M, R, Self>,
        renderer: &mut R,
        defaults: &<R as iced_native::Renderer>::Defaults,
        layout: FloatingPanesLayout<'_>,
        cursor_position: Point,
    ) -> <R as iced_native::Renderer>::Output
    {
        <R as WidgetRenderer>::draw_panes(renderer, panes, defaults, layout, cursor_position)
    }

    fn hash_panes(panes: &FloatingPanes<'a, M, R, Self>, state: &mut Hasher) {}

    fn on_event(
        panes: &mut FloatingPanes<'a, M, R, Self>,
        event: Event,
        layout: FloatingPanesLayout<'_>,
        cursor_position: Point,
        messages: &mut Vec<M>,
        renderer: &R,
        clipboard: Option<&dyn Clipboard>,
    ) -> bool
    {
        match event {
            Event::Mouse(MouseEvent::CursorMoved { x, y }) => {
                let cursor_position = Vec2::new(x, y);

                panes.behaviour_state.highlight = None;

                // Highlight channel, if possible
                for (pane_layout, node_index) in layout.panes().zip(panes.children.keys().copied()) {
                    let pane_bounding_box =
                        pane_layout.bounds().grow_symmetrical(style::consts::SPACING_HORIZONTAL as f32, 0.0);

                    if !pane_bounding_box.contains(cursor_position.into_array().into()) {
                        continue;
                    }

                    let inputs_layout = pane_layout.content().channels_with_direction(ChannelDirection::In);
                    let outputs_layout = pane_layout.content().channels_with_direction(ChannelDirection::Out);
                    let channels = inputs_layout
                        .channels()
                        .enumerate()
                        .map(|(index, layout)| (index, layout, ChannelDirection::In))
                        .chain(
                            outputs_layout
                                .channels()
                                .enumerate()
                                .map(|(index, layout)| (index, layout, ChannelDirection::Out)),
                        );

                    let highlighted_channel = channels
                        .filter(|(channel_index, channel_layout, channel_direction)| {
                            // If a new connection is being formed, make sure the target channel
                            // can be connected to.
                            if let Some(selected_channel) = panes.behaviour_state.selected_channel.as_ref() {
                                let channel = ChannelIdentifier {
                                    node_index,
                                    channel_index: *channel_index,
                                    channel_direction: *channel_direction,
                                };

                                if !FloatingPanesBehaviour::can_connect(panes, *selected_channel, channel) {
                                    return false;
                                }
                            }

                            NodeElement::<M, R>::is_channel_selected(
                                channel_layout.clone(),
                                channel_direction.clone(),
                                cursor_position,
                            )
                        })
                        .next();

                    if let Some((channel_index, _, channel_direction)) = highlighted_channel {
                        let channel = ChannelIdentifier {
                            node_index: node_index.clone(),
                            channel_index,
                            channel_direction,
                        };
                        panes.behaviour_state.highlight = Some(Highlight::Channel(channel));
                    }
                }

                // Otherwise, highlight a connection, if one is not being created
                if panes.behaviour_state.highlight.is_none()
                    && panes.behaviour_state.selected_channel.is_none()
                {
                    const MAX_CONNECTION_HIGHLIGHT_DISTANCE: f32 = 6.0;

                    let closest_connection = panes
                        .behaviour
                        .connections
                        .iter()
                        .map(|connection| {
                            let layout_from = layout
                                .panes()
                                .nth(
                                    NodeElement::<M, R>::get_layout_index_from_channel(
                                        panes,
                                        connection.from(),
                                    )
                                    .unwrap(),
                                )
                                .unwrap();
                            let layout_to = layout
                                .panes()
                                .nth(
                                    NodeElement::<M, R>::get_layout_index_from_channel(
                                        panes,
                                        connection.to(),
                                    )
                                    .unwrap(),
                                )
                                .unwrap();
                            let layout_outputs =
                                layout_from.content().channels_with_direction(ChannelDirection::Out);
                            let layout_inputs =
                                layout_to.content().channels_with_direction(ChannelDirection::In);
                            let layout_output = layout_outputs.channel(connection.from().channel_index);
                            let layout_input = layout_inputs.channel(connection.to().channel_index);
                            let connection_curve =
                                ConnectionCurve::from_channel_layouts::<M, R>(layout_output, layout_input);
                            let connection_distance_squared = connection_curve
                                .get_distance_squared(cursor_position, MAX_CONNECTION_HIGHLIGHT_DISTANCE);

                            (connection, connection_distance_squared)
                        })
                        .filter_map(|(connection, distance_squared)| {
                            distance_squared.map(move |distance_squared| (connection, distance_squared))
                        })
                        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                        .map(|(connection, _)| connection);

                    if let Some(closest_connection) = closest_connection {
                        panes.behaviour_state.highlight =
                            Some(Highlight::Connection(closest_connection.clone()));
                    }
                }
            }
            Event::Mouse(MouseEvent::ButtonPressed(MouseButton::Left)) => {
                if let Some(highlight) = panes.behaviour_state.highlight.take() {
                    match highlight {
                        Highlight::Connection(highlighted_connection) => {
                            panes.behaviour_state.selected_channel = Some(highlighted_connection.from());
                            messages
                                .push((panes.behaviour.on_channel_disconnect)(highlighted_connection.to()));
                        }
                        Highlight::Channel(channel @ ChannelIdentifier { channel_direction, .. }) => {
                            let disconnect = match channel_direction {
                                ChannelDirection::In => panes.behaviour.is_connected(channel),
                                ChannelDirection::Out => false,
                            };

                            // Is connection pending?
                            if let Some(selected_channel) = panes.behaviour_state.selected_channel.clone() {
                                if FloatingPanesBehaviour::can_connect(panes, selected_channel, channel) {
                                    if disconnect {
                                        messages.push((panes.behaviour.on_channel_disconnect)(channel));
                                    }

                                    let channels = match selected_channel.channel_direction {
                                        ChannelDirection::In => [channel, selected_channel],
                                        ChannelDirection::Out => [selected_channel, channel],
                                    };

                                    messages.push((panes.behaviour.on_connection_create)(
                                        Connection::try_from_identifiers(channels).unwrap(),
                                    ));
                                    panes.behaviour_state.selected_channel = None;
                                }
                            } else {
                                if disconnect {
                                    let connection = panes
                                        .behaviour
                                        .connections
                                        .iter()
                                        .find(|connection| connection.contains_channel(channel));
                                    if let Some(connection) = connection {
                                        let other_channel =
                                            connection.channel(channel.channel_direction.inverse());
                                        panes.behaviour_state.selected_channel = Some(other_channel);

                                        messages.push((panes.behaviour.on_channel_disconnect)(channel));
                                    }
                                } else {
                                    panes.behaviour_state.selected_channel = Some(channel);
                                }
                            }
                        }
                    }

                    // Properly update the highlight
                    Self::on_event(
                        panes,
                        Event::Mouse(MouseEvent::CursorMoved {
                            x: panes.state.cursor_position.x,
                            y: panes.state.cursor_position.y,
                        }),
                        layout,
                        cursor_position,
                        messages,
                        renderer,
                        clipboard,
                    );
                    return true;
                }

                panes.behaviour_state.selected_channel = None;
            }
            _ => (),
        }

        false
    }
}

pub struct FloatingPaneBehaviourData {
    pub node_configuration: NodeConfiguration,
}

#[derive(Default)]
pub struct FloatingPaneBehaviourState {}

#[derive(Debug)]
pub enum Highlight {
    Channel(ChannelIdentifier),
    Connection(Connection),
}

#[derive(Default)]
pub struct FloatingPanesBehaviourState {
    pub selected_channel: Option<ChannelIdentifier>,
    pub highlight: Option<Highlight>,
}

/// Good practice: Rendering is made to be generic over the backend using this trait, which
/// is to be implemented on the specific `Renderer`.
pub trait WidgetRenderer:
    margin::WidgetRenderer
    + floating_panes::WidgetRenderer
    + iced_native::Renderer
    + iced_native::text::Renderer
    + iced_native::column::Renderer
    + iced_native::widget::container::Renderer
    + iced_native::widget::text_input::Renderer
    + Sized
{
    fn draw_panes<M: Clone>(
        &mut self,
        panes: &FloatingPanes<'_, M, Self, FloatingPanesBehaviour<M>>,
        defaults: &Self::Defaults,
        layout: FloatingPanesLayout<'_>,
        cursor_position: Point,
    ) -> Self::Output;
}

impl<B> WidgetRenderer for iced_graphics::Renderer<B>
where B: Backend + iced_graphics::backend::Text
{
    fn draw_panes<M: Clone>(
        &mut self,
        panes: &FloatingPanes<'_, M, Self, FloatingPanesBehaviour<M>>,
        defaults: &Self::Defaults,
        layout: FloatingPanesLayout<'_>,
        cursor_position: Point,
    ) -> Self::Output
    {
        let mut mouse_interaction = mouse::Interaction::default();
        let mut primitives = Vec::new();

        primitives.extend(panes.children.iter().zip(layout.panes()).map(|((child_index, child), layout)| {
            let (primitive, new_mouse_interaction) =
                child.element_tree.draw(self, defaults, layout.into(), cursor_position);

            if new_mouse_interaction > mouse_interaction {
                mouse_interaction = new_mouse_interaction;
            }

            primitive
        }));

        // Draw connections
        let mut frame = Frame::new(layout.bounds().size());

        // Draw existing connections
        for connection in &panes.behaviour.connections {
            let layout_from = layout.pane_with_index(
                NodeElement::<M, Self>::get_layout_index_from_channel(panes, connection.from()).unwrap(),
            );
            let layout_to = layout.pane_with_index(
                NodeElement::<M, Self>::get_layout_index_from_channel(panes, connection.to()).unwrap(),
            );

            let layout_outputs = layout_from.content().channels_with_direction(ChannelDirection::Out);
            let layout_inputs = layout_to.content().channels_with_direction(ChannelDirection::In);
            let layout_output = layout_outputs.channel(connection.from().channel_index);
            let layout_input = layout_inputs.channel(connection.to().channel_index);

            // primitives.push(
            //     draw_bounds(layout_output, Color::from_rgb(1.0, 0.0, 0.0))
            // );
            // primitives.push(
            //     draw_bounds(layout_input, Color::from_rgb(0.0, 0.0, 1.0))
            // );

            let from = NodeElement::<M, Self>::get_connection_point(layout_output, ChannelDirection::Out);
            let to = NodeElement::<M, Self>::get_connection_point(layout_input, ChannelDirection::In);

            let highlighted = if let Some(highlight) = panes.behaviour_state.highlight.as_ref() {
                match highlight {
                    Highlight::Connection(highlighted_connection) => connection == highlighted_connection,
                    Highlight::Channel(highlighted_channel) => {
                        connection.contains_channel(highlighted_channel.clone())
                    }
                }
            } else {
                false
            };
            let stroke = if highlighted {
                Stroke {
                    color: Color::from_rgba(0.5, 1.0, 0.0, 1.0),
                    width: 3.0,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Round,
                }
            } else {
                Stroke {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
                    width: 1.5,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Round,
                }
            };

            // primitives.push(draw_point(from.into_array().into(), Color::from_rgb(1.0, 0.0, 0.0)));
            // primitives.push(draw_point(to.into_array().into(), Color::from_rgb(0.0, 0.0, 1.0)));

            ConnectionCurve { from, to }.draw(&mut frame, stroke);

            // Code to visualize finding the closest point to the curve
            // {
            //     // TODO: When checking whether the cursor is above a curve, first construct
            //     // a bounding convex polygon or AABB that encloses the curve + the max distance
            //     // at which the selection should be active
            //     let segments = util::get_connection_curve(from, to);
            //     let projection = segments.project_point(panes.state.cursor_position);
            //     let projection = segments.sample(projection.t);
            //     let radius = projection.distance(panes.state.cursor_position);

            //     frame.stroke(
            //         &Path::circle(panes.state.cursor_position.into_array().into(), radius),
            //         Stroke { color: Color::WHITE, width: 1.0, ..Default::default() },
            //     );
            //     primitives
            //         .push(util::draw_point(projection.into_array().into(), Color::from_rgb(1.0, 0.0, 1.0)));
            // }
        }

        // Draw pending connection
        if let Some(selected_channel) = panes.behaviour_state.selected_channel.as_ref() {
            let pane_layout = layout
                .panes()
                .nth(NodeElement::<M, Self>::get_layout_index_from_channel(panes, *selected_channel).unwrap())
                .unwrap();
            let layout_channels =
                pane_layout.content().channels_with_direction(selected_channel.channel_direction);
            let layout_channel = layout_channels.channel(selected_channel.channel_index);

            let connected_position = NodeElement::<M, Self>::get_connection_point(
                layout_channel,
                selected_channel.channel_direction,
            );
            let target_position = if let Some(Highlight::Channel(highlighted_channel)) =
                panes.behaviour_state.highlight.as_ref()
            {
                let child_layout = layout
                    .panes()
                    .nth(
                        NodeElement::<M, Self>::get_layout_index_from_channel(panes, *highlighted_channel)
                            .unwrap(),
                    )
                    .unwrap();
                let layout_channels =
                    child_layout.content().channels_with_direction(highlighted_channel.channel_direction);
                let layout_channel = layout_channels.channel(highlighted_channel.channel_index);
                NodeElement::<M, Self>::get_connection_point(
                    layout_channel,
                    highlighted_channel.channel_direction,
                )
            } else {
                panes.state.cursor_position
            };

            let (from, to) = match selected_channel.channel_direction {
                ChannelDirection::In => (target_position, connected_position),
                ChannelDirection::Out => (connected_position, target_position),
            };

            let stroke = Stroke {
                color: Color::from_rgba(1.0, 0.6, 0.0, 1.0),
                width: 3.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Round,
            };

            ConnectionCurve { from, to }.draw(&mut frame, stroke);
        }

        // Draw connection points
        {
            const CONNECTION_POINT_RADIUS: f32 = 3.0;
            const CONNECTION_POINT_RADIUS_HIGHLIGHTED: f32 = 4.5;

            for (pane_layout, node_index) in layout.panes().zip(panes.children.keys().copied()) {
                let inputs_layout = pane_layout.content().channels_with_direction(ChannelDirection::In);
                let outputs_layout = pane_layout.content().channels_with_direction(ChannelDirection::Out);
                let channel_layouts = inputs_layout
                    .channels()
                    .enumerate()
                    .map(|(index, layout)| (index, layout, ChannelDirection::In))
                    .chain(
                        outputs_layout
                            .channels()
                            .enumerate()
                            .map(|(index, layout)| (index, layout, ChannelDirection::Out)),
                    );

                for (channel_index, channel_layout, channel_direction) in channel_layouts {
                    let position =
                        NodeElement::<M, Self>::get_connection_point(channel_layout, channel_direction);
                    let channel = ChannelIdentifier { node_index, channel_index, channel_direction };
                    let highlighted = if let Some(Highlight::Channel(highlighted_channel)) =
                        panes.behaviour_state.highlight.as_ref()
                    {
                        *highlighted_channel == channel
                    } else {
                        false
                    };
                    let (radius, color) = if highlighted {
                        (CONNECTION_POINT_RADIUS_HIGHLIGHTED, Color::from_rgb(0.5, 1.0, 0.0))
                    } else {
                        (CONNECTION_POINT_RADIUS, Color::WHITE)
                    };

                    primitives.push(util::draw_point(position, color, radius));
                }
            }
        }

        primitives.push(frame.into_geometry().into_primitive());

        (Primitive::Group { primitives }, mouse_interaction)
    }
}

pub struct ConnectionCurve {
    pub from: Vec2<f32>,
    pub to: Vec2<f32>,
}

impl ConnectionCurve {
    fn from_channel_layouts<M: Clone, R: WidgetRenderer>(
        output: ChannelLayout,
        input: ChannelLayout,
    ) -> Self
    {
        let from = NodeElement::<M, R>::get_connection_point(output, ChannelDirection::Out);
        let to = NodeElement::<M, R>::get_connection_point(input, ChannelDirection::In);
        Self { from, to }
    }

    fn draw(&self, frame: &mut Frame, stroke: Stroke) {
        let segments = util::get_connection_curve(self.from, self.to);
        let path = Path::new(|builder| {
            builder.move_to(self.from.into_array().into());
            segments.build_segments(builder);

            // Debug control points
            // for segment in &segments.segments {
            //     let points = [&segment.from, &segment.ctrl, &segment.to];
            //     for i in 0..points.len() {
            //         let from = points[i];
            //         let to = points[(i + 1) % points.len()];
            //         builder.move_to(from.to_array().into());
            //         builder.line_to(to.to_array().into());
            //     }
            // }

            // Debug bounding box
            // let aabb = Self::bounds_from_curve(&segments).grow_uniform(6.0);
            // builder.line_segment_loop(&aabb.vertices()[..]);
        });

        frame.stroke(&path, stroke);
    }

    fn bounds_from_curve(segments: &Segments<QuadraticBezierSegment<f32>>) -> Rectangle {
        let min = Vec2::<f32>::new(
            [segments[0].from.x, segments[0].ctrl.x, segments[1].ctrl.x, segments[1].to.x]
                .iter()
                .copied()
                .fold_first(util::partial_min)
                .unwrap(),
            util::partial_min(segments[0].from.y, segments[1].to.y),
        );
        let max = Vec2::<f32>::new(
            [segments[0].from.x, segments[0].ctrl.x, segments[1].ctrl.x, segments[1].to.x]
                .iter()
                .copied()
                .fold_first(util::partial_max)
                .unwrap(),
            util::partial_max(segments[0].from.y, segments[1].to.y),
        );

        Rectangle::from_min_max(min, max)
    }

    fn bounds(&self) -> Rectangle {
        Self::bounds_from_curve(&util::get_connection_curve(self.from, self.to))
    }

    fn get_distance_squared(&self, point: Vec2<f32>, max_distance: f32) -> Option<f32> {
        let segments = util::get_connection_curve(self.from, self.to);

        // Before performing expensive computations, check whether the point is within the bounding
        // box.
        let bounds = Self::bounds_from_curve(&segments).grow_uniform(max_distance);

        if bounds.contains(point.into_array().into()) {
            let projection = segments.project_point(point);
            let projection = segments.sample(projection.t);
            let connection_distance_squared = projection.distance_squared(point);

            if connection_distance_squared <= max_distance * max_distance {
                return Some(connection_distance_squared);
            }
        }

        None
    }
}

typed_layout! {
    type_name: Channels,
    traverse: [
        {
            parent_type_name: FloatingPaneContent,
            fn_name: channels_with_direction,
            fn_args: [channel_direction: ChannelDirection],
            fn: |parent: Layout<'a>, channel_direction: ChannelDirection| {
                parent
                    .children()
                    .nth(1)
                    .unwrap()
                    .children()
                    .nth(1)
                    .unwrap()
                    .children()
                    .nth(1)
                    .unwrap()
                    .children()
                    .nth(match channel_direction {
                        ChannelDirection::In => 0,
                        ChannelDirection::Out => 1,
                    })
                    .unwrap()
            },
        },
    ],
}

typed_layout! {
    type_name: Channel,
    traverse: [
        {
            parent_type_name: Channels,
            fn_name: channel,
            fn_args: [channel_index: usize],
            fn: |parent: Layout<'a>, channel_index: usize| {
                parent.children().nth(channel_index).unwrap()
            },
        },
    ],
    children_of: {
        parent_type_name: Channels,
        fn_name: channels,
    },
}
