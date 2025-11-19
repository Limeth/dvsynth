use super::*;
use crate::graph::{GraphValidationErrorAffectedElement, GraphValidationErrors};
use crate::node::{ChannelPassBy, ChannelRef, ConnectionPassBy, NodeConfiguration, TypeEnum, TypeExt};
use crate::style::InteractionStatus;
use crate::util::{RectangleExt, Segments, StrokeType};
use crate::{ChannelDirection, ChannelIdentifier, Connection, style, util};
use iced::alignment::Horizontal;
use iced::mouse::Cursor;
use iced::widget::canvas::{Fill, Frame};
use iced::widget::{Column, Container, Row, Space, Text};
use iced::{Size, Vector};
use iced_core::event::Status;
use iced_core::layout::{Layout, Limits, Node};
use iced_core::mouse::{self, Button as MouseButton, Event as MouseEvent};
use iced_core::overlay::{self, Group, Overlay};
use iced_core::widget::{Tree, Widget};
use iced_core::{self, Clipboard, Element, Event, Length, Point, Rectangle};
use iced_core::{Color, Shell};
use iced_graphics::geometry::{LineCap, LineDash, LineJoin, Path, Stroke, Style};
use lyon_geom::QuadraticBezierSegment;
use ordered_float::OrderedFloat;
use petgraph::graph::NodeIndex;
use std::hash::Hash;
use std::marker::PhantomData;
use vek::Vec2;

impl<'a> ChannelRef<'a> {
    pub fn render<M, T, R>(&self) -> Element<'a, M, T, R>
    where
        M: 'a + Clone,
        T: 'a + iced::widget::text::Catalog,
        R: 'a + WidgetRenderer,
    {
        Text::new(self.title.to_string()).size(style::consts::TEXT_SIZE_REGULAR).into()
    }
}

#[derive(Default)]
pub struct NodeElementState {
    __marker: (), // prevent direct construction for future proofing
}

pub struct NodeElementBuilder<'a, M: 'a + Clone, T: 'a, R: 'a + WidgetRenderer> {
    index: NodeIndex,
    state: &'a NodeElementState,
    node_behaviour_element: Option<Element<'a, M, T, R>>,
    // TODO: Change to Size
    width: Length,
    height: Length,
    input_channels: Vec<ChannelRef<'a>>,
    output_channels: Vec<ChannelRef<'a>>,
    __marker: std::marker::PhantomData<&'a (M, R)>,
}

/// A widget specifically made to be used as the child of the [`FloatingPanes`] widget alongside the
/// custom behaviour [`FloatingPanesBehaviour`] to function as a node graph editor.
#[allow(dead_code)]
pub struct NodeElement<'a, M: 'a + Clone, T: 'a, R: 'a + WidgetRenderer> {
    index: NodeIndex,
    state: &'a NodeElementState,
    width: Length,
    height: Length,
    element_tree: Element<'a, M, T, R>,
}

