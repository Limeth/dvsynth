use std::hash::Hash;
use iced_native::{self, Size, Length, Point, Hasher, Event, Clipboard, Column, Text};
use iced_native::{overlay, Element};
use iced_native::mouse::{self, Event as MouseEvent, Button as MouseButton};
use iced_native::widget::{Widget, Container};
use iced_native::layout::{Layout, Limits, Node};
use iced_graphics::{self, Backend, Defaults, Primitive, Background, Color, Rectangle};
use vek::Vec2;
use ordered_float::OrderedFloat;
use super::*;

pub struct FloatingPaneBuilder<'a, M: 'a, R: 'a + WidgetRenderer> {
    pub state: &'a mut FloatingPaneState,
    pub element: Element<'a, M, R>,
    pub title: Option<&'a str>,
    pub title_size: Option<u16>,
    pub title_margin: Spacing,
    pub style: Option<<R as WidgetRenderer>::StyleFloatingPane>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> FloatingPaneBuilder<'a, M, R> {
    pub fn new(
        state: &'a mut FloatingPaneState,
        element: impl Into<Element<'a, M, R>>,
    ) -> Self {
        Self {
            state,
            element: element.into(),
            title: Default::default(),
            title_size: Default::default(),
            title_margin: Default::default(),
            style: Default::default(),
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

    pub fn style<T>(mut self, style: Option<T>) -> Self
            where T: Into<<R as WidgetRenderer>::StyleFloatingPane> {
        self.style = style.map(Into::into);
        self
    }

    pub fn build(mut self) -> FloatingPane<'a, M, R> {
        FloatingPane {
            state: self.state,
            element_tree: {
                let mut column = Column::<M, R>::new();

                if let Some(title) = self.title.take() {
                    let mut text = Text::new(title.to_string());

                    if let Some(title_size) = self.title_size.take() {
                        text = text.size(title_size);
                    }

                    column = column.push(Margin::new(text, self.title_margin.clone()));
                }

                let mut element_container = Container::new(self.element);

                if let Some(style) = self.style.as_ref() {
                    element_container = element_container.style(style.content_container_style());
                }

                let mut container = Container::new(column.push(element_container));

                if let Some(style) = self.style.as_ref() {
                    container = container.style(style.root_container_style());
                }

                container.into() // Container { Column [ title, Container { element } ] }
            },
        }
    }
}

#[derive(Default, Debug)]
pub struct GrabState {
    pub grab_element_position: Vec2<f32>,
    pub grab_mouse_position: Vec2<f32>,
}

impl Hash for GrabState {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.grab_element_position.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_mouse_position.map(OrderedFloat::from).as_slice().hash(state);
    }
}

#[derive(Default, Debug)]
pub struct FloatingPaneState {
    pub position: Vec2<f32>,
    pub grab_state: Option<GrabState>, // to move this pane around
}

impl Hash for FloatingPaneState {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.position.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_state.hash(state);
    }
}

impl FloatingPaneState {
    pub fn with_position(position: impl Into<Vec2<f32>>) -> Self {
        Self {
            position: position.into(),
            grab_state: Default::default(),
        }
    }
}

pub struct FloatingPane<'a, M: 'a, R: 'a + WidgetRenderer> {
    state: &'a mut FloatingPaneState,
    element_tree: Element<'a, M, R>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> FloatingPane<'a, M, R> {
    pub fn builder(
        state: &'a mut FloatingPaneState,
        element: impl Into<Element<'a, M, R>>,
    ) -> FloatingPaneBuilder<'a, M, R> {
        FloatingPaneBuilder::new(state, element)
    }
}

#[derive(Default, Debug)]
pub struct FloatingPanesState {
    cursor_position: Vec2<f32>,
    panes_offset: Vec2<f32>, // the vector to offset all floating panes by
    grab_state: Option<GrabState>, // to pan across the pane view (via panes_offset)
}

impl Hash for FloatingPanesState {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.panes_offset.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_state.hash(state);
    }
}

