use std::any::TypeId;
use std::{f64, iter};

use iced::Vector;
use itertools::chain;
use tracing::{Span, trace_span};
use xilem::Vec2;
use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    EventCtx, HasProperty, PointerButtonEvent, PointerEvent, PointerUpdate, QueryCtx,
};
use xilem::masonry::kurbo::{BezPath, Rect, RoundedRect};
use xilem::masonry::peniko::FontData;
use xilem::masonry::peniko::color::AlphaColor;
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{Affine, Line, Point, Size, Stroke};

use xilem::masonry::core::{
    AccessCtx, Axis, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use xilem::masonry::properties::types::Length;
use xilem::masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use xilem::masonry::properties::{Background, BorderColor, BorderWidth, CornerRadius, Padding};
use xilem::masonry::theme::DEFAULT_GAP;
use xilem::masonry::util::{debug_panic, fill, include_screenshot, stroke};
use xilem::masonry::widgets::IndexedStack;
use xilem::view::PointerButton;
use xilem::winit::window::CursorIcon;

use crate::node::ChannelDirection;

/// A container with either horizontal or vertical layout.
///
/// This widget is the foundation of most layouts, and is highly configurable.
///
/// The flex model used by Masonry has different behaviour than you might be familiar with from the web.
/// Only children which have an explicit flex factor, set by the first parameter of
/// [`FlexParams::new`](FlexParams::new) being `Some`, will share remaining space flexibly.
/// Children which do not have an explicit flex factor set will be laid out as their natural size.
/// For some widgets (such as [`TextInput`](crate::widgets::TextInput)), this will be
/// all the space made available to the flex (in at least one axis).
/// In the web model, this is equivalent to the default `flex` being `none` (on the web, this is instead `auto`).
/// This can lead to surprising results, including later siblings of the expanded child being pushed off-screen.
/// A general rule of thumb is to set a flex factor on all "large" children in the flex axis, especially
/// portals, sized boxes, and text inputs (in horizontal flex areas).
/// That is, any item which needs to shrink to fit within the viewport should have a
/// flex factor set.
///
/// There is also no support for flex grow or flex shrink; instead, each flexible child takes up
/// the proportion of remaining space (after all "non-flex" children are laid out) specified
/// by its flex factor.
/// In the web flex algorithm, if a widget cannot expand to its target flex size, that remaining space is distributed
/// to the other sibling flex widgets recursively.
/// However, this widget does not implement this behaviour at the moment, as it uses a single-pass layout algorithm.
/// Instead, if a flex child of this widget does not expand to the target size provided by this parent, the difference is distributed
/// to the space between widgets according to this widget's [`MainAxisAlignment`](Flex::set_main_axis_alignment).
///
#[doc = include_screenshot!("flex_col_main_axis_spaceAround.png", "Flex column with multiple labels.")]
pub struct FloatingPanes {
    // direction: Axis,
    // cross_alignment: CrossAxisAlignment,
    // main_alignment: MainAxisAlignment,
    // fill_major_axis: bool,
    // gap: Length,
    children: Vec<Child>,
    gesture: Option<Gesture>,
    /// Offset of all child panes in the local coordinates of this `FloatingPanes` widget.
    children_offset: Vec2,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct FloatingPaneParams {
    pub title: String,
    /// Position of the pane in the local coordinates of the `FloatingPanes` widget.
    pub position_local: Point,
}

// TODO: Make generic over widget types?
pub struct Child {
    pub content: WidgetPod<dyn Widget>,
    pub channels_in: Vec<WidgetPod<dyn Widget>>,
    pub channels_out: Vec<WidgetPod<dyn Widget>>,
    pub params: FloatingPaneParams,
    pub calculated_size: Size,
    // TODO: Should this be part of FloatingPaneParams?
    // extra_data: Point,
}

impl Child {
    pub fn layout_origin(&self, children_offset: Vec2) -> Point {
        self.params.position_local + children_offset
    }

    pub fn layout_bounding_rect(&self, children_offset: Vec2) -> Rect {
        Rect::from_origin_size(self.layout_origin(children_offset), self.calculated_size)
    }

    pub fn get_channels(&self, direction: ChannelDirection) -> &Vec<WidgetPod<dyn Widget>> {
        match direction {
            ChannelDirection::In => &self.channels_in,
            ChannelDirection::Out => &self.channels_out,
        }
    }

    pub fn get_channels_mut(&mut self, direction: ChannelDirection) -> &mut Vec<WidgetPod<dyn Widget>> {
        match direction {
            ChannelDirection::In => &mut self.channels_in,
            ChannelDirection::Out => &mut self.channels_out,
        }
    }
}

impl Default for FloatingPanes {
    fn default() -> Self {
        FloatingPanes::new()
    }
}

// --- MARK: IMPL FLEX
impl FloatingPanes {
    /// Create a new `Flex` oriented along the provided axis.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gesture: None,
            children_offset: Vec2::ZERO,
            // direction: axis,
            // cross_alignment: CrossAxisAlignment::Center,
            // main_alignment: MainAxisAlignment::Start,
            // fill_major_axis: false,
            // gap: DEFAULT_GAP,
        }
    }

    // /// Builder-style method for specifying the children's [`CrossAxisAlignment`].
    // pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
    //     self.cross_alignment = alignment;
    //     self
    // }

    // /// Builder-style method for specifying the children's [`MainAxisAlignment`].
    // pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
    //     self.main_alignment = alignment;
    //     self
    // }

    // /// Builder-style method for setting whether the container must expand
    // /// to fill the available space on its main axis.
    // pub fn must_fill_main_axis(mut self, fill: bool) -> Self {
    //     self.fill_major_axis = fill;
    //     self
    // }

    // /// Builder-style method for setting a gap along the
    // /// major axis between any two elements in logical pixels.
    // ///
    // /// By default this is [`DEFAULT_GAP`].
    // ///
    // /// Equivalent to the css [gap] property.
    // ///
    // /// This gap is between any two children, including spacers.
    // /// As such, when adding a spacer, you add both the spacer's size (or computed flex size)
    // /// and the gap between the spacer and its neighbors.
    // /// As such, if you're adding lots of spacers to a flex parent, you may want to set
    // /// its gap to zero to make the layout more predictable.
    // ///
    // /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    // // TODO: Semantics - should this include fixed spacers?
    // pub fn with_gap(mut self, gap: Length) -> Self {
    //     self.gap = gap;
    //     self
    // }

    /// Builder-style variant of [`Flex::add_child`].
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(
        mut self,
        content: NewWidget<impl Widget + ?Sized>,
        channels_in: Vec<NewWidget<impl Widget + ?Sized>>,
        channels_out: Vec<NewWidget<impl Widget + ?Sized>>,
        params: FloatingPaneParams,
    ) -> Self {
        let child = Child {
            content: content.erased().to_pod(),
            channels_in: channels_in.into_iter().map(|channel| channel.erased().to_pod()).collect(),
            channels_out: channels_out.into_iter().map(|channel| channel.erased().to_pod()).collect(),
            params,
            calculated_size: Size::ZERO,
        };
        self.children.push(child);
        self
    }

    /// Returns the number of children (widgets and spacers) this flex container has.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this flex container has no children (widgets or spacers).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// --- MARK: WIDGETMUT
impl FloatingPanes {
    // /// Set the flex direction (see [`Axis`]).
    // pub fn set_direction(this: &mut WidgetMut<'_, Self>, direction: Axis) {
    //     this.widget.direction = direction;
    //     this.ctx.request_layout();
    // }

    // /// Set the children's [`CrossAxisAlignment`].
    // pub fn set_cross_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: CrossAxisAlignment) {
    //     this.widget.cross_alignment = alignment;
    //     this.ctx.request_layout();
    // }

    // /// Set the children's [`MainAxisAlignment`].
    // pub fn set_main_axis_alignment(this: &mut WidgetMut<'_, Self>, alignment: MainAxisAlignment) {
    //     this.widget.main_alignment = alignment;
    //     this.ctx.request_layout();
    // }

    // /// Set whether the container must expand to fill the available space on
    // /// its main axis.
    // pub fn set_must_fill_main_axis(this: &mut WidgetMut<'_, Self>, fill: bool) {
    //     this.widget.fill_major_axis = fill;
    //     this.ctx.request_layout();
    // }

    // /// Set the spacing along the major axis between any two elements in logical pixels.
    // ///
    // /// Equivalent to the css [gap] property.
    // ///
    // /// This gap is between any two children, including spacers.
    // /// As such, using a non-zero gap and also adding spacers may lead to counter-intuitive results.
    // /// You should usually pick one or the other.
    // ///
    // /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    // pub fn set_gap(this: &mut WidgetMut<'_, Self>, gap: Length) {
    //     this.widget.gap = gap;
    //     this.ctx.request_layout();
    // }

    /// Insert a non-flex child widget at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn insert_child(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        content: NewWidget<impl Widget + ?Sized>,
        channels_in: Vec<NewWidget<impl Widget + ?Sized>>,
        channels_out: Vec<NewWidget<impl Widget + ?Sized>>,
        params: FloatingPaneParams,
    ) {
        let child = Child {
            content: content.erased().to_pod(),
            channels_in: channels_in.into_iter().map(|channel| channel.erased().to_pod()).collect(),
            channels_out: channels_out.into_iter().map(|channel| channel.erased().to_pod()).collect(),
            params,
            calculated_size: Size::ZERO,
        };
        this.widget.children.insert(idx, child);
        this.ctx.children_changed();
    }

    /// Remove the child at `idx`.
    ///
    /// This child can be a widget or a spacer.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn remove_child(this: &mut WidgetMut<'_, Self>, idx: usize) {
        let child = this.widget.children.remove(idx);
        this.ctx.remove_child(child.content);
        this.ctx.request_layout();
    }

    pub fn insert_child_channel(
        this: &mut WidgetMut<'_, Self>,
        pane_index: usize,
        channel_direction: ChannelDirection,
        channel_index: usize,
        label: NewWidget<impl Widget + ?Sized>,
    ) {
        let channels = this.widget.children[pane_index].get_channels_mut(channel_direction);
        channels.insert(channel_index, label.erased().to_pod());
        this.ctx.children_changed();
    }

    pub fn remove_child_channel(
        this: &mut WidgetMut<'_, Self>,
        pane_index: usize,
        channel_direction: ChannelDirection,
        channel_index: usize,
    ) {
        let channels = this.widget.children[pane_index].get_channels_mut(channel_direction);
        let channel = channels.remove(channel_index);
        this.ctx.remove_child(channel);
        this.ctx.request_layout();
    }

    /// Returns a mutable reference to the child widget at `idx`.
    ///
    /// # Panics
    ///
    /// Panics if the index is larger than the number of children.
    pub fn child_content_mut<'t>(this: &'t mut WidgetMut<'_, Self>, idx: usize) -> WidgetMut<'t, dyn Widget> {
        let content = &mut this.widget.children[idx].content;
        this.ctx.get_mut(content)
    }

    pub fn child_channel_label_mut<'t>(
        this: &'t mut WidgetMut<'_, Self>,
        pane_idx: usize,
        channel_direction: ChannelDirection,
        channel_index: usize,
    ) -> WidgetMut<'t, dyn Widget> {
        let content = &mut this.widget.children[pane_idx].get_channels_mut(channel_direction)[channel_index];
        this.ctx.get_mut(content)
    }

    /// Updates the flex parameters for the child at `idx`,
    ///
    /// # Panics
    ///
    /// Panics if the element at `idx` is not a widget.
    pub fn update_child_params(
        this: &mut WidgetMut<'_, Self>,
        idx: usize,
        params: impl Into<FloatingPaneParams>,
    ) {
        let child = &mut this.widget.children[idx];
        child.params = params.into();
        // let child_val = std::mem::replace(child, Child::FixedSpacer(Length::ZERO, 0.0));
        // let widget = match child_val {
        //     Child::Fixed { widget, .. } | Child::Flex { widget, .. } => widget,
        //     _ => {
        //         panic!("Can't update flex parameters of a spacer element");
        //     }
        // };
        // let new_child = new_flex_child(params.into(), widget);
        // *child = new_child;
        this.ctx.children_changed();
    }

    /// Remove all children from the container.
    pub fn clear(this: &mut WidgetMut<'_, Self>) {
        if !this.widget.children.is_empty() {
            this.ctx.request_layout();

            for child in this.widget.children.drain(..) {
                this.ctx.remove_child(child.content);
            }
        }
    }
}

