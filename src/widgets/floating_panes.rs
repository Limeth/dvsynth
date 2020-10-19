use std::hash::Hash;
use iced_native::{self, Size, Length, Point, Hasher, Event, Clipboard, Column, Text};
use iced_native::{overlay, Element};
use iced_native::mouse::{self, Event as MouseEvent, Button as MouseButton};
use iced_native::widget::{Widget, Container};
use iced_native::layout::{Layout, Limits, Node};
use iced_graphics::{self, Backend, Defaults, Primitive};
use vek::Vec2;
use ordered_float::OrderedFloat;
use super::*;

pub struct FloatingPaneBuilder<'a, M: 'a, R: 'a + WidgetRenderer> {
    pub state: &'a mut FloatingPaneState,
    pub element: Element<'a, M, R>,
    pub title: Option<&'a str>,
    pub title_style: Option<<R as iced_native::widget::container::Renderer>::Style>,
    pub title_size: Option<u16>,
    pub title_margin: Spacing,
    pub pane_style: Option<<R as iced_native::widget::container::Renderer>::Style>,
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
            title_style: Default::default(),
            title_size: Default::default(),
            title_margin: Default::default(),
            pane_style: Default::default(),
        }
    }

    pub fn title(mut self, title: Option<&'a str>) -> Self {
        self.title = title;
        self
    }

    pub fn title_style<T>(mut self, title_style: Option<T>) -> Self
            where T: Into<<R as iced_native::widget::container::Renderer>::Style> {
        self.title_style = title_style.map(Into::into);
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

    pub fn pane_style<T>(mut self, pane_style: Option<T>) -> Self
            where T: Into<<R as iced_native::widget::container::Renderer>::Style> {
        self.pane_style = pane_style.map(Into::into);
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

                    let margin = Margin::new(text, self.title_margin.clone());
                    let mut container = Container::new(margin);

                    if let Some(title_style) = self.title_style.take() {
                        container = container.style(title_style);
                    }

                    column = column.push(container);
                }

                let mut container = Container::new(column.push(self.element));

                if let Some(pane_style) = self.pane_style.take() {
                    container = container.style(pane_style);
                }

                container.into() // Container { Column [ title, element ] }
            },
        }
    }
}

#[derive(Default, Debug)]
pub struct GrabState {
    pub grab_pane_position: Vec2<f32>,
    pub grab_mouse_position: Vec2<f32>,
}

impl Hash for GrabState {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.grab_pane_position.map(OrderedFloat::from).as_slice().hash(state);
        self.grab_mouse_position.map(OrderedFloat::from).as_slice().hash(state);
    }
}

#[derive(Default, Debug)]
pub struct FloatingPaneState {
    pub position: Vec2<f32>,
    pub grab_state: Option<GrabState>,
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
}

pub struct FloatingPanes<'a, M: 'a, R: 'a + WidgetRenderer> {
    state: &'a mut FloatingPanesState,
    width: Length,
    height: Length,
    max_width: u32, // TODO
    max_height: u32, // TODO combine
    children: Vec<FloatingPane<'a, M, R>>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> FloatingPanes<'a, M, R> {
    pub fn new(state: &'a mut FloatingPanesState) -> Self {
        Self {
            state,
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: u32::MAX,
            max_height: u32::MAX,
            children: Vec::new(),
        }
    }

    /// Sets the width of the [`Row`].
    ///
    /// [`Row`]: struct.Row.html
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`Row`].
    ///
    /// [`Row`]: struct.Row.html
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the maximum width of the [`Row`].
    ///
    /// [`Row`]: struct.Row.html
    pub fn max_width(mut self, max_width: u32) -> Self {
        self.max_width = max_width;
        self
    }

    /// Sets the maximum height of the [`Row`].
    ///
    /// [`Row`]: struct.Row.html
    pub fn max_height(mut self, max_height: u32) -> Self {
        self.max_height = max_height;
        self
    }

    /// Adds an [`Element`] to the [`Row`].
    ///
    /// [`Element`]: ../struct.Element.html
    /// [`Row`]: struct.Row.html
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
            .max_width(self.max_width)
            .max_height(self.max_height)
            .width(self.width)
            .height(self.height);

        Node::with_children(
            Size::new(self.max_width as f32, self.max_height as f32), // FIXME
            self.children.iter()
                .map(|child| {
                    let mut node = child.element_tree.layout(renderer, &limits);

                    node.move_to(child.state.position.into_array().into());

                    node
                })
                .collect::<Vec<_>>(),
        )
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &<R as iced_native::Renderer>::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> <R as iced_native::Renderer>::Output {
        <R as WidgetRenderer>::draw(renderer, defaults, &self.children, layout, cursor_position)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.height.hash(state);
        self.max_width.hash(state);
        self.max_height.hash(state);

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

        self.children.iter_mut().zip(layout.children()).for_each(
            |(child, pane_layout)| {
                let child_layout = pane_layout.children().nth(0).expect("Invalid UI state.");
                let child_layout = child_layout.children().nth(1).expect("Invalid UI state.");
                let is_on_title = pane_layout.bounds().contains(panes_state.cursor_position.into_array().into())
                    && !child_layout.bounds().contains(panes_state.cursor_position.into_array().into());

                match &event {
                    Event::Mouse(MouseEvent::CursorMoved { x, y }) => {
                        if let Some(grab_state) = &child.state.grab_state {
                            child.state.position = panes_state.cursor_position.as_::<f32>()
                                + grab_state.grab_pane_position
                                - grab_state.grab_mouse_position;
                        }
                    }
                    Event::Mouse(MouseEvent::ButtonPressed(MouseButton::Left)) if is_on_title => {
                        child.state.grab_state = Some(GrabState {
                            grab_mouse_position: panes_state.cursor_position,
                            grab_pane_position: child.state.position,
                        });
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
                )
            },
        );
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
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        children: &[FloatingPane<'_, M, Self>],
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> Self::Output;
}

impl<B> WidgetRenderer for iced_graphics::Renderer<B>
where
    B: Backend + iced_graphics::backend::Text,
{
    fn draw<Message>(
        &mut self,
        defaults: &Self::Defaults,
        content: &[FloatingPane<'_, Message, Self>],
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> Self::Output {
        let mut mouse_interaction = mouse::Interaction::default();

        (
            Primitive::Group {
                primitives: content
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
                    .collect(),
            },
            mouse_interaction,
        )
    }
}