pub struct FloatingPanes<'a, M: 'a, R: 'a + WidgetRenderer> {
    state: &'a mut FloatingPanesState,
    width: Length,
    height: Length,
    extents: Vec2<u32>,
    style: Option<<R as WidgetRenderer>::StyleFloatingPanes>,
    children: Vec<FloatingPane<'a, M, R>>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> FloatingPanes<'a, M, R> {
    pub fn new(state: &'a mut FloatingPanesState) -> Self {
        Self {
            state,
            width: Length::Shrink,
            height: Length::Shrink,
            extents: [u32::MAX, u32::MAX].into(),
            style: None,
            children: Vec::new(),
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

    pub fn style<T>(mut self, style: T) -> Self
            where T: Into<<R as WidgetRenderer>::StyleFloatingPanes> {
        self.style = Some(style.into());
        self
    }

    pub fn push(mut self, child: FloatingPane<'a, M, R>) -> Self {
        self.children.push(child.into());
        self
    }
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> Widget<M, R> for FloatingPanes<'a, M, R> {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &R, limits: &Limits) -> Node {
        let limits = limits
            .max_width(self.extents[0])
            .max_height(self.extents[1])
            .width(self.width)
            .height(self.height);
        let mut node = Node::with_children(
            Size::new(self.extents[0] as f32, self.extents[1] as f32),
            self.children.iter()
                .map(|child| {
                    let mut node = child.element_tree.layout(renderer, &limits);

                    node.move_to(child.state.position.into_array().into());

                    node
                })
                .collect::<Vec<_>>(),
        );

        node.move_to(self.state.panes_offset.into_array().into());

        node
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

        self.state.hash(state);
        self.width.hash(state);
        self.height.hash(state);
        self.extents.hash(state);

        for child in &self.children {
            child.state.hash(state);
            child.element_tree.hash_layout(state);
        }
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
        if let Event::Mouse(MouseEvent::CursorMoved { x, y }) = &event {
            self.state.cursor_position = [*x, *y].into();
        }

        let panes_state = &self.state;
        // only assigned when LMB is pressed
        let cursor_on_pane = self.children.iter_mut().zip(layout.children()).map(
            |(child, pane_layout)| {
                let mut cursor_on_pane = false; // only assigned when LMB is pressed

                match &event {
                    Event::Mouse(MouseEvent::CursorMoved { .. }) => {
                        if let Some(grab_state) = &child.state.grab_state {
                            child.state.position = panes_state.cursor_position.as_::<f32>()
                                + grab_state.grab_element_position
                                - grab_state.grab_mouse_position;
                        }
                    }
                    Event::Mouse(MouseEvent::ButtonPressed(MouseButton::Left)) => {
                        let child_layout = pane_layout.children().nth(0).expect("Invalid UI state.");
                        let child_layout = child_layout.children().nth(1).expect("Invalid UI state.");
                        cursor_on_pane = pane_layout.bounds().contains(panes_state.cursor_position.into_array().into());
                        let cursor_on_title = cursor_on_pane && !child_layout.bounds().contains(panes_state.cursor_position.into_array().into());

                        if cursor_on_title {
                            child.state.grab_state = Some(GrabState {
                                grab_mouse_position: panes_state.cursor_position,
                                grab_element_position: child.state.position,
                            });
                        }
                    }
                    Event::Mouse(MouseEvent::ButtonReleased(MouseButton::Left)) => {
                        child.state.grab_state = None;
                    }
                    _ => ()
                }

                child.element_tree.on_event(
                    event.clone(),
                    pane_layout,
                    cursor_position,
                    messages,
                    renderer,
                    clipboard,
                );

                cursor_on_pane
            },
        ).fold(false, |acc, new| acc || new);

        // TODO: Make it possible to bind keyboard/mouse buttons to pan regardless of whether the
        // cursor is on top of a pane.
        match event {
            Event::Mouse(MouseEvent::CursorMoved { .. }) => {
                if let Some(grab_state) = &self.state.grab_state {
                    self.state.panes_offset = panes_state.cursor_position.as_::<f32>()
                        + grab_state.grab_element_position
                        - grab_state.grab_mouse_position;
                }
            }
            Event::Mouse(MouseEvent::ButtonPressed(MouseButton::Left)) if !cursor_on_pane => {
                self.state.grab_state = Some(GrabState {
                    grab_mouse_position: self.state.cursor_position,
                    grab_element_position: self.state.panes_offset,
                });
            }
            Event::Mouse(MouseEvent::ButtonReleased(MouseButton::Left)) => {
                self.state.grab_state = None;
            }
            _ => ()
        }
    }

    fn overlay(
        &mut self,
        layout: Layout<'_>
    ) -> Option<overlay::Element<'_, M, R>> {
        self.children
            .iter_mut()
            .zip(layout.children())
            .filter_map(|(child, layout)| child.element_tree.overlay(layout))
            .next()
    }
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> From<FloatingPanes<'a, M, R>> for Element<'a, M, R> {
    fn from(other: FloatingPanes<'a, M, R>) -> Self {
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
    type StyleFloatingPane: StyleFloatingPaneBounds<Self>;
    type StyleFloatingPanes;

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &FloatingPanes<'_, M, Self>,
    ) -> Self::Output;
}

impl<B> WidgetRenderer for iced_graphics::Renderer<B>
where
    B: Backend + iced_graphics::backend::Text,
{
    type StyleFloatingPane = Box<dyn FloatingPaneStyleSheet>;
    type StyleFloatingPanes = Box<dyn FloatingPanesStyleSheet>;

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        element: &FloatingPanes<'_, M, Self>,
    ) -> Self::Output {
        let grabbing = element.state.grab_state.is_some()
            || element.children.iter().any(|floating_pane| floating_pane.state.grab_state.is_some());

        let mut mouse_interaction = if grabbing {
            mouse::Interaction::Grabbing
        } else {
            mouse::Interaction::default()
        };

        let mut primitives = Vec::new();

        primitives.push(Primitive::Quad {
            bounds: Rectangle::new(Point::ORIGIN, layout.bounds().size()),
            background: Background::Color(
                element.style.as_ref()
                    .map(|style| style.style().background_color)
                    .unwrap_or(Color::TRANSPARENT)
            ),
            border_radius: 0,
            border_width: 0,
            border_color: Color::BLACK,
        });

        primitives.extend(
            element.children
                .iter()
                .zip(layout.children())
                .map(|(child, layout)| {
                    let (primitive, new_mouse_interaction) =
                        child.element_tree.draw(self, defaults, layout, cursor_position);

                    if new_mouse_interaction > mouse_interaction {
                        mouse_interaction = new_mouse_interaction;
                    }

                    primitive
                })
        );

        (Primitive::Group { primitives }, mouse_interaction)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FloatingPaneStyle {
    pub title_background_color: Color,
    pub title_text_color: Color,
    pub body_background_color: Color,
}

pub trait StyleFloatingPaneBounds<R: WidgetRenderer> {
    fn root_container_style(&self) -> <R as iced_native::widget::container::Renderer>::Style;
    fn content_container_style(&self) -> <R as iced_native::widget::container::Renderer>::Style;
}

pub trait FloatingPaneStyleSheet {
    fn style(&self) -> FloatingPaneStyle;
}

impl<B> StyleFloatingPaneBounds<iced_graphics::Renderer<B>> for Box<dyn FloatingPaneStyleSheet>
where B: Backend + iced_graphics::backend::Text,
{
    fn root_container_style(&self) -> Box<(dyn iced::container::StyleSheet + 'static)> {
        struct StyleSheet(FloatingPaneStyle);

        impl iced::container::StyleSheet for StyleSheet {
            fn style(&self) -> iced::container::Style {
                iced::container::Style {
                    background: Some(Background::Color(self.0.title_background_color)),
                    text_color: Some(self.0.title_text_color),
                    ..Default::default()
                }
            }
        }

        Box::new(StyleSheet(self.style()))
    }

    fn content_container_style(&self) -> Box<(dyn iced::container::StyleSheet + 'static)> {
        struct StyleSheet(FloatingPaneStyle);

        impl iced::container::StyleSheet for StyleSheet {
            fn style(&self) -> iced::container::Style {
                iced::container::Style {
                    background: Some(Background::Color(self.0.body_background_color)),
                    ..Default::default()
                }
            }
        }

        Box::new(StyleSheet(self.style()))

    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FloatingPanesStyle {
    pub background_color: Color,
}

pub trait FloatingPanesStyleSheet {
    fn style(&self) -> FloatingPanesStyle;
}