// fn get_spacing(alignment: MainAxisAlignment, extra: f64, child_count: usize) -> (f64, f64) {
//     let space_before;
//     let space_between;
//     match alignment {
//         _ if child_count == 0 => {
//             space_before = 0.;
//             space_between = 0.;
//         }
//         MainAxisAlignment::Start => {
//             space_before = 0.;
//             space_between = 0.;
//         }
//         MainAxisAlignment::End => {
//             space_before = extra;
//             space_between = 0.;
//         }
//         MainAxisAlignment::Center => {
//             space_before = extra / 2.;
//             space_between = 0.;
//         }
//         MainAxisAlignment::SpaceBetween => {
//             let equal_space = extra / (child_count - 1).max(1) as f64;
//             space_before = 0.;
//             space_between = equal_space;
//         }
//         MainAxisAlignment::SpaceEvenly => {
//             let equal_space = extra / (child_count + 1) as f64;
//             space_before = equal_space;
//             space_between = equal_space;
//         }
//         MainAxisAlignment::SpaceAround => {
//             let equal_space = extra / (2 * child_count) as f64;
//             space_before = equal_space;
//             space_between = equal_space * 2.;
//         }
//     }
//     (space_before, space_between)
// }

impl HasProperty<Background> for FloatingPanes {}
impl HasProperty<BorderColor> for FloatingPanes {}
impl HasProperty<BorderWidth> for FloatingPanes {}
impl HasProperty<CornerRadius> for FloatingPanes {}
impl HasProperty<Padding> for FloatingPanes {}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct GrabStateMove {
    pub grab_element_offset: Vec2,
    pub grab_start_position_local: Point,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct GrabStateResize {
    pub grab_element_position_local: Point,
    pub grab_element_size: Point,
    pub grab_mouse_position_local: Point,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PaneResizeDirection {
    None,
    Negative,
    Positive,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq)]
pub struct PaneResizeDirections {
    x: PaneResizeDirection,
    y: PaneResizeDirection,
}

impl PaneResizeDirections {
    pub const NONE: Self =
        PaneResizeDirections { x: PaneResizeDirection::None, y: PaneResizeDirection::None };

