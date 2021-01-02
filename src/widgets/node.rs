use super::*;
use crate::graph::GraphValidationErrors;
use crate::node::{ChannelPassBy, ChannelRef, ConnectionPassBy, NodeConfiguration, TypeEnum, TypeExt};
use crate::util::{RectangleExt, Segments, StrokeType};
use crate::{style, util, ChannelDirection, ChannelIdentifier, Connection};
use iced::widget::canvas::{Fill, FillRule};
use iced::widget::Space;
use iced_graphics::canvas::{Frame, LineCap, LineJoin, Path, Stroke};
use iced_graphics::{self, Backend, Primitive};
use iced_native::event::Status;
use iced_native::layout::{Layout, Limits, Node};
use iced_native::mouse::{self, Button as MouseButton, Event as MouseEvent};
use iced_native::widget::Widget;
use iced_native::Color;
use iced_native::{self, Align, Clipboard, Column, Event, Hasher, Length, Point, Rectangle, Row, Text};
use iced_native::{overlay, Element};
use lyon_geom::QuadraticBezierSegment;
use petgraph::graph::NodeIndex;
use std::hash::Hash;
use vek::Vec2;

impl<'a> ChannelRef<'a> {
    pub fn render<M: 'a + Clone, R: 'a + WidgetRenderer>(&self) -> Element<'a, M, R> {
        Text::new(self.title.to_string()).size(style::consts::TEXT_SIZE_REGULAR).into()
    }
}

#[derive(Default)]
pub struct NodeElementState {
    __marker: (), // prevent direct construction for future proofing
}

pub struct NodeElementBuilder<'a, M: 'a + Clone, R: 'a + WidgetRenderer> {
    index: NodeIndex,
    state: &'a mut NodeElementState,
    node_behaviour_element: Option<Element<'a, M, R>>,
    width: Length,
    height: Length,
    input_channels: Vec<ChannelRef<'a>>,
    output_channels: Vec<ChannelRef<'a>>,
    __marker: std::marker::PhantomData<&'a (M, R)>,
}

/// A widget specifically made to be used as the child of the [`FloatingPanes`] widget alongside the
/// custom behaviour [`FloatingPanesBehaviour`] to function as a node graph editor.
#[allow(dead_code)]
pub struct NodeElement<'a, M: 'a + Clone, R: 'a + WidgetRenderer> {
    index: NodeIndex,
    state: &'a mut NodeElementState,
    width: Length,
    height: Length,
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
            input_channels: Default::default(),
            output_channels: Default::default(),
            __marker: Default::default(),
        }
    }

    pub fn node_behaviour_element(
        mut self,
        node_behaviour_element: impl Into<Option<Element<'a, M, R>>>,
    ) -> Self {
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

    pub fn push_input_channel(mut self, channel: impl Into<ChannelRef<'a>>) -> Self {
        self.input_channels.push(channel.into());
        self
    }

    pub fn push_output_channel(mut self, channel: impl Into<ChannelRef<'a>>) -> Self {
        self.output_channels.push(channel.into());
        self
    }

    pub fn build(self) -> NodeElement<'a, M, R> {
        NodeElement {
            index: self.index,
            state: self.state,
            width: self.width,
            height: self.height,
            element_tree: {
                // Element { Margin { Row [ Column [ .. ], Column [ .. ] ] } }
                Margin::new(
                    {
                        let mut column =
                            Column::new().width(Length::Fill).spacing(style::consts::SPACING_VERTICAL);

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
                                .push(Space::with_width(Length::Fill))
                                .push({
                                    // output channels
                                    let mut column = Column::new()
                                        .spacing(style::consts::SPACING_VERTICAL)
                                        .align_items(Align::End);

                                    for output_channel in &self.output_channels {
                                        column = column.push(output_channel.render());
                                    }

                                    column
                                }),
                        );

                        column
                    },
                    style::consts::SPACING,
                )
                .into()
            },
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
    ) -> bool {
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
    ) -> Option<usize> {
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
        viewport: &Rectangle,
    ) -> <R as iced_native::Renderer>::Output {
        self.element_tree.draw(renderer, defaults, layout, cursor_position, viewport)
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
    ) -> Status {
        self.element_tree.on_event(event, layout, cursor_position, messages, renderer, clipboard)
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
    // FIXME: Make it possible to store references instead of cloning
    pub graph_validation_errors: GraphValidationErrors,
}

macro_rules! get_is_aliased {
    ($panes:expr) => {
        move |from| {
            $panes.behaviour.connections.iter().filter(|connection| connection.from() == from).count() > 1
        }
    };
}

