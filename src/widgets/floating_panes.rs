use super::*;
use crate::style::InteractionStatus;
use crate::util::RectangleExt;
use iced::event::Status;
use iced::mouse::{self, Cursor, Interaction};
use iced::widget::{Column, Container};
use iced::{Element, Size, Vector, overlay};
use iced_core::layout::{Limits, Node};
use iced_core::overlay::Group;
use iced_core::renderer::Quad;
use iced_core::widget::{Text, Tree};
use iced_core::{self, Clipboard, Event, Layout, Length, Point, Shell, Widget};
use iced_core::{Background, Color, Rectangle};
use iced_winit::winit::event::MouseButton;
use indexmap::IndexMap;
use ordered_float::OrderedFloat;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use vek::Vec2;

pub enum PrimitiveEnum {
    Quad(Quad),
    Group(Vec<PrimitiveEnum>),
}

pub struct ContentDrawResult /*<R: WidgetRenderer>*/ {
    pub override_parent_cursor: bool,
    // pub output: R::Output,
}

/// A widget-like trait for customizing the behaviour of the [`FloatingPanes`] widget
pub trait FloatingPanesBehaviour<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer>: Sized {
    type FloatingPaneIndex: Hash + Eq;

    /// Additional data passed by value during construction of each pane.
    /// Custom data to pass to the FloatingPanes widget (shared by all floating panes) can be
    /// stored within the implementation of `Self`.
    type FloatingPaneBehaviourData;

    /// Mutable state of each pane stored externally from the widget.
    type FloatingPaneBehaviourState;

    /// Mutable state of all floating panes stored externally from the widget.
    type FloatingPanesBehaviourState;

    fn draw_panes(
        panes: &FloatingPanes<'a, M, T, R, Self>,
        tree: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) -> ContentDrawResult;

    /// Handle event before it isi processed by the main event handler.
    /// Returns `true` if the main event handler should be skipped.
    fn on_event(
        panes: &mut FloatingPanes<'a, M, T, R, Self>,
        tree: &mut Tree,
        event: Event,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status;

    fn overlay<'b>(
        panes: &'b mut FloatingPanes<'a, M, T, R, Self>,
        state: &'b mut Tree,
        layout: FloatingPanesLayout<'_>,
        renderer: &R,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>>;
}

pub struct FloatingPanesBehaviourDefault;

impl<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer + iced_wgpu::primitive::Renderer>
    FloatingPanesBehaviour<'a, M, T, R> for FloatingPanesBehaviourDefault
{
    type FloatingPaneIndex = u32;
    type FloatingPaneBehaviourData = ();
    type FloatingPaneBehaviourState = ();
    type FloatingPanesBehaviourState = ();

    fn draw_panes(
        panes: &FloatingPanes<'a, M, T, R, Self>,
        tree: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) -> ContentDrawResult {
        let mut mouse_interaction = Interaction::default();
        let primitives = panes.children.iter().zip(layout.panes()).map(|((_, child), layout)| {
            // let (primitive, new_mouse_interaction) =
            child.element_tree.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                layout.into(),
                cursor,
                viewport,
            );

            // if new_mouse_interaction > mouse_interaction {
            //     mouse_interaction = new_mouse_interaction;
            // }

            // primitive
        });

        // for primitive in primitives {
        //     renderer.draw_primitive(viewport, primitive);
        // }

        /*
        ContentDrawResult {
            override_parent_cursor: false,
            output: (
                PrimitiveEnum::Group(
                    panes
                        .children
                        .iter()
                        .zip(layout.panes())
                        .map(|((_, child), layout)| {
                            let (primitive, new_mouse_interaction) = child.element_tree.draw(
                                renderer,
                                defaults,
                                layout.into(),
                                cursor_position,
                                viewport,
                            );

                            if new_mouse_interaction > mouse_interaction {
                                mouse_interaction = new_mouse_interaction;
                            }

                            primitive
                        })
                        .collect(),
                ),
                mouse_interaction,
            ),
        }
        */
        ContentDrawResult { override_parent_cursor: false }
    }

    fn on_event(
        panes: &mut FloatingPanes<'a, M, T, R, Self>,
        tree: &mut Tree,
        event: Event,
        layout: FloatingPanesLayout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        Status::Ignored
    }