    pub fn is_none(&self) -> bool {
        self.x == PaneResizeDirection::None && self.y == PaneResizeDirection::None
    }
}

#[derive(Debug, Clone, PartialEq)]
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

// --- MARK: IMPL WIDGET
impl Widget for FloatingPanes {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary), state, ..
            }) if self.gesture.is_none() => {
                let local_point = ctx.local_position(state.position);
                if let Some((hovered_pane_index, child)) =
                    self.children.iter().enumerate().find(|(_, child)| {
                        child.layout_bounding_rect(self.children_offset).contains(local_point)
                    })
                {
                    self.gesture = Some(Gesture::GrabPane {
                        pane_index: hovered_pane_index,
                        grab_state: GrabStateMove {
                            grab_element_offset: child.params.position_local.to_vec2(),
                            grab_start_position_local: local_point,
                        },
                    });
                } else {
                    self.gesture = Some(Gesture::GrabBackground(GrabStateMove {
                        grab_element_offset: self.children_offset,
                        grab_start_position_local: local_point,
                    }));
                }
                ctx.set_handled();
            }
            PointerEvent::Up(PointerButtonEvent { button: Some(PointerButton::Primary), .. })
                if self.gesture.is_some() =>
            {
                self.gesture = None;
                ctx.set_handled();
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => match &self.gesture {
                Some(Gesture::GrabBackground(GrabStateMove {
                    grab_element_offset,
                    grab_start_position_local,
                })) => {
                    self.children_offset = ctx.local_position(current.position).to_vec2()
                        + *grab_element_offset
                        - grab_start_position_local.to_vec2();
                    ctx.request_layout();
                    ctx.set_handled();
                }
                Some(Gesture::GrabPane {
                    pane_index,
                    grab_state: GrabStateMove { grab_element_offset, grab_start_position_local },
                }) => {
                    if let Some(grabbed_pane) = self.children.get_mut(*pane_index) {
                        grabbed_pane.params.position_local = ctx.local_position(current.position)
                            + *grab_element_offset
                            - grab_start_position_local.to_vec2();
                        ctx.request_layout();
                    }
                    ctx.set_handled();
                }
                _ => (),
            },
            _ => (),
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        for child in self.children.iter_mut() {
            ctx.register_child(&mut child.content);
            child.channels_in.iter_mut().for_each(|channel| ctx.register_child(channel));
            child.channels_out.iter_mut().for_each(|channel| ctx.register_child(channel));
        }
    }

    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        Background::prop_changed(ctx, property_type);
        BorderColor::prop_changed(ctx, property_type);
        BorderWidth::prop_changed(ctx, property_type);
        CornerRadius::prop_changed(ctx, property_type);
        Padding::prop_changed(ctx, property_type);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        // SETUP
        let border = props.get::<BorderWidth>();
        let padding = props.get::<Padding>();

        let bc = *bc;
        let bc = border.layout_down(bc);
        let bc = padding.layout_down(bc);

        // // we loosen our constraints when passing to children.
        // let loosened_bc = bc.loosen();

        for child in &mut self.children {
            let child_bc = BoxConstraints::UNBOUNDED;
            // TODO: Resizeable panes.
            // child_bc = BoxConstraints::new(Size::new(50.0, 100.0), Size::new(50.0, 100.0));
            child.calculated_size = ctx.run_layout(&mut child.content, &child_bc);
            let position = child.layout_bounding_rect(self.children_offset).origin();
            // let child_baseline = ctx.child_baseline_offset(widget);
            ctx.place_child(&mut child.content, position);

            let channel_bc =
                BoxConstraints::new(Size::ZERO, Size::new(child.calculated_size.width / 2.0, f64::INFINITY));

            let mut layout_channels = |channels: &mut [_], offset: Vec2| {
                let mut channel_position = child.calculated_size.height;

                for channel in channels {
                    let channel_size = ctx.run_layout(channel, &channel_bc);
                    ctx.place_child(channel, position + offset + Vec2::new(0.0, channel_position));
                    channel_position += channel_size.height;
                }

                channel_position
            };

            child.calculated_size.height = std::cmp::max_by(
                layout_channels(&mut child.channels_in, Vec2::ZERO),
                layout_channels(&mut child.channels_out, Vec2::new(child.calculated_size.width / 2.0, 0.0)),
                |lhs, rhs| lhs.total_cmp(rhs),
            );
        }

        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, scene: &mut Scene) {
        let border_width = props.get::<BorderWidth>();
        let border_radius = props.get::<CornerRadius>();
        let bg = props.get::<Background>();
        let border_color = props.get::<BorderColor>();

        let bg_rect = border_width.bg_rect(ctx.size(), border_radius);
        let border_rect = border_width.border_rect(ctx.size(), border_radius);

        let brush = bg.get_peniko_brush_for_rect(bg_rect.rect());
        fill(scene, &bg_rect, &brush);
        stroke(scene, &border_rect, border_color.color, border_width.width);

        // Clip children drawn outside the layout area.
        scene.push_clip_layer(Affine::IDENTITY, &bg_rect);

        for child in &self.children {
            let child_bg_rect = child.layout_bounding_rect(self.children_offset);
            // TODO: customizable background
            let brush = Background::Color(AlphaColor::from_rgb8(0x3F, 0x3F, 0x1F))
                .get_peniko_brush_for_rect(child_bg_rect);

            fill(scene, &child_bg_rect, &brush);
        }

        // paint the baseline if we're debugging layout
        if ctx.debug_paint_enabled() && ctx.baseline_offset() != 0.0 {
            let color = ctx.debug_color();
            let my_baseline = ctx.size().height - ctx.baseline_offset();
            let line = Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));

            let stroke_style = Stroke::new(1.0).with_dashes(0., [4.0, 4.0]);
            scene.stroke(&stroke_style, Affine::IDENTITY, color, None, &line);
        }
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        match &self.gesture {
            Some(Gesture::GrabBackground(_) | Gesture::GrabPane { .. }) => CursorIcon::Grabbing,
            Some(Gesture::ResizePane { directions, .. }) => {
                use PaneResizeDirection::*;
                match (directions.x, directions.y) {
                    (Negative, None) => CursorIcon::WResize,
                    (Positive, None) => CursorIcon::EResize,
                    (None, Negative) => CursorIcon::NResize,
                    (None, Positive) => CursorIcon::SResize,
                    (Negative, Negative) => CursorIcon::NwResize,
                    (Positive, Negative) => CursorIcon::NeResize,
                    (Positive, Positive) => CursorIcon::SeResize,
                    (Negative, Positive) => CursorIcon::SwResize,
                    (None, None) => unreachable!(),
                }
            }
            None => CursorIcon::Default,
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        self.children
            .iter()
            .flat_map(|widget| {
                chain![
                    iter::once(widget.content.id()),
                    widget.channels_in.iter().map(|channel| channel.id()),
                    widget.channels_out.iter().map(|channel| channel.id()),
                ]
            })
            .collect()
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        trace_span!("Flex", id = id.trace())
    }
}