impl<M: Clone> FloatingPanesBehaviour<M> {
    /// A reflexive function to check whether two channels can be connected
    fn can_connect<'a, R: 'a + WidgetRenderer>(
        panes: &FloatingPanes<'a, M, R, Self>,
        from: ChannelIdentifier,
        to: ChannelIdentifier,
    ) -> bool {
        if let Some(connection) = Connection::try_from_identifiers([from, to]) {
            connection.is_valid(&get_is_aliased!(panes), &move |channel| {
                let pane = panes.children.get(&channel.node_index).unwrap();

                pane.behaviour_data.node_configuration.channel(channel.channel_direction, channel.into())
            })
        } else {
            false
        }
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
        viewport: &Rectangle,
    ) -> ContentDrawResult<R> {
        <R as WidgetRenderer>::draw_panes(renderer, panes, defaults, layout, cursor_position, viewport)
    }

    fn hash_panes(_panes: &FloatingPanes<'a, M, R, Self>, _state: &mut Hasher) {
        // This implementation of [`floating_panes::FloatingPanesBehaviour`] does not influence the
        // layout of the floating panes.
    }

    fn on_event(
        panes: &mut FloatingPanes<'a, M, R, Self>,
        event: Event,
        layout: FloatingPanesLayout<'_>,
        cursor_position: Point,
        messages: &mut Vec<M>,
        renderer: &R,
        clipboard: Option<&dyn Clipboard>,
    ) -> Status {
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

                    let node = panes.children.get(&node_index).unwrap();
                    let inputs_layout = pane_layout
                        .content()
                        .channels_with_direction(ChannelDirection::In)
                        .channels()
                        .zip(node.behaviour_data.node_configuration.channels(ChannelDirection::In));
                    let outputs_layout = pane_layout
                        .content()
                        .channels_with_direction(ChannelDirection::Out)
                        .channels()
                        .zip(node.behaviour_data.node_configuration.channels(ChannelDirection::Out));
                    let channel_layouts = inputs_layout.chain(outputs_layout);

                    let highlighted_channel = channel_layouts
                        .filter(|(channel_layout, channel_ref)| {
                            // If a new connection is being formed, make sure the target channel
                            // can be connected to.
                            if let Some(selected_channel) = panes.behaviour_state.selected_channel.as_ref() {
                                let node_configuration = &panes
                                    .children
                                    .get(&node_index)
                                    .unwrap()
                                    .behaviour_data
                                    .node_configuration;
                                let channel = channel_ref.into_identifier(node_index);

                                if !FloatingPanesBehaviour::can_connect(panes, *selected_channel, channel) {
                                    return false;
                                }
                            }

                            NodeElement::<M, R>::is_channel_selected(
                                channel_layout.clone(),
                                channel_ref.direction,
                                cursor_position,
                            )
                        })
                        .next();

                    if let Some((channel_layout, channel_ref)) = highlighted_channel {
                        let channel = channel_ref.into_identifier(node_index);
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
                    return Status::Captured;
                }

                panes.behaviour_state.selected_channel = None;
            }
            _ => (),
        }

        Status::Ignored
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
        viewport: &Rectangle,
    ) -> ContentDrawResult<Self>;
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
        viewport: &Rectangle,
    ) -> ContentDrawResult<Self> {
        let mut mouse_interaction = mouse::Interaction::default();
        let mut primitives = Vec::new();

        primitives.extend(panes.children.iter().zip(layout.panes()).map(
            |((_child_index, child), layout)| {
                let (primitive, new_mouse_interaction) =
                    child.element_tree.draw(self, defaults, layout.into(), cursor_position, viewport);

                if new_mouse_interaction > mouse_interaction {
                    mouse_interaction = new_mouse_interaction;
                }

                primitive
            },
        ));

        let mut frame = Frame::new(layout.bounds().size());

        // Highlight pane-related errors
        for ((node_index, _pane), pane_layout) in panes.children.iter().zip(layout.panes()) {
            if panes.behaviour.graph_validation_errors.is_invalid(*node_index) {
                let layout_bounds = pane_layout.bounds();
                frame.stroke(
                    &Path::rectangle(layout_bounds.min().into_array().into(), layout_bounds.size()),
                    Stroke {
                        color: Color::from_rgb(1.0, 0.0, 0.0),
                        width: 2.0,
                        line_cap: LineCap::Square,
                        line_join: LineJoin::Miter,
                    },
                );
            }
        }

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
            let mut stroke = if highlighted {
                Stroke {
                    color: Color::from_rgba(0.5, 1.0, 0.0, 1.0),
                    width: 3.0,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                }
            } else {
                Stroke {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
                    width: 2.0,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                }
            };

            // Highlight connection-related errors
            if panes.behaviour.graph_validation_errors.is_invalid(connection.clone()) {
                stroke.color = Color::from_rgba(1.0, 0.0, 0.0, 1.0);
            }

            // primitives.push(draw_point(from.into_array().into(), Color::from_rgb(1.0, 0.0, 0.0)));
            // primitives.push(draw_point(to.into_array().into(), Color::from_rgb(0.0, 0.0, 1.0)));
            let connection_pass_by =
                ConnectionPassBy::derive_connection_pass_by(&get_is_aliased!(panes), connection);

            ConnectionCurve { from, to }.draw(&mut frame, stroke, connection_pass_by.get_stroke_type());

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
            let (target_position, connection_pass_by) = if let Some(Highlight::Channel(highlighted_channel)) =
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
                let target_position = NodeElement::<M, Self>::get_connection_point(
                    layout_channel,
                    highlighted_channel.channel_direction,
                );

                let connection =
                    Connection::try_from_identifiers([*selected_channel, *highlighted_channel]).unwrap();
                let connection_pass_by =
                    ConnectionPassBy::derive_connection_pass_by(&get_is_aliased!(panes), &connection);

                (target_position, connection_pass_by)
            } else {
                let connection_pass_by = ConnectionPassBy::derive_pending_connection_pass_by(
                    &get_is_aliased!(panes),
                    *selected_channel,
                );

                (panes.state.cursor_position, connection_pass_by)
            };

            let (from, to) = match selected_channel.channel_direction {
                ChannelDirection::In => (target_position, connected_position),
                ChannelDirection::Out => (connected_position, target_position),
            };

            let stroke = Stroke {
                color: Color::from_rgba(1.0, 0.6, 0.0, 1.0),
                width: 3.0,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
            };

            ConnectionCurve { from, to }.draw(&mut frame, stroke, connection_pass_by.get_stroke_type());
        }

        primitives.push(frame.into_geometry().into_primitive());

        // Draw connection points
        {
            for (pane_layout, node_index) in layout.panes().zip(panes.children.keys().copied()) {
                let node = panes.children.get(&node_index).unwrap();
                let inputs_layout = pane_layout
                    .content()
                    .channels_with_direction(ChannelDirection::In)
                    .channels()
                    .zip(node.behaviour_data.node_configuration.channels(ChannelDirection::In));
                let outputs_layout = pane_layout
                    .content()
                    .channels_with_direction(ChannelDirection::Out)
                    .channels()
                    .zip(node.behaviour_data.node_configuration.channels(ChannelDirection::Out));
                let channel_layouts = inputs_layout.chain(outputs_layout);

                for (channel_layout, channel_ref) in channel_layouts {
                    let position =
                        NodeElement::<M, Self>::get_connection_point(channel_layout, channel_ref.direction);
                    let channel = channel_ref.into_identifier(node_index);
                    let highlighted = if let Some(Highlight::Channel(highlighted_channel)) =
                        panes.behaviour_state.highlight.as_ref()
                    {
                        *highlighted_channel == channel
                    } else {
                        false
                    };
                    let error = panes.behaviour.graph_validation_errors.is_invalid(channel);

                    draw_connection_point(
                        panes,
                        &mut primitives,
                        node_index,
                        position,
                        channel_ref.edge_endpoint.pass_by,
                        highlighted,
                        error,
                    );
                }
            }
        }

        ContentDrawResult {
            override_parent_cursor: panes.behaviour_state.highlight.is_some(),
            output: (Primitive::Group { primitives }, mouse_interaction),
        }
    }
}