    fn overlay<'b>(
        panes: &'b mut FloatingPanes<'a, M, T, R, Self>,
        state: &'b mut Tree,
        layout: FloatingPanesLayout<'_>,
        renderer: &R,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>> {
        let mut group = Group::new();

        for (((_, child), child_state), child_layout) in
            panes.children.iter_mut().zip(state.children.iter_mut()).zip(layout.panes())
        {
            group = group.push(
                child
                    .element_tree
                    .as_widget_mut()
                    .overlay(child_state, child_layout.into(), renderer, translation)
                    .unwrap(),
            );
        }

        Some(overlay::Element::new(Box::new(group)))

        // for ((_, pane), layout) in panes.children.iter_mut().zip(layout.panes()) {
        //     if let Some(overlay) =
        //         pane.element_tree.as_widget_mut().overlay(state, layout.into(), renderer, translation)
        //     {
        //         return Some(overlay);
        //     }
        // }

        // None
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum FloatingPaneLength {
    Shrink,
    Fixed(f32),
}

impl Default for FloatingPaneLength {
    fn default() -> Self {
        FloatingPaneLength::Shrink
    }
}

impl From<f32> for FloatingPaneLength {
    fn from(other: f32) -> Self {
        FloatingPaneLength::Fixed(other)
    }
}

pub struct FloatingPaneBuilder<
    'a,
    M: 'a,
    T: 'a,
    R: 'a + WidgetRenderer,
    C: 'a + FloatingPanesBehaviour<'a, M, T, R>,
> {
    pub content: Element<'a, M, T, R>,
    pub state: &'a mut FloatingPaneState,
    pub behaviour_state: &'a mut C::FloatingPaneBehaviourState,
    pub behaviour_data: C::FloatingPaneBehaviourData,
    pub title: Option<&'a str>,
    pub title_size: Option<u16>,
    pub title_margin: Spacing,
    // pub style: Option<<R as WidgetRenderer>::StyleFloatingPane>,
    /// Whether the floating pane is resizeable in each axis
    pub min_size: Vec2<f32>,
    pub resizeable: Vec2<bool>,
    pub __marker: std::marker::PhantomData<(M, C)>,
}

impl<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer, C: 'a + FloatingPanesBehaviour<'a, M, T, R>>
    FloatingPaneBuilder<'a, M, T, R, C>
{
    pub fn new(
        content: impl Into<Element<'a, M, T, R>>,
        state: &'a mut FloatingPaneState,
        behaviour_state: &'a mut C::FloatingPaneBehaviourState,
        behaviour_data: C::FloatingPaneBehaviourData,
    ) -> Self {
        Self {
            content: content.into(),
            state,
            behaviour_data,
            behaviour_state,
            title: Default::default(),
            title_size: Default::default(),
            title_margin: Default::default(),
            // style: Default::default(),
            min_size: [0.0, 0.0].into(),
            resizeable: Default::default(),
            __marker: Default::default(),
        }
    }

    pub fn title(mut self, title: Option<&'a str>) -> Self {
        self.title = title;
        self
    }

    pub fn title_size(mut self, title_size: Option<u16>) -> Self {
        self.title_size = title_size;
        self
    }

    pub fn title_margin(mut self, title_margin: Spacing) -> Self {
        self.title_margin = title_margin;
        self
    }

    // pub fn style<T>(mut self, style: Option<T>) -> Self
    // where T: Into<<R as WidgetRenderer>::StyleFloatingPane> {
    //     self.style = style.map(Into::into);
    //     self
    // }

    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_size[0] = min_width;
        self
    }

    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_size[1] = min_height;
        self
    }

    pub fn width_resizeable(mut self, resizeable: bool) -> Self {
        self.resizeable[0] = resizeable;
        self
    }

    pub fn height_resizeable(mut self, resizeable: bool) -> Self {
        self.resizeable[1] = resizeable;
        self
    }