// --- MARK: TESTS
#[cfg(test)]
mod tests {
    use super::*;
    use xilem::masonry::properties::types::AsUnit;
    use xilem::masonry::theme::{ACCENT_COLOR, default_property_set};
    use xilem::masonry::widgets::Label;

    // #[test]
    // fn test_main_axis_alignment_spacing() {
    //     let apply_align = |align, extra, child_count| {
    //         let (space_before, space_between) = get_spacing(align, extra, child_count);
    //         let space_after = extra - space_before - space_between * child_count.saturating_sub(1) as f64;
    //         (space_before, space_between, space_after)
    //     };

    //     // Formatting note: in the comments below:
    //     // `[-]` represents a child.
    //     // a number represents a non-zero amount of space.

    //     let align = MainAxisAlignment::Start;
    //     let (before, _, after) = apply_align(align, 10., 1);
    //     // Spacing: [-] 10
    //     assert_eq!(before, 0.);
    //     assert_eq!(after, 10.);

    //     let (before, between, after) = apply_align(align, 10., 2);
    //     // Spacing: [-][-] 10
    //     assert_eq!(before, 0.);
    //     assert_eq!(between, 0.);
    //     assert_eq!(after, 10.);

    //     let align = MainAxisAlignment::End;
    //     let (before, _, after) = apply_align(align, 10., 1);
    //     // Spacing: 10 [-]
    //     assert_eq!(before, 10.);
    //     assert_eq!(after, 0.);