impl<'a, M, T, R> NodeElementBuilder<'a, M, T, R>
where
    M: 'a + Clone,
    T: 'a + iced::widget::text::Catalog,
    R: 'a + WidgetRenderer,
{
    pub fn new(index: NodeIndex, state: &'a NodeElementState) -> Self {
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
        node_behaviour_element: impl Into<Option<Element<'a, M, T, R>>>,
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

    pub fn build(self) -> NodeElement<'a, M, T, R> {
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
                                        .align_x(Horizontal::Left);

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
                                        .align_x(Horizontal::Right);

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

impl<'a, M, T, R> NodeElement<'a, M, T, R>
where
    M: 'a + Clone,
    T: 'a + iced::widget::text::Catalog + iced::widget::container::Catalog,
    R: 'a + WidgetRenderer,
{
    pub fn builder(index: NodeIndex, state: &'a NodeElementState) -> NodeElementBuilder<'a, M, T, R> {
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
        panes: &FloatingPanes<'a, M, T, R, FloatingPanesBehaviour<M, R>>,
        channel: ChannelIdentifier,
    ) -> Option<usize> {
        panes.get_layout_index_from_pane_index(&channel.node_index)
    }
}

impl<'a, M, T, R> Widget<M, T, R> for NodeElement<'a, M, T, R>
where
    M: 'a + Clone,
    T: 'a + iced::widget::text::Catalog,
    R: 'a + WidgetRenderer,
{
    fn size(&self) -> Size<iced::Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, state: &mut Tree, renderer: &R, limits: &Limits) -> Node {
        // let limits = limits
        //     .max_width(self.extents[0])
        //     .max_height(self.extents[1])
        //     .width(self.width)
        //     .height(self.height);
        self.element_tree.as_widget().layout(state, renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.element_tree.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

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
        self.element_tree
            .as_widget_mut()
            .on_event(state, event, layout, cursor, renderer, clipboard, shell, viewport)
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &R,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>> {
        self.element_tree.as_widget_mut().overlay(state, layout, renderer, translation)
    }
}

impl<'a, M, T, R> From<NodeElement<'a, M, T, R>> for Element<'a, M, T, R>
where
    M: 'a + Clone,
    T: 'a + iced::widget::text::Catalog,
    R: 'a + WidgetRenderer,
{
    fn from(other: NodeElement<'a, M, T, R>) -> Self {
        Element::new(other)
    }
}

pub struct FloatingPanesBehaviour<M, R: WidgetRenderer> {
    // TODO: Make replace all of these event handlers with a common event handler over an enum?
    pub on_channel_disconnect: fn(ChannelIdentifier) -> M,
    pub on_connection_create: fn(Connection) -> M,
    pub on_highlight_change: fn(Option<Highlight>) -> M,
    pub on_selected_channel_changed: fn(Option<ChannelIdentifier>) -> M,
    pub connections: Vec<Connection>,
    // FIXME: Make it possible to store references instead of cloning
    pub graph_validation_errors: GraphValidationErrors,
    // pub tooltip_style: Option<<R as WidgetRenderer>::StyleTooltip>,
    pub __marker: PhantomData<R>,
}

macro_rules! get_is_aliased {
    ($panes:expr) => {
        move |from| {
            $panes.behaviour.connections.iter().filter(|connection| connection.from() == from).count() > 1
        }
    };
}

impl<M: Clone, R: WidgetRenderer> FloatingPanesBehaviour<M, R> {
    /// A reflexive function to check whether two channels can be connected
    fn can_connect<'a, T>(
        panes: &FloatingPanes<'a, M, T, R, Self>,
        from: ChannelIdentifier,
        to: ChannelIdentifier,
    ) -> bool
    where
        T: iced::widget::text::Catalog + iced::widget::container::Catalog,
    {
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

impl<'a, M, T, R> floating_panes::FloatingPanesBehaviour<'a, M, T, R> for FloatingPanesBehaviour<M, R>
where
    M: Clone + 'a,
    T: 'a + iced::widget::text::Catalog + iced::widget::container::Catalog,
    R: 'a + WidgetRenderer,
{
    type FloatingPaneIndex = NodeIndex;
    type FloatingPaneBehaviourData = FloatingPaneBehaviourData;
    type FloatingPaneBehaviourState = FloatingPaneBehaviourState;
    type FloatingPanesBehaviourState = FloatingPanesBehaviourState;

    fn draw_panes(
        panes: &FloatingPanes<'a, M, T, R, Self>,
        state: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) -> ContentDrawResult {
        <R as WidgetRenderer>::draw_panes(renderer, panes, state, theme, style, layout, cursor, viewport)
    }

    fn on_event(
        panes: &mut FloatingPanes<'a, M, T, R, Self>,
        state: &mut Tree,
        event: Event,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        match event {
            Event::Mouse(MouseEvent::CursorMoved { position: Point { x, y } }) => {
                let highlight = (|| {
                    let cursor_position = Vec2::new(x, y);

                    // Highlight channel, if possible
                    for (pane_layout, node_index) in layout.panes().zip(panes.children.keys().copied()) {
                        let pane_bounding_box = pane_layout
                            .bounds()
                            .grow_symmetrical(style::consts::SPACING_HORIZONTAL as f32, 0.0);

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
                                if let Some(selected_channel) =
                                    panes.behaviour_state.selected_channel.as_ref()
                                {
                                    let channel = channel_ref.into_identifier(node_index);

                                    if !FloatingPanesBehaviour::can_connect(panes, *selected_channel, channel)
                                    {
                                        return false;
                                    }
                                }

                                NodeElement::<M, T, R>::is_channel_selected(
                                    channel_layout.clone(),
                                    channel_ref.direction,
                                    cursor_position,
                                )
                            })
                            .next();

                        if let Some((_channel_layout, channel_ref)) = highlighted_channel {
                            let channel = channel_ref.into_identifier(node_index);
                            return Some(Highlight::Channel(channel));
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
                                        NodeElement::<M, T, R>::get_layout_index_from_channel(
                                            panes,
                                            connection.from(),
                                        )
                                        .unwrap(),
                                    )
                                    .unwrap();
                                let layout_to = layout
                                    .panes()
                                    .nth(
                                        NodeElement::<M, T, R>::get_layout_index_from_channel(
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
                                let connection_curve = ConnectionCurve::from_channel_layouts::<M, T, R>(
                                    layout_output,
                                    layout_input,
                                );
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
                            return Some(Highlight::Connection(closest_connection.clone()));
                        }
                    }

                    None
                })();

                shell.publish((panes.behaviour.on_highlight_change)(highlight));
            }
            Event::Mouse(MouseEvent::ButtonPressed(MouseButton::Left)) => {
                if let Some(highlight) = &panes.behaviour_state.highlight {
                    let mut new_selected_channel = None;
                    {
                        match highlight {
                            Highlight::Connection(highlighted_connection) => {
                                new_selected_channel = Some(highlighted_connection.from());
                                shell.publish((panes.behaviour.on_channel_disconnect)(
                                    highlighted_connection.to(),
                                ));
                            }
                            Highlight::Channel(channel @ ChannelIdentifier { channel_direction, .. }) => {
                                let disconnect = match channel_direction {
                                    ChannelDirection::In => panes.behaviour.is_connected(*channel),
                                    ChannelDirection::Out => false,
                                };

                                // Is connection pending?
                                if let Some(selected_channel) = panes.behaviour_state.selected_channel.clone()
                                {
                                    if FloatingPanesBehaviour::can_connect(panes, selected_channel, *channel)
                                    {
                                        if disconnect {
                                            shell.publish((panes.behaviour.on_channel_disconnect)(*channel));
                                        }

                                        let channels = match selected_channel.channel_direction {
                                            ChannelDirection::In => [*channel, selected_channel],
                                            ChannelDirection::Out => [selected_channel, *channel],
                                        };

                                        shell.publish((panes.behaviour.on_connection_create)(
                                            Connection::try_from_identifiers(channels).unwrap(),
                                        ));
                                        new_selected_channel = None;
                                    }
                                } else {
                                    if disconnect {
                                        let connection = panes
                                            .behaviour
                                            .connections
                                            .iter()
                                            .find(|connection| connection.contains_channel(*channel));
                                        if let Some(connection) = connection {
                                            let other_channel =
                                                connection.channel(channel.channel_direction.inverse());
                                            new_selected_channel = Some(other_channel);

                                            shell.publish((panes.behaviour.on_channel_disconnect)(*channel));
                                        }
                                    } else {
                                        new_selected_channel = Some(*channel);
                                    }
                                }
                            }
                        }
                    };

                    shell.publish((panes.behaviour.on_selected_channel_changed)(new_selected_channel));

                    // Properly update the highlight
                    Self::on_event(
                        panes,
                        state,
                        Event::Mouse(MouseEvent::CursorMoved {
                            position: cursor.position().unwrap_or_default(),
                        }),
                        layout,
                        cursor,
                        renderer,
                        clipboard,
                        shell,
                        viewport,
                    );

                    return Status::Captured;
                }
            }
            _ => (),
        }

        Status::Ignored
    }

    fn overlay<'b>(
        panes: &'b mut FloatingPanes<'a, M, T, R, Self>,
        state: &'b mut Tree,
        layout: FloatingPanesLayout<'_>,
        renderer: &R,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>> {
        let mut errors = panes
            .behaviour_state
            .highlight
            .as_ref()
            .map(|highlight| {
                panes
                    .behaviour
                    .graph_validation_errors
                    .get_related_errors(highlight.clone().into_graph_validation_error_affected_element())
            })
            .unwrap_or(&[]);

        if errors.is_empty() {
            errors = panes
                .children
                .iter()
                .find_map(|(pane_index, pane)| {
                    if pane.state.title_bar_status > InteractionStatus::Idle {
                        let errors = panes.behaviour.graph_validation_errors.get_related_errors(*pane_index);

                        Some(errors)
                    } else {
                        None
                    }
                })
                .unwrap_or(&[]);
        }

        if !errors.is_empty() {
            let mut column = Column::<M, T, R>::new();

            for error in errors {
                let display = error.display();
                let mut error_element = Column::<M, T, R>::new()
                    .max_width(512)
                    .push(Text::new(display.title.to_string()).size(style::consts::TEXT_SIZE_TITLE))
                    .push(Text::new(display.description.to_string()).size(style::consts::TEXT_SIZE_REGULAR));

                if let Some(suggestion) = display.suggestion.as_ref() {
                    error_element = error_element.push(
                        Text::new(format!("Suggestion: {}", suggestion))
                            .size(style::consts::TEXT_SIZE_REGULAR),
                    );
                }

                let mut container =
                    Container::<M, T, R>::new(Margin::new(error_element, style::consts::SPACING));

                // if let Some(style) = panes.behaviour.tooltip_style.as_ref() {
                //     container = container.style(style.container_style());
                // }

                column = column.push(container);
            }

            // let position: Point = panes.state.cursor_position.into_array().into();
            let position = Point { x: translation.x, y: translation.y };
            let overlay = WidgetOverlay::<M, T, R, _>::new(
                column,
                WidgetOverlayAlignment { top: true, left: false },
                position,
            );

            return Some(overlay::Element::new(Box::new(overlay)));
        }

        let mut group = Group::new();

        for (((_, pane), child_state), child_layout) in
            panes.children.iter_mut().zip(state.children.iter_mut()).zip(layout.panes())
        {
            if let Some(overlay) = pane.element_tree.as_widget_mut().overlay(
                child_state,
                child_layout.into(),
                renderer,
                translation,
            ) {
                group = group.push(overlay);
            }
        }

        Some(group.into())
    }
}

pub struct FloatingPaneBehaviourData {
    pub node_configuration: NodeConfiguration,
}

#[derive(Default)]
pub struct FloatingPaneBehaviourState {}

#[derive(Debug, Clone)]
pub enum Highlight {
    Channel(ChannelIdentifier),
    Connection(Connection),
}

impl Highlight {
    pub fn into_graph_validation_error_affected_element(self) -> GraphValidationErrorAffectedElement {
        use Highlight::*;
        match self {
            Channel(channel) => GraphValidationErrorAffectedElement::Channel(channel),
            Connection(connection) => GraphValidationErrorAffectedElement::Connection(connection),
        }
    }
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
    + iced_graphics::geometry::Renderer
    + floating_panes::WidgetRenderer
    + iced_core::Renderer
    + iced_core::text::Renderer
    + Sized
{
    // type StyleTooltip: StyleTooltipBounds<Self>;

    fn draw_panes<M, T>(
        &mut self,
        panes: &FloatingPanes<'_, M, T, Self, FloatingPanesBehaviour<M, Self>>,
        state: &Tree,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) -> ContentDrawResult
    where
        M: Clone,
        T: iced::widget::text::Catalog + iced::widget::container::Catalog;
}

impl<R> WidgetRenderer for R
where R: margin::WidgetRenderer
        + iced_graphics::geometry::Renderer
        + floating_panes::WidgetRenderer
        + iced_core::Renderer
        + iced_core::text::Renderer
        + Sized
{
    // type StyleTooltip = Box<dyn TooltipStyleSheet>;

    fn draw_panes<M, T>(
        &mut self,
        panes: &FloatingPanes<'_, M, T, Self, FloatingPanesBehaviour<M, Self>>,
        state: &Tree,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) -> ContentDrawResult
    where
        M: Clone,
        T: iced::widget::text::Catalog + iced::widget::container::Catalog,
    {
        let mut mouse_interaction = mouse::Interaction::default();
        let mut primitives = Vec::new();

        primitives.extend(panes.children.iter().zip(layout.panes()).zip(&state.children).map(
            |(((_child_index, child), child_layout), child_state)| {
                /*let (primitive, new_mouse_interaction) =*/
                child.element_tree.as_widget().draw(
                    child_state,
                    self,
                    theme,
                    style,
                    child_layout.into(),
                    cursor,
                    viewport,
                );

                // if new_mouse_interaction > mouse_interaction {
                //     mouse_interaction = new_mouse_interaction;
                // }

                // primitive
            },
        ));

        let mut frame = Frame::new(self, layout.bounds().size());

        // Highlight pane-related errors
        for ((node_index, _pane), pane_layout) in panes.children.iter().zip(layout.panes()) {
            if panes.behaviour.graph_validation_errors.is_invalid(*node_index) {
                let layout_bounds = pane_layout.bounds();
                frame.stroke(
                    &Path::rectangle(layout_bounds.min().into_array().into(), layout_bounds.size()),
                    Stroke {
                        style: Style::Solid(Color::from_rgb(1.0, 0.0, 0.0)),
                        width: 2.0,
                        line_cap: LineCap::Square,
                        line_join: LineJoin::Miter,
                        line_dash: Default::default(),
                    },
                );
            }
        }

        // Draw existing connections
        for connection in &panes.behaviour.connections {
            let layout_from = layout.pane_with_index(
                NodeElement::<M, T, Self>::get_layout_index_from_channel(panes, connection.from()).unwrap(),
            );
            let layout_to = layout.pane_with_index(
                NodeElement::<M, T, Self>::get_layout_index_from_channel(panes, connection.to()).unwrap(),
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

            let from = NodeElement::<M, T, Self>::get_connection_point(layout_output, ChannelDirection::Out);
            let to = NodeElement::<M, T, Self>::get_connection_point(layout_input, ChannelDirection::In);

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
                    style: Style::Solid(Color::from_rgba(0.5, 1.0, 0.0, 1.0)),
                    width: 3.0,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    line_dash: Default::default(),
                }
            } else {
                Stroke {
                    style: Style::Solid(Color::from_rgba(1.0, 1.0, 1.0, 1.0)),
                    width: 2.0,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    line_dash: Default::default(),
                }
            };

            // Highlight connection-related errors
            if panes.behaviour.graph_validation_errors.is_invalid(connection.clone()) {
                stroke.style = Style::Solid(Color::from_rgba(1.0, 0.0, 0.0, 1.0));
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
                .nth(
                    NodeElement::<M, T, Self>::get_layout_index_from_channel(panes, *selected_channel)
                        .unwrap(),
                )
                .unwrap();
            let layout_channels =
                pane_layout.content().channels_with_direction(selected_channel.channel_direction);
            let layout_channel = layout_channels.channel(selected_channel.channel_index);

            let connected_position = NodeElement::<M, T, Self>::get_connection_point(
                layout_channel,
                selected_channel.channel_direction,
            );
            let (target_position, connection_pass_by) = if let Some(Highlight::Channel(highlighted_channel)) =
                panes.behaviour_state.highlight.as_ref()
            {
                let child_layout = layout
                    .panes()
                    .nth(
                        NodeElement::<M, T, Self>::get_layout_index_from_channel(panes, *highlighted_channel)
                            .unwrap(),
                    )
                    .unwrap();
                let layout_channels =
                    child_layout.content().channels_with_direction(highlighted_channel.channel_direction);
                let layout_channel = layout_channels.channel(highlighted_channel.channel_index);
                let target_position = NodeElement::<M, T, Self>::get_connection_point(
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
                let target_position = cursor.position().unwrap_or_default();

                ([target_position.x, target_position.y].into(), connection_pass_by)
            };

            let (from, to) = match selected_channel.channel_direction {
                ChannelDirection::In => (target_position, connected_position),
                ChannelDirection::Out => (connected_position, target_position),
            };

            let stroke = Stroke {
                style: Style::Solid(Color::from_rgba(1.0, 0.6, 0.0, 1.0)),
                width: 3.0,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                line_dash: Default::default(),
            };

            ConnectionCurve { from, to }.draw(&mut frame, stroke, connection_pass_by.get_stroke_type());
        }

        // primitives.push(frame.into_geometry().into_primitive());

        // Draw connection points
        {
            for (pane_layout, (&node_index, pane)) in layout.panes().zip(&panes.children) {
                let node = panes.children.get(&node_index).unwrap();

                fn debug_layout(layout: &Layout, depth: usize) {
                    let prefix = std::iter::repeat_n(' ', depth).collect::<String>();
                    if layout.children().next().is_none() {
                        println!("{prefix}{{}}");
                    } else {
                        println!("{prefix}{{");
                        for child in layout.children() {
                            debug_layout(&child, depth + 1);
                        }
                        println!("{prefix}}}");
                    }
                }

                print!("Node #{node_index:?}");
                debug_layout(&pane_layout.into(), 0);

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
                    let position = NodeElement::<M, T, Self>::get_connection_point(
                        channel_layout,
                        channel_ref.direction,
                    );
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
                        self,
                        panes,
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
            // output: (Primitive::Group { primitives }, mouse_interaction),
        }
    }
}

// pub trait StyleTooltipBounds<R: WidgetRenderer> {
//     fn container_style(&self) -> <R as iced_runtime::widget::container::Renderer>::Style;
// }

// pub trait TooltipStyleSheet {
//     fn style(&self) -> TooltipStyle;
// }

// impl<R> StyleTooltipBounds<R> for Box<dyn TooltipStyleSheet> {
//     fn container_style(&self) -> Box<(dyn iced::container::StyleSheet + 'static)> {
//         self.style().container
//     }
// }

// pub struct TooltipStyle {
//     pub container: Box<(dyn iced::container::StyleSheet + 'static)>,
// }

fn draw_connection_point<M, T, R>(
    renderer: &mut R,
    panes: &FloatingPanes<'_, M, T, R, FloatingPanesBehaviour<M, R>>,
    node_index: NodeIndex,
    position: Vec2<f32>,
    channel_pass_by: ChannelPassBy,
    highlighted: bool,
    error: bool,
) where
    M: Clone,
    T: iced::widget::text::Catalog + iced::widget::container::Catalog,
    R: WidgetRenderer,
{
    let solid = channel_pass_by > ChannelPassBy::SharedReference;
    let (radius, mut color) =
        if highlighted { (5.0, Color::from_rgb(0.5, 1.0, 0.0)) } else { (3.5, Color::WHITE) };

    if error {
        color = Color::from_rgb(1.0, 0.0, 0.0);
    }

    util::draw_point(renderer, position, color, radius);

    if !solid {
        let pane = panes.children.get(&node_index).unwrap();
        // TODO
        // let color =
        // pane.style.as_ref().unwrap().style(style::InteractionStatus::Idle).body_background_color;
        let color = Color::from_rgb(1.0, 0.8, 0.2);

        util::draw_point(renderer, position, color, radius * (2.0 / 3.0));
    }
}

pub struct ConnectionCurve {
    pub from: Vec2<f32>,
    pub to: Vec2<f32>,
}

impl ConnectionCurve {
    fn from_channel_layouts<M, T, R>(output: ChannelLayout, input: ChannelLayout) -> Self
    where
        M: Clone,
        T: iced::widget::text::Catalog + iced::widget::container::Catalog,
        R: WidgetRenderer,
    {
        let from = NodeElement::<M, T, R>::get_connection_point(output, ChannelDirection::Out);
        let to = NodeElement::<M, T, R>::get_connection_point(input, ChannelDirection::In);
        Self { from, to }
    }

    fn draw<R: iced_graphics::geometry::Renderer>(
        &self,
        frame: &mut Frame<R>,
        stroke: Stroke,
        stroke_type: StrokeType,
    ) {
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
                .reduce(util::partial_min)
                .unwrap(),
            util::partial_min(segments[0].from.y, segments[1].to.y),
        );
        let max = Vec2::<f32>::new(
            [segments[0].from.x, segments[0].ctrl.x, segments[1].ctrl.x, segments[1].to.x]
                .iter()
                .copied()
                .reduce(util::partial_max)
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

pub struct WidgetOverlayAlignment {
    pub top: bool,
    pub left: bool,
}

struct WidgetOverlay<M: Clone, T, R: WidgetRenderer, W: Widget<M, T, R>> {
    pub widget: W,
    pub alignment: WidgetOverlayAlignment,
    pub position: Point,
    __marker: PhantomData<(M, T, R)>,
}

impl<M: Clone, T, R: WidgetRenderer, W: Widget<M, T, R>> WidgetOverlay<M, T, R, W> {
    pub fn new(widget: W, alignment: WidgetOverlayAlignment, position: Point) -> Self {
        Self { widget, alignment, position, __marker: Default::default() }
    }

    pub fn tree(&self) -> Tree {
        Tree::new(&self.widget as &dyn Widget<M, T, R>)
    }
}

impl<M: Clone, T, R: WidgetRenderer, W: Widget<M, T, R>> Overlay<M, T, R> for WidgetOverlay<M, T, R, W> {
    fn layout(&mut self, renderer: &R, bounds: Size) -> Node {
        let mut node = self.widget.layout(&mut self.tree(), renderer, &Limits::new(Size::ZERO, bounds));
        let node_bounds = node.bounds();
        let mut position = self.position;

        if self.alignment.left {
            position.x -= node_bounds.width;
        }

        if self.alignment.top {
            position.y -= node_bounds.height;
        }

        node.move_to(position)
    }

    fn draw(
        &self,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        self.widget.draw(&self.tree(), renderer, theme, style, layout, cursor, &layout.bounds())
    }
}

typed_layout! {
    type_name: Channels,
    traverse: [
        {
            parent_type_name: FloatingPaneContent,
            fn_name: channels_with_direction,
            fn_args: [channel_direction: ChannelDirection],
            layout_fn: |parent: Layout<'a>, channel_direction: ChannelDirection| {
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
            tree_ref_fn: |parent: &'a Tree, channel_direction: ChannelDirection| {
                &parent
                    .children[1]
                    .children[1]
                    .children[1]
                    .children[match channel_direction {
                        ChannelDirection::In => 0,
                        ChannelDirection::Out => 2,
                    }]
            },
            tree_mut_fn: |parent: &'a mut Tree, channel_direction: ChannelDirection| {
                &mut parent
                    .children[1]
                    .children[1]
                    .children[1]
                    .children[match channel_direction {
                        ChannelDirection::In => 0,
                        ChannelDirection::Out => 2,
                    }]
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
            layout_fn: |parent: Layout<'a>, channel_index: usize| {
                parent.children().nth(channel_index).unwrap()
            },
            tree_ref_fn: |parent: &'a Tree, channel_index: usize| {
                &parent.children[channel_index]
            },
            tree_mut_fn: |parent: &'a mut Tree, channel_index: usize| {
                &mut parent.children[channel_index]
            },
        },
    ],
    children_of: {
        parent_type_name: Channels,
        fn_name: channels,
    },
}