    pub fn build(mut self) -> FloatingPane<'a, M, T, R, C>
    where T: iced::widget::text::Catalog + iced::widget::container::Catalog {
        FloatingPane {
            behaviour_data: self.behaviour_data,
            min_size: self.min_size,
            resizeable: self.resizeable,
            element_tree: {
                let mut column = Column::<M, T, R>::new();

                if let Some(title) = self.title.take() {
                    let mut text = Text::<T, R>::new(title.to_string());

                    if let Some(title_size) = self.title_size.take() {
                        text = text.size(title_size);
                    }

                    let element: Element<M, T, R> = text.into();
                    let margin = Margin::<M, T, R>::new(element, self.title_margin.clone());
                    column = column.push(margin);
                }

                let mut element_container = Container::<M, T, R>::new(self.content);

                // if let Some(style) = self.style.as_ref() {
                //     element_container =
                //         element_container.style(style.content_container_style(self.state.title_bar_status));
                // }

                let mut container = Container::new(column.push(element_container));

                // if let Some(style) = self.style.as_ref() {
                //     container = container.style(style.root_container_style(self.state.title_bar_status));
                // }

                container = match self.state.size[0] {
                    FloatingPaneLength::Shrink => container,
                    FloatingPaneLength::Fixed(units) => container.width(Length::Fixed(units)),
                };

                container = match self.state.size[1] {
                    FloatingPaneLength::Shrink => container,
                    FloatingPaneLength::Fixed(units) => container.height(Length::Fixed(units)),
                };

                container.into() // Container { Column [ title, Container { element } ] }
            },
            state: self.state,
            // style: self.style,
            __marker: Default::default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct FloatingPaneState {
    pub position: Vec2<f32>,
    pub size: Vec2<FloatingPaneLength>,
    pub title_bar_status: InteractionStatus,
}

// impl Hash for FloatingPaneState {
//     fn hash<H>(&self, state: &mut H)
//     where H: std::hash::Hasher {
//         self.position.map(OrderedFloat::from).as_slice().hash(state);
//         self.size.hash(state);
//         self.title_bar_status.hash(state);
//     }
// }

impl FloatingPaneState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_position(mut self, position: impl Into<Vec2<f32>>) -> Self {
        self.position = position.into();
        self
    }

    pub fn with_width(mut self, width: impl Into<FloatingPaneLength>) -> Self {
        self.size[0] = width.into();
        self
    }

    pub fn with_height(mut self, height: impl Into<FloatingPaneLength>) -> Self {
        self.size[1] = height.into();
        self
    }
}

/// A single floating pane within the [`FloatingPanes`] widget.
pub struct FloatingPane<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer, C: 'a + FloatingPanesBehaviour<'a, M, T, R>>
{
    pub state: &'a mut FloatingPaneState,
    pub behaviour_data: C::FloatingPaneBehaviourData,
    // pub style: Option<<R as WidgetRenderer>::StyleFloatingPane>,
    pub element_tree: Element<'a, M, T, R>,
    pub min_size: Vec2<f32>,
    pub resizeable: Vec2<bool>,
    pub __marker: std::marker::PhantomData<C>,
}

impl<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer, C: 'a + FloatingPanesBehaviour<'a, M, T, R>>
    FloatingPane<'a, M, T, R, C>
{
    pub fn builder(
        content: impl Into<Element<'a, M, T, R>>,
        state: &'a mut FloatingPaneState,
        behaviour_state: &'a mut C::FloatingPaneBehaviourState,
        behaviour_data: C::FloatingPaneBehaviourData,
    ) -> FloatingPaneBuilder<'a, M, T, R, C> {
        FloatingPaneBuilder::new(content, state, behaviour_state, behaviour_data)
    }

    pub fn get_pane_resize_directions(
        &self,
        pane_layout: FloatingPaneLayout,
        cursor_position: Vec2<f32>,
    ) -> PaneResizeDirections {
        const RESIZE_BOUND_OUTER_SIZE: f32 = 8.0;
        const RESIZE_BOUND_OVERLAP_SIZE: f32 = 12.0;

        // Nothing to compute if the pane is not resizeable
        if !self.resizeable[0] && !self.resizeable[1] {
            return PaneResizeDirections::NONE;
        }

        let cursor_point: Point = cursor_position.into_array().into();
        let pane_bounds = pane_layout.bounds();

        // Cannot resize while the cursor is inside of the pane
        if pane_bounds.contains(cursor_point) {
            return PaneResizeDirections::NONE;
        }

        // Early bounds check for optimization
        if !pane_bounds.grow_uniform(RESIZE_BOUND_OUTER_SIZE).contains(cursor_point) {
            return PaneResizeDirections::NONE;
        }

        let pane_layout_size: Vec2<f32> = Into::<[f32; 2]>::into(pane_bounds.size()).into();
        let omnidirectional = self.resizeable[0] && self.resizeable[1];
        let outer_size_secondary = if omnidirectional { RESIZE_BOUND_OUTER_SIZE } else { 0.0 };
        let overlap_size: Vec2<f32> = if omnidirectional {
            (pane_layout_size / 2.0)
                .map(|c| std::cmp::min(OrderedFloat(c), OrderedFloat(RESIZE_BOUND_OVERLAP_SIZE)).into())
        } else {
            Vec2::<f32>::zero()
        };

        let horizontal_direction = if self.resizeable[0] {
            let left = Rectangle {
                x: pane_bounds.min_x() - RESIZE_BOUND_OUTER_SIZE,
                y: pane_bounds.min_y() - outer_size_secondary,
                width: RESIZE_BOUND_OUTER_SIZE + overlap_size[0],
                height: pane_bounds.height + 2.0 * outer_size_secondary,
            };
            let right = Rectangle {
                x: pane_bounds.max_x() - overlap_size[0],
                y: pane_bounds.min_y() - outer_size_secondary,
                width: RESIZE_BOUND_OUTER_SIZE + overlap_size[0],
                height: pane_bounds.height + 2.0 * outer_size_secondary,
            };
            PaneResizeDirection::from_hovered_regions(
                left.contains(cursor_point),
                right.contains(cursor_point),
            )
        } else {
            PaneResizeDirection::None
        };
        let vertical_direction = if self.resizeable[1] {
            let top = Rectangle {
                x: pane_bounds.min_x() - outer_size_secondary,
                y: pane_bounds.min_y() - RESIZE_BOUND_OUTER_SIZE,
                width: pane_bounds.width + 2.0 * outer_size_secondary,
                height: RESIZE_BOUND_OUTER_SIZE + overlap_size[1],
            };
            let bottom = Rectangle {
                x: pane_bounds.min_x() - outer_size_secondary,
                y: pane_bounds.max_y() - overlap_size[1],
                width: pane_bounds.width + 2.0 * outer_size_secondary,
                height: RESIZE_BOUND_OUTER_SIZE + overlap_size[1],
            };
            PaneResizeDirection::from_hovered_regions(
                top.contains(cursor_point),
                bottom.contains(cursor_point),
            )
        } else {
            PaneResizeDirection::None
        };

        [horizontal_direction, vertical_direction].into()
    }
}

#[derive(Default, Debug, Clone)]
pub struct GrabStateResize {
    pub grab_element_position: Vec2<f32>,
    pub grab_element_size: Vec2<f32>,
    pub grab_mouse_position: Vec2<f32>,
}

impl Hash for GrabStateResize {
    fn hash<H>(&self, state: &mut H)
    where H: std::hash::Hasher {
        self.grab_element_position.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_element_size.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_mouse_position.map(OrderedFloat::from).as_slice().hash(state);
    }
}

#[derive(Default, Debug, Clone)]
pub struct GrabStateMove {
    pub grab_element_position: Vec2<f32>,
    pub grab_mouse_position: Vec2<f32>,
}

impl Hash for GrabStateMove {
    fn hash<H>(&self, state: &mut H)
    where H: std::hash::Hasher {
        self.grab_element_position.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_mouse_position.map(OrderedFloat::from).as_slice().hash(state);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PaneResizeDirection {
    None,
    Negative,
    Positive,
}

impl PaneResizeDirection {
    pub fn from_hovered_regions(negative: bool, positive: bool) -> Self {
        match (negative, positive) {
            (false, true) => PaneResizeDirection::Positive,
            (true, false) => PaneResizeDirection::Negative,
            _ => PaneResizeDirection::None,
        }
    }
}

#[derive(Debug, Hash, Clone, Copy)]
pub struct PaneResizeDirections(Vec2<PaneResizeDirection>);

impl PaneResizeDirections {
    pub const NONE: Self =
        PaneResizeDirections(Vec2 { x: PaneResizeDirection::None, y: PaneResizeDirection::None });

    pub fn is_none(&self) -> bool {
        self.x == PaneResizeDirection::None && self.y == PaneResizeDirection::None
    }
}

impl<T: Into<Vec2<PaneResizeDirection>>> From<T> for PaneResizeDirections {
    fn from(other: T) -> Self {
        PaneResizeDirections(other.into())
    }
}

impl Deref for PaneResizeDirections {
    type Target = Vec2<PaneResizeDirection>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PaneResizeDirections {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Hash, Clone)]
pub enum Gesture {
    /// To pan across the pane view (via panes_offset)
    GrabBackground(GrabStateMove),
    /// To move panes around
    GrabPane { pane_index: usize, grab_state: GrabStateMove },
    /// To resize panes, if possible
    ResizePane {
        pending: bool,
        pane_index: usize,
        grab_state: GrabStateResize,
        directions: PaneResizeDirections,
    },
}

impl Gesture {
    pub fn get_mouse_interaction(&self) -> Interaction {
        use Gesture::*;
        match self {
            GrabBackground(_) => Interaction::Grabbing,
            GrabPane { .. } => Interaction::Grabbing,
            ResizePane { directions, .. } => {
                // FIXME: Iced currently only supports vertical and horizontal resize cursors
                if directions[0] != PaneResizeDirection::None {
                    Interaction::ResizingHorizontally
                } else if directions[1] != PaneResizeDirection::None {
                    Interaction::ResizingVertically
                } else {
                    Interaction::default()
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct FloatingPanesState {
    pub cursor_position: Vec2<f32>,
    /// The vector to offset all floating panes' positions by
    pub panes_offset: Vec2<f32>,
    pub gesture: Option<Gesture>,
}

impl Hash for FloatingPanesState {
    fn hash<H>(&self, state: &mut H)
    where H: std::hash::Hasher {
        self.panes_offset.map(OrderedFloat::from).as_slice().hash(state);
        self.gesture.hash(state);
    }
}

pub struct FloatingPanes<
    'a,
    M: 'a,
    T: 'a,
    R: 'a + WidgetRenderer,
    C: 'a + FloatingPanesBehaviour<'a, M, T, R>,
> {
    pub state: &'a mut FloatingPanesState,
    pub behaviour_state: &'a mut C::FloatingPanesBehaviourState,
    pub behaviour: C,
    pub width: Length,
    pub height: Length,
    pub extents: Vec2<f32>,
    // pub style: Option<<R as WidgetRenderer>::StyleFloatingPanes>,
    pub children: IndexMap<C::FloatingPaneIndex, FloatingPane<'a, M, T, R, C>>,
    pub on_layout_change: Box<dyn Fn() -> M>,
}

impl<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer, C: 'a + FloatingPanesBehaviour<'a, M, T, R>>
    FloatingPanes<'a, M, T, R, C>
{
    pub fn new(
        state: &'a mut FloatingPanesState,
        behaviour_state: &'a mut C::FloatingPanesBehaviourState,
        behaviour: C,
        on_layout_change: Box<dyn Fn() -> M>,
    ) -> Self {
        Self {
            state,
            behaviour_state,
            behaviour,
            width: Length::Shrink,
            height: Length::Shrink,
            extents: [f32::INFINITY, f32::INFINITY].into(),
            // style: None,
            children: Default::default(),
            on_layout_change,
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

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.extents[0] = max_width;
        self
    }

    pub fn max_height(mut self, max_height: f32) -> Self {
        self.extents[1] = max_height;
        self
    }

    pub fn extents(mut self, extents: Vec2<f32>) -> Self {
        self.extents = extents;
        self
    }

    // pub fn style<T>(mut self, style: T) -> Self
    // where T: Into<<R as WidgetRenderer>::StyleFloatingPanes> {
    //     self.style = Some(style.into());
    //     self
    // }

    pub fn insert(mut self, index: C::FloatingPaneIndex, child: FloatingPane<'a, M, T, R, C>) -> Self {
        self.children.insert(index, child.into());
        self
    }

    // Use typed layouts instead
    // pub fn get_content_layout_from_child_layout(child_layout: Layout<'_>) -> Layout<'_> {
    //     child_layout.children().nth(0).unwrap().children().nth(1).unwrap().children().nth(0).unwrap()
    // }

    pub fn get_layout_index_from_pane_index(&self, pane_index: &C::FloatingPaneIndex) -> Option<usize> {
        self.children.get_index_of(pane_index)
    }

    pub fn update_pending_gestures(&mut self, layout: FloatingPanesLayout, shell: &mut Shell<'_, M>) {
        // Update the interaction status of title bars
        for ((_, (_, pane)), pane_layout) in self.children.iter_mut().enumerate().zip(layout.panes()) {
            let content_layout = pane_layout.content();
            let pane_bounds = pane_layout.bounds();
            let cursor_on_pane = pane_bounds.contains(self.state.cursor_position.into_array().into());
            let cursor_on_title = cursor_on_pane
                && !content_layout.bounds().contains(self.state.cursor_position.into_array().into());

            let new_title_bar_status =
                if cursor_on_title { InteractionStatus::Hovered } else { InteractionStatus::Idle };

            if new_title_bar_status != pane.state.title_bar_status {
                pane.state.title_bar_status = new_title_bar_status;

                shell.publish((self.on_layout_change)());
            }
        }

        self.state.gesture = self.children.iter_mut().enumerate().zip(layout.panes()).find_map({
            let panes_state = &self.state;
            move |((pane_index, (_, pane)), pane_layout)| {
                let resize_directions =
                    pane.get_pane_resize_directions(pane_layout, panes_state.cursor_position);

                if !resize_directions.is_none() {
                    Some(Gesture::ResizePane {
                        pending: true,
                        pane_index,
                        grab_state: GrabStateResize {
                            grab_element_position: pane.state.position,
                            grab_element_size: Into::<[f32; 2]>::into(pane_layout.bounds().size()).into(),
                            grab_mouse_position: panes_state.cursor_position,
                        },
                        directions: resize_directions,
                    })
                } else {
                    None
                }
            }
        });
    }
}

impl<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer, C: 'a + FloatingPanesBehaviour<'a, M, T, R>> Widget<M, T, R>
    for FloatingPanes<'a, M, T, R, C>
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, tree: &mut Tree, renderer: &R, limits: &Limits) -> Node {
        let limits = limits
            .max_width(self.extents[0])
            .max_height(self.extents[1])
            .width(self.width)
            .height(self.height);

        Node::with_children(
            Size::new(self.extents[0] as f32, self.extents[1] as f32),
            self.children
                .iter()
                .map(|(_, child)| {
                    child
                        .element_tree
                        .as_widget()
                        .layout(tree, renderer, &limits)
                        .move_to(child.state.position.into_array())
                })
                .collect::<Vec<_>>(),
        )
        .move_to(self.state.panes_offset.into_array())
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut R,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        <R as WidgetRenderer>::draw(renderer, self, tree, theme, style, layout, cursor, viewport)
    }

    // fn hash_layout(&self, state: &mut Hasher) {
    //     struct Marker;
    //     std::any::TypeId::of::<Marker>().hash(state);

    //     self.state.hash(state);
    //     self.width.hash(state);
    //     self.height.hash(state);
    //     self.extents.hash(state);

    //     for (_, child) in &self.children {
    //         child.state.hash(state);
    //         child.element_tree.hash_layout(state);
    //     }

    //     C::hash_panes(&self, state);
    // }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        let layout: FloatingPanesLayout = layout.into();

        if C::on_event(self, tree, event.clone(), layout, cursor, renderer, clipboard, shell, viewport)
            == Status::Captured
        {
            return Status::Captured;
        }

        // Set to `true`, if the event should not be propagated to child panes.
        let mut status = Status::Ignored;

        // TODO: Make it possible to bind keyboard/mouse buttons to pan regardless of whether the
        // cursor is on top of a pane.
        match &event {
            Event::Mouse(mouse::Event::CursorMoved { position: Point { x, y } }) => {
                self.state.cursor_position = [*x, *y].into();

                match self.state.gesture.clone() {
                    Some(Gesture::GrabPane { pane_index, grab_state }) => {
                        if let Some((_, pane)) = self.children.get_index_mut(pane_index) {
                            pane.state.position = self.state.cursor_position.as_::<f32>()
                                + grab_state.grab_element_position
                                - grab_state.grab_mouse_position;
                            shell.publish((self.on_layout_change)());
                        }
                    }
                    Some(Gesture::GrabBackground(grab_state)) => {
                        self.state.panes_offset = self.state.cursor_position.as_::<f32>()
                            + grab_state.grab_element_position
                            - grab_state.grab_mouse_position;
                        shell.publish((self.on_layout_change)());
                    }
                    Some(Gesture::ResizePane { pending: false, pane_index, grab_state, directions }) => {
                        if let Some((_, pane)) = self.children.get_index_mut(pane_index) {
                            for component_index in 0..2 {
                                if let FloatingPaneLength::Fixed(pane_size) =
                                    &mut pane.state.size[component_index]
                                {
                                    let original_element_size = grab_state.grab_element_size[component_index];
                                    let original_element_position =
                                        grab_state.grab_element_position[component_index];
                                    let original_mouse_position =
                                        grab_state.grab_mouse_position[component_index];
                                    let current_mouse_position =
                                        self.state.cursor_position[component_index] as f32;
                                    let mouse_offset = current_mouse_position - original_mouse_position;
                                    let new_element_size: f32 = std::cmp::max(
                                        OrderedFloat(
                                            original_element_size
                                                + mouse_offset
                                                    * match directions[component_index] {
                                                        PaneResizeDirection::None => 0.0,
                                                        PaneResizeDirection::Negative => -1.0,
                                                        PaneResizeDirection::Positive => 1.0,
                                                    },
                                        ),
                                        OrderedFloat(pane.min_size[component_index]),
                                    )
                                    .into();
                                    let size_delta = new_element_size - original_element_size;

                                    pane.state.position[component_index] = original_element_position
                                        + size_delta
                                            * match directions[component_index] {
                                                PaneResizeDirection::None | PaneResizeDirection::Positive => {
                                                    0.0
                                                }
                                                PaneResizeDirection::Negative => -1.0,
                                            };
                                    *pane_size = new_element_size;
                                }
                            }

                            shell.publish((self.on_layout_change)());
                        }
                    }
                    _ => {
                        self.update_pending_gestures(layout, shell);
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                self.state.gesture = self.children.iter_mut().enumerate().find_map({
                    let panes_state = &self.state;
                    move |(pane_index, (_, pane))| {
                        if let Some(Gesture::ResizePane { pane_index, grab_state, directions, .. }) =
                            panes_state.gesture.clone()
                        {
                            Some(Gesture::ResizePane { pending: false, pane_index, grab_state, directions })
                        } else {
                            if pane.state.title_bar_status == InteractionStatus::Hovered {
                                pane.state.title_bar_status = InteractionStatus::Focused;

                                Some(Gesture::GrabPane {
                                    pane_index,
                                    grab_state: GrabStateMove {
                                        grab_mouse_position: panes_state.cursor_position,
                                        grab_element_position: pane.state.position,
                                    },
                                })
                            } else {
                                None
                            }
                        }
                    }
                });

                if self.state.gesture.is_none() {
                    let mouse_on_top_of_pane = layout.panes().any({
                        let panes_state = &self.state;
                        let cursor_point: Point = panes_state.cursor_position.into_array().into();
                        move |pane_layout| pane_layout.bounds().contains(cursor_point)
                    });

                    if !mouse_on_top_of_pane {
                        self.state.gesture = Some(Gesture::GrabBackground(GrabStateMove {
                            grab_mouse_position: self.state.cursor_position,
                            grab_element_position: self.state.panes_offset,
                        }));
                    }
                } else {
                    shell.publish((self.on_layout_change)());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.update_pending_gestures(layout, shell);
            }
            _ => (),
        }

        if status == Status::Ignored {
            status = self.children.iter_mut().zip(layout.panes()).fold(
                Status::Ignored,
                |status, ((_, pane), pane_layout)| {
                    status.merge(pane.element_tree.as_widget_mut().on_event(
                        tree,
                        event.clone(),
                        pane_layout.into(),
                        cursor,
                        renderer,
                        clipboard,
                        shell,
                        viewport,
                    ))
                },
            );
        }

        status
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &R,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, T, R>> {
        C::overlay(self, state, layout.into(), renderer, translation)
    }
}

impl<'a, M: 'a, T: 'a, R: 'a + WidgetRenderer, C: 'a + FloatingPanesBehaviour<'a, M, T, R>>
    From<FloatingPanes<'a, M, T, R, C>> for Element<'a, M, T, R>
{
    fn from(other: FloatingPanes<'a, M, T, R, C>) -> Self {
        Element::new(other)
    }
}

/// Good practice: Rendering is made to be generic over the backend using this trait, which
/// is to be implemented on the specific `Renderer`.
pub trait WidgetRenderer:
    margin::WidgetRenderer
    + iced_core::Renderer
    + iced_core::text::Renderer
    // + iced_runtime::column::Renderer
    // + iced_runtime::widget::container::Renderer
    + Sized
{
    // type StyleFloatingPane: StyleFloatingPaneBounds<Self>;
    // type StyleFloatingPanes;

    fn draw<'a, M: 'a, T: 'a, C: 'a + FloatingPanesBehaviour<'a, M, T, Self>>(
        &mut self,
        element: &FloatingPanes<'a, M, T, Self, C>,
        tree: &Tree,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    );
}

impl<R> WidgetRenderer for R
where R: margin::WidgetRenderer + iced_core::Renderer + iced_core::text::Renderer + Sized
{
    // type StyleFloatingPane = Box<dyn FloatingPaneStyleSheet>;
    // type StyleFloatingPanes = Box<dyn FloatingPanesStyleSheet>;

    fn draw<'a, M: 'a, T: 'a, C: 'a + FloatingPanesBehaviour<'a, M, T, Self>>(
        &mut self,
        element: &FloatingPanes<'a, M, T, Self, C>,
        tree: &Tree,
        theme: &T,
        style: &iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let mut mouse_interaction = element
            .state
            .gesture
            .as_ref()
            .map(Gesture::get_mouse_interaction)
            .unwrap_or(Interaction::default());

        let background_primitive = PrimitiveEnum::Quad(Quad {
            bounds: Rectangle::new(Point::ORIGIN, layout.bounds().size()),
            // background: Background::Color(
            //     element
            //         .style
            //         .as_ref()
            //         .map(|style| style.style().background_color)
            //         .unwrap_or(Color::TRANSPARENT),
            // ),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        });

        let ContentDrawResult {
            override_parent_cursor,
            // output: (panes_primitive, content_mouse_interaction),
        } = C::draw_panes(element, tree, self, theme, style, layout.into(), cursor, viewport);

        // if override_parent_cursor {
        //     mouse_interaction = content_mouse_interaction;
        // } else {
        //     mouse_interaction = std::cmp::max(mouse_interaction, content_mouse_interaction);
        // };

        // let primitives = vec![background_primitive, panes_primitive];

        // (background_primitive /* Originally `primitives` */, mouse_interaction)
    }
}

/*
#[derive(Clone, Copy, Debug, Default)]
pub struct FloatingPaneStyle {
    pub title_background_color: Color,
    pub title_text_color: Color,
    pub body_background_color: Color,
}

pub trait StyleFloatingPaneBounds<R: WidgetRenderer> {
    fn root_container_style(
        &self,
        title_bar_status: InteractionStatus,
    ) -> <R as iced_runtime::widget::container::StyleSheet>::Style;
    fn content_container_style(
        &self,
        title_bar_status: InteractionStatus,
    ) -> <R as iced_runtime::widget::container::StyleSheet>::Style;
}

pub trait FloatingPaneStyleSheet {
    fn style(&self, title_bar_status: InteractionStatus) -> FloatingPaneStyle;
}

impl<B> StyleFloatingPaneBounds<iced_graphics::Renderer<B>> for Box<dyn FloatingPaneStyleSheet>
where B: Backend + iced_graphics::backend::Text
{
    fn root_container_style(
        &self,
        title_bar_status: InteractionStatus,
    ) -> Box<(dyn iced::widget::container::StyleSheet + 'static)> {
        struct StyleSheet(FloatingPaneStyle);

        impl iced::widget::container::StyleSheet for StyleSheet {
            type Style = FloatingPaneStyle;

            fn appearance(&self, style: &Self::Style) -> iced::widget::container::Appearance {
                iced::widget::container::Appearance {
                    background: Some(Background::Color(self.0.title_background_color)),
                    text_color: Some(self.0.title_text_color),
                    ..Default::default()
                }
            }
        }

        Box::new(StyleSheet(self.style(title_bar_status)))
    }

    fn content_container_style(
        &self,
        title_bar_status: InteractionStatus,
    ) -> Box<(dyn iced::widget::container::StyleSheet + 'static)> {
        struct StyleSheet(FloatingPaneStyle);

        impl iced::widget::container::StyleSheet for StyleSheet {
            type Style = FloatingPaneStyle;

            fn appearance(&self, style: &Self::Style) -> iced::widget::container::Appearance {
                iced::widget::container::Appearance {
                    background: Some(Background::Color(self.0.body_background_color)),
                    ..Default::default()
                }
            }
        }

        Box::new(StyleSheet(self.style(title_bar_status)))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FloatingPanesStyle {
    pub background_color: Color,
}

pub trait FloatingPanesStyleSheet {
    fn style(&self) -> FloatingPanesStyle;
}
    */

typed_layout! {
    type_name: FloatingPanes,
}

typed_layout! {
    type_name: FloatingPane,
    traverse: [
        {
            parent_type_name: FloatingPanes,
            fn_name: pane_with_index,
            fn_args: [pane_index: usize],
            fn: |parent: Layout<'a>, pane_index: usize| {
                parent.children().nth(pane_index).unwrap()
            },
        },
    ],
    children_of: {
        parent_type_name: FloatingPanes,
        fn_name: panes,
    },
}

typed_layout! {
    type_name: FloatingPaneContent,
    traverse: [
        {
            parent_type_name: FloatingPane,
            fn_name: content,
            fn_args: [],
            fn: |parent: Layout<'a>| {
                parent.children().nth(0).unwrap().children().nth(1).unwrap().children().nth(0).unwrap()
            },
        },
    ],
}