    //     let (before, between, after) = apply_align(align, 10., 2);
    //     // Spacing: 10 [-][-]
    //     assert_eq!(before, 10.);
    //     assert_eq!(between, 0.);
    //     assert_eq!(after, 0.);

    //     let align = MainAxisAlignment::Center;
    //     let (before, _, after) = apply_align(align, 10., 1);
    //     // Spacing: 5 [-] 5
    //     assert_eq!(before, 5.);
    //     assert_eq!(after, 5.);

    //     let (before, between, after) = apply_align(align, 10., 3);
    //     // Spacing: 5 [-][-][-] 5
    //     assert_eq!(before, 5.);
    //     assert_eq!(between, 0.);
    //     assert_eq!(after, 5.);

    //     let (before, between, after) = apply_align(align, 5., 2);
    //     // Spacing: 2.5 [-][-] 2.5
    //     assert_eq!(before, 2.5);
    //     assert_eq!(between, 0.);
    //     assert_eq!(after, 2.5);

    //     let align = MainAxisAlignment::SpaceBetween;
    //     let (before, _, after) = apply_align(align, 10., 1);
    //     // Spacing: [-] 10
    //     assert_eq!(before, 0.);
    //     assert_eq!(after, 10.);

    //     let (before, between, after) = apply_align(align, 10., 2);
    //     // Spacing: [-] 10 [-]
    //     assert_eq!(before, 0.);
    //     assert_eq!(between, 10.);
    //     assert_eq!(after, 0.);

