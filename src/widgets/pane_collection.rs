use std::hash::Hash;
use iced_native::{self, Size, Length, Point, Hasher, Event, Clipboard};
use iced_native::{mouse, overlay, Element};
use iced_native::widget::Widget;
use iced_native::layout::{Layout, Limits, Node};
use iced_graphics::{self, Backend, Defaults, Primitive};

pub struct FloatingPane<'a, M: 'a, R: WidgetRenderer> {
    pub element: Element<'a, M, R>,
    pub position: [i32; 2],
    pub title: Option<&'a str>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> FloatingPane<'a, M, R> {
    pub fn new(element: impl Into<Element<'a, M, R>>) -> Self {
        Self {
            element: element.into(),
            position: Default::default(),
            title: Default::default(),
        }
    }

    pub fn position(mut self, position: [i32; 2]) -> Self {
        self.position = position;
        self
    }

    pub fn title(mut self, title: Option<&'a str>) -> Self {
        self.title = title;
        self
    }
}

pub struct FloatingPanes<'a, M: 'a, R: 'a + WidgetRenderer> {
    width: Length,
    height: Length,
    max_width: u32,
    max_height: u32,
    children: Vec<FloatingPane<'a, M, R>>,
}

impl<'a, M: 'a, R: 'a + WidgetRenderer> FloatingPanes<'a, M, R> {
    pub fn new() -> Self {
        Self {
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
                    let mut node = child.element.layout(renderer, &limits);

                    node.move_to(Point::new(
                        child.position[0] as f32,
                        child.position[1] as f32,
                    ));

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
            child.element.hash_layout(state);
            child.position.hash(state);
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
        self.children.iter_mut().zip(layout.children()).for_each(
            |(child, layout)| {
                child.element.on_event(
                    event.clone(),
                    layout,
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
            .filter_map(|(child, layout)| child.element.overlay(layout))
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
pub trait WidgetRenderer: iced_native::Renderer + Sized {
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
    B: Backend,
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
                            child.element.draw(self, defaults, layout, cursor_position);

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