fn draw_connection_point<M: Clone, B>(
    panes: &FloatingPanes<'_, M, iced_graphics::Renderer<B>, FloatingPanesBehaviour<M>>,
    primitives: &mut Vec<Primitive>,
    node_index: NodeIndex,
    position: Vec2<f32>,
    channel_pass_by: ChannelPassBy,
    highlighted: bool,
    error: bool,
) where
    B: Backend + iced_graphics::backend::Text,
{
    let solid = channel_pass_by > ChannelPassBy::SharedReference;
    let (radius, mut color) =
        if highlighted { (5.0, Color::from_rgb(0.5, 1.0, 0.0)) } else { (3.5, Color::WHITE) };

    if error {
        color = Color::from_rgb(1.0, 0.0, 0.0);
    }

    primitives.push(util::draw_point(position, color, radius));

    if !solid {
        let pane = panes.children.get(&node_index).unwrap();
        let color = pane.style.as_ref().unwrap().style().body_background_color;

        primitives.push(util::draw_point(position, color, radius * (2.0 / 3.0)));
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
    ) -> Self {
        let from = NodeElement::<M, R>::get_connection_point(output, ChannelDirection::Out);
        let to = NodeElement::<M, R>::get_connection_point(input, ChannelDirection::In);
        Self { from, to }
    }

    fn draw(&self, frame: &mut Frame, stroke: Stroke, stroke_type: StrokeType) {
        let segments = util::get_connection_curve(self.from, self.to);
        let path = Path::new(|builder| {
            builder.move_to(self.from.into_array().into());
            // segments.build_segments(builder);
            segments.stroke(builder, stroke_type);

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

    #[allow(dead_code)]
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
                        ChannelDirection::Out => 2,
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