    //     let (before, between, after) = apply_align(align, 30., 5);
    //     // Spacing: [-] 7.5 [-] 7.5 [-] 7.5 [-] 7.5 [-]
    //     assert_eq!(before, 0.);
    //     assert_eq!(between, 7.5);
    //     assert_eq!(after, 0.);

    //     let align = MainAxisAlignment::SpaceEvenly;
    //     let (before, _, after) = apply_align(align, 10., 1);
    //     // Spacing: 5 [-] 5
    //     assert_eq!(before, 5.);
    //     assert_eq!(after, 5.);

    //     let (before, between, after) = apply_align(align, 10., 3);
    //     // Spacing: 2.5 [-] 2.5 [-] 2.5 [-] 2.5
    //     assert_eq!(before, 2.5);
    //     assert_eq!(between, 2.5);
    //     assert_eq!(after, 2.5);

    //     let align = MainAxisAlignment::SpaceAround;
    //     let (before, _, after) = apply_align(align, 10., 1);
    //     // Spacing: 5 [-] 5
    //     assert_eq!(before, 5.);
    //     assert_eq!(after, 5.);

    //     let (before, between, after) = apply_align(align, 10., 2);
    //     // Spacing: 2.5 [-] 5 [-] 2.5
    //     assert_eq!(before, 2.5);
    //     assert_eq!(between, 5.);
    //     assert_eq!(after, 2.5);

    //     let (before, between, after) = apply_align(align, 35., 5);
    //     // Spacing: 3.5 [-] 7 [-] 7 [-] 7 [-] 7 [-] 3.5
    //     assert_eq!(before, 3.5);
    //     assert_eq!(between, 7.);
    //     assert_eq!(after, 3.5);
    // }

    // #[test]
    // fn invalid_flex_params() {
    //     use masonry_testing::assert_debug_panics;
    //     assert_debug_panics!(FlexParams::new(0.0, None), "Flex value should be > 0.0");
    //     assert_debug_panics!(FlexParams::new(-0.0, None), "Flex value should be > 0.0");
    //     assert_debug_panics!(FlexParams::new(-1.0, None), "Flex value should be > 0.0");
    // }

    // use xilem::masonry::testing::{TestHarness, assert_render_snapshot};

    // #[test]
    // fn flex_row_fixed_size_only() {
    //     let widget = NewWidget::new_with_props(
    //         Flex::row()
    //             .with_child(Label::new("hello").with_auto_id())
    //             .with_child(Label::new("world").with_auto_id())
    //             .with_child(Label::new("foo").with_auto_id())
    //             .with_child(Label::new("bar").with_auto_id()),
    //         (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
    //     );

    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Start);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fixed_children_start");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Center);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fixed_children_center");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::End);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fixed_children_end");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceBetween);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fixed_children_spaceBetween");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceEvenly);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fixed_children_spaceEvenly");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceAround);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fixed_children_spaceAround");
    // }

    // // TODO - Reduce copy-pasting?
    // #[test]
    // fn flex_row_cross_axis_snapshots() {
    //     let widget = NewWidget::new_with_props(
    //         Flex::row()
    //             .with_child(Label::new("hello").with_auto_id())
    //             .with_flex_child(Label::new("world").with_auto_id(), 1.0)
    //             .with_child(Label::new("foo").with_auto_id())
    //             .with_flex_child(
    //                 Label::new("bar").with_auto_id(),
    //                 FlexParams::new(2.0, CrossAxisAlignment::Start),
    //             ),
    //         (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
    //     );

    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Start);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_cross_axis_start");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Center);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_cross_axis_center");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::End);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_cross_axis_end");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Baseline);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_cross_axis_baseline");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Fill);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_cross_axis_fill");
    // }

    // #[test]
    // fn flex_row_main_axis_snapshots() {
    //     let widget = NewWidget::new_with_props(
    //         Flex::row()
    //             .with_child(Label::new("hello").with_auto_id())
    //             .with_flex_child(Label::new("world").with_auto_id(), 1.0)
    //             .with_child(Label::new("foo").with_auto_id())
    //             .with_flex_child(
    //                 Label::new("bar").with_auto_id(),
    //                 FlexParams::new(2.0, CrossAxisAlignment::Start),
    //             ),
    //         (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
    //     );

    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);

    //     // MAIN AXIS ALIGNMENT

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Start);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_main_axis_start");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Center);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_main_axis_center");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::End);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_main_axis_end");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceBetween);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_main_axis_spaceBetween");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceEvenly);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_main_axis_spaceEvenly");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceAround);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_main_axis_spaceAround");

    //     // FILL MAIN AXIS
    //     // TODO - This doesn't seem to do anything?

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_must_fill_main_axis(&mut flex, true);
    //     });
    //     assert_render_snapshot!(harness, "flex_row_fill_main_axis");
    // }

    // #[test]
    // fn flex_col_cross_axis_snapshots() {
    //     let widget = NewWidget::new_with_props(
    //         Flex::column()
    //             .with_child(Label::new("hello").with_auto_id())
    //             .with_flex_child(Label::new("world").with_auto_id(), 1.0)
    //             .with_child(Label::new("foo").with_auto_id())
    //             .with_flex_child(
    //                 Label::new("bar").with_auto_id(),
    //                 FlexParams::new(2.0, CrossAxisAlignment::Start),
    //             ),
    //         (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
    //     );

    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Start);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_cross_axis_start");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Center);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_cross_axis_center");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::End);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_cross_axis_end");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Baseline);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_cross_axis_baseline");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_cross_axis_alignment(&mut flex, CrossAxisAlignment::Fill);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_cross_axis_fill");
    // }

    // #[test]
    // fn flex_col_main_axis_snapshots() {
    //     let widget = NewWidget::new_with_props(
    //         Flex::column()
    //             .with_child(Label::new("hello").with_auto_id())
    //             .with_flex_child(Label::new("world").with_auto_id(), 1.0)
    //             .with_child(Label::new("foo").with_auto_id())
    //             .with_flex_child(
    //                 Label::new("bar").with_auto_id(),
    //                 FlexParams::new(2.0, CrossAxisAlignment::Start),
    //             ),
    //         (BorderWidth::all(2.0), BorderColor::new(ACCENT_COLOR)).into(),
    //     );

    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);

    //     // MAIN AXIS ALIGNMENT

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Start);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_main_axis_start");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::Center);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_main_axis_center");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::End);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_main_axis_end");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceBetween);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_main_axis_spaceBetween");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceEvenly);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_main_axis_spaceEvenly");

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_main_axis_alignment(&mut flex, MainAxisAlignment::SpaceAround);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_main_axis_spaceAround");

    //     // FILL MAIN AXIS
    //     // TODO - This doesn't seem to do anything?

    //     harness.edit_root_widget(|mut flex| {
    //         Flex::set_must_fill_main_axis(&mut flex, true);
    //     });
    //     assert_render_snapshot!(harness, "flex_col_fill_main_axis");
    // }

    // #[test]
    // fn edit_flex_container() {
    //     let image_1 = {
    //         let widget = Flex::column()
    //             .with_child(Label::new("a").with_auto_id())
    //             .with_child(Label::new("b").with_auto_id())
    //             .with_child(Label::new("c").with_auto_id())
    //             .with_child(Label::new("d").with_auto_id())
    //             .with_auto_id();
    //         // -> abcd

    //         let window_size = Size::new(200.0, 150.0);
    //         let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);

    //         harness.edit_root_widget(|mut flex| {
    //             Flex::remove_child(&mut flex, 1);
    //             // -> acd
    //             Flex::add_child(&mut flex, Label::new("x").with_auto_id());
    //             // -> acdx
    //             Flex::add_flex_child(&mut flex, Label::new("y").with_auto_id(), 2.0);
    //             // -> acdxy
    //             Flex::add_spacer(&mut flex, 5.px());
    //             // -> acdxy_
    //             Flex::add_flex_spacer(&mut flex, 1.0);
    //             // -> acdxy__
    //             Flex::insert_child(&mut flex, 2, Label::new("i").with_auto_id());
    //             // -> acidxy__
    //             Flex::insert_flex_child(&mut flex, 2, Label::new("j").with_auto_id(), 2.0);
    //             // -> acjidxy__
    //             Flex::insert_spacer(&mut flex, 2, 5.px());
    //             // -> ac_jidxy__
    //             Flex::insert_flex_spacer(&mut flex, 2, 1.0);
    //             // -> ac__jidxy__
    //         });

    //         harness.render()
    //     };

    //     let image_2 = {
    //         let widget = Flex::column()
    //             .with_child(Label::new("a").with_auto_id())
    //             .with_child(Label::new("c").with_auto_id())
    //             .with_flex_spacer(1.0)
    //             .with_spacer(5.px())
    //             .with_flex_child(Label::new("j").with_auto_id(), 2.0)
    //             .with_child(Label::new("i").with_auto_id())
    //             .with_child(Label::new("d").with_auto_id())
    //             .with_child(Label::new("x").with_auto_id())
    //             .with_flex_child(Label::new("y").with_auto_id(), 2.0)
    //             .with_spacer(5.px())
    //             .with_flex_spacer(1.0)
    //             .with_auto_id();

    //         let window_size = Size::new(200.0, 150.0);
    //         let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);
    //         harness.render()
    //     };

    //     // We don't use assert_eq because we don't want rich assert
    //     assert!(image_1 == image_2);
    // }

    // #[test]
    // fn get_flex_child() {
    //     let widget = Flex::column()
    //         .with_child(Label::new("hello").with_auto_id())
    //         .with_child(Label::new("world").with_auto_id())
    //         .with_spacer(1.px())
    //         .with_auto_id();

    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);
    //     harness.edit_root_widget(|mut flex| {
    //         let mut child = Flex::child_mut(&mut flex, 1).unwrap();
    //         assert_eq!(child.try_downcast::<Label>().unwrap().widget.text().to_string(), "world");
    //         drop(child);

    //         assert!(Flex::child_mut(&mut flex, 2).is_none());
    //     });

    //     // TODO - test out-of-bounds access?
    // }

    // #[test]
    // fn divide_by_zero() {
    //     let widget = Flex::column().with_flex_spacer(0.0).with_auto_id();

    //     // Running layout should not panic when the flex sum is zero.
    //     let window_size = Size::new(200.0, 150.0);
    //     let mut harness = TestHarness::create_with_size(default_property_set(), widget, window_size);
    //     harness.render();
    // }
}
