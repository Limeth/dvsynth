use std::marker::PhantomData;

use crate::widgets;
pub use crate::widgets::FloatingPaneParams;
use xilem::masonry::core::{Axis, FromDynWidget, Widget, WidgetMut};
use xilem::masonry::properties::types::Length;
pub use xilem::masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};

use xilem::core::{
    AppendVec, ElementSplice, MessageContext, MessageResult, Mut, SuperElement, View, ViewElement, ViewId,
    ViewMarker, ViewPathTracker, ViewSequence,
};
use xilem::{AnyWidgetView, Pod, ViewCtx, WidgetView};

pub fn floating_panes<State, Action, Seq: FloatingPaneSequence<State, Action>>(
    sequence: Seq,
) -> FloatingPanes<Seq, State, Action> {
    FloatingPanes { sequence, phantom: PhantomData }
}

/// The [`View`] created by [`flex`] from a sequence.
///
/// See `flex` documentation for more context.
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct FloatingPanes<Seq, State, Action = ()> {
    sequence: Seq,
    // axis: Axis,
    // cross_axis_alignment: CrossAxisAlignment,
    // main_axis_alignment: MainAxisAlignment,
    // fill_major_axis: bool,
    // gap: Length,
    phantom: PhantomData<fn() -> (State, Action)>,
}

impl<Seq, State, Action> FloatingPanes<Seq, State, Action> {
    // /// Set the flex direction (see [`Axis`]).
    // pub fn direction(mut self, axis: Axis) -> Self {
    //     self.axis = axis;
    //     self
    // }
    // /// Set the children's [`CrossAxisAlignment`].
    // pub fn cross_axis_alignment(mut self, axis: CrossAxisAlignment) -> Self {
    //     self.cross_axis_alignment = axis;
    //     self
    // }
    // /// Set the children's [`MainAxisAlignment`].
    // pub fn main_axis_alignment(mut self, axis: MainAxisAlignment) -> Self {
    //     self.main_axis_alignment = axis;
    //     self
    // }
    // /// Set whether the container must expand to fill the available space on
    // /// its main axis.
    // pub fn must_fill_major_axis(mut self, fill_major_axis: bool) -> Self {
    //     self.fill_major_axis = fill_major_axis;
    //     self
    // }

    // /// Set the spacing along the major axis between any two elements in logical pixels.
    // ///
    // /// Equivalent to the css [gap] property.
    // ///
    // /// This gap is between any two children, including spacers.
    // /// As such, when adding a spacer, you add both the spacer's size (or computed flex size)
    // /// and the gap between the spacer and its neighbors.
    // /// As such, if you're adding lots of spacers to a flex parent, you may want to set
    // /// its gap to zero to make the layout more predictable.
    // ///
    // /// Leave unset to use the default spacing which is [`DEFAULT_GAP`].
    // ///
    // /// [gap]: https://developer.mozilla.org/en-US/docs/Web/CSS/gap
    // /// [`DEFAULT_GAP`]: masonry::theme::DEFAULT_GAP
    // #[track_caller]
    // pub fn gap(mut self, gap: Length) -> Self {
    //     self.gap = gap;
    //     self
    // }
}

mod hidden {
    use super::{FloatingPaneElement, FloatingPaneItem};
    use xilem::core::{AppendVec, View};
    use xilem::{AnyWidgetView, ViewCtx};

    #[doc(hidden)]
    #[expect(unnameable_types, reason = "Implementation detail, public because of trait visibility rules")]
    pub struct FloatingPaneState<SeqState> {
        pub(crate) seq_state: SeqState,
        pub(crate) scratch: AppendVec<FloatingPaneElement>,
    }

    // #[doc(hidden)]
    // #[expect(unnameable_types, reason = "Implementation detail, public because of trait visibility rules")]
    // pub struct AnyFlexChildState<State: 'static, Action: 'static> {
    //     /// Just the optional view state of the flex item view
    //     #[allow(clippy::type_complexity, reason = "There's no way to avoid spelling out this type.")]
    //     pub(crate) inner:
    //         Option<
    //             <FlexItem<Box<AnyWidgetView<State, Action>>, State, Action> as View<
    //                 State,
    //                 Action,
    //                 ViewCtx,
    //             >>::ViewState,
    //         >,
    //     /// The generational id handling is essentially very similar to that of the `Option<impl ViewSequence>`,
    //     /// where `None` would represent a Spacer, and `Some` a view
    //     pub(crate) generation: u64,
    // }
}

// use hidden::AnyFlexChildState;
use hidden::FloatingPaneState;

impl<Seq, State, Action> ViewMarker for FloatingPanes<Seq, State, Action> {}
impl<State, Action, Seq> View<State, Action, ViewCtx> for FloatingPanes<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: FloatingPaneSequence<State, Action>,
{
    type Element = Pod<widgets::FloatingPanes>;

    type ViewState = FloatingPaneState<Seq::SeqState>;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let mut elements = AppendVec::default();
        let mut widget = widgets::FloatingPanes::new();
        //widgets::FloatingPanes::for_axis(self.axis)
        // .with_gap(self.gap)
        // .cross_axis_alignment(self.cross_axis_alignment)
        // .must_fill_main_axis(self.fill_major_axis)
        // .main_axis_alignment(self.main_axis_alignment);
        let seq_state = self.sequence.seq_build(ctx, &mut elements, app_state);
        for child in elements.drain() {
            widget = widget.with_child(child.content.new_widget, child.params);
        }
        let pod = ctx.create_pod(widget);
        let state = FloatingPaneState { seq_state, scratch: elements };

        (pod, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        FloatingPaneState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        // if prev.axis != self.axis {
        //     widgets::FloatingPanes::set_direction(&mut element, self.axis);
        // }
        // if prev.cross_axis_alignment != self.cross_axis_alignment {
        //     widgets::FloatingPanes::set_cross_axis_alignment(&mut element, self.cross_axis_alignment);
        // }
        // if prev.main_axis_alignment != self.main_axis_alignment {
        //     widgets::FloatingPanes::set_main_axis_alignment(&mut element, self.main_axis_alignment);
        // }
        // if prev.fill_major_axis != self.fill_major_axis {
        //     widgets::FloatingPanes::set_must_fill_main_axis(&mut element, self.fill_major_axis);
        // }
        // if prev.gap != self.gap {
        //     widgets::FloatingPanes::set_gap(&mut element, self.gap);
        // }
        let mut splice = FlexSplice::new(element, scratch);
        self.sequence.seq_rebuild(&prev.sequence, seq_state, ctx, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
    }

    fn teardown(
        &self,
        FloatingPaneState { seq_state, scratch }: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        let mut splice = FlexSplice::new(element, scratch);
        self.sequence.seq_teardown(seq_state, ctx, &mut splice);
        debug_assert!(scratch.is_empty());
    }

    fn message(
        &self,
        FloatingPaneState { seq_state, scratch }: &mut Self::ViewState,
        message: &mut MessageContext,
        element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut splice = FlexSplice::new(element, scratch);
        let result = self.sequence.seq_message(seq_state, message, &mut splice, app_state);
        debug_assert!(scratch.is_empty());
        result
    }
}

/// A child element of a [`Flex`] view.
pub struct FloatingPaneElement {
    params: FloatingPaneParams,
    content: Pod<dyn Widget>,
}

/// A mutable reference to a [`FlexElement`], used internally by Xilem traits.
pub struct FlexElementMut<'w> {
    parent: WidgetMut<'w, widgets::FloatingPanes>,
    idx: usize,
}

struct FlexSplice<'w, 's> {
    idx: usize,
    element: WidgetMut<'w, widgets::FloatingPanes>,
    scratch: &'s mut AppendVec<FloatingPaneElement>,
}

impl<'w, 's> FlexSplice<'w, 's> {
    fn new(
        element: WidgetMut<'w, widgets::FloatingPanes>,
        scratch: &'s mut AppendVec<FloatingPaneElement>,
    ) -> Self {
        debug_assert!(scratch.is_empty());
        Self { idx: 0, element, scratch }
    }
}

impl ViewElement for FloatingPaneElement {
    type Mut<'w> = FlexElementMut<'w>;
}

impl SuperElement<Self, ViewCtx> for FloatingPaneElement {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let r = {
            let parent = this.parent.reborrow_mut();
            let reborrow = FlexElementMut { idx: this.idx, parent };
            f(reborrow)
        };
        (this, r)
    }
}

impl<W: Widget + FromDynWidget + ?Sized> SuperElement<Pod<W>, ViewCtx> for FloatingPaneElement {
    fn upcast(_: &mut ViewCtx, child: Pod<W>) -> Self {
        Self { content: child.erased(), params: Default::default() }
    }

    fn with_downcast_val<R>(
        mut this: Mut<'_, Self>,
        f: impl FnOnce(Mut<'_, Pod<W>>) -> R,
    ) -> (Mut<'_, Self>, R) {
        let ret = {
            let mut child = widgets::FloatingPanes::child_mut(&mut this.parent, this.idx);
            let downcast = child.downcast();
            f(downcast)
        };

        (this, ret)
    }
}

impl ElementSplice<FloatingPaneElement> for FlexSplice<'_, '_> {
    fn insert(&mut self, element: FloatingPaneElement) {
        widgets::FloatingPanes::insert_child(
            &mut self.element,
            self.idx,
            element.content.new_widget,
            element.params,
        );
        self.idx += 1;
    }

    fn with_scratch<R>(&mut self, f: impl FnOnce(&mut AppendVec<FloatingPaneElement>) -> R) -> R {
        let ret = f(self.scratch);
        for element in self.scratch.drain() {
            widgets::FloatingPanes::insert_child(
                &mut self.element,
                self.idx,
                element.content.new_widget,
                element.params,
            );
            self.idx += 1;
        }
        ret
    }

    fn mutate<R>(&mut self, f: impl FnOnce(Mut<'_, FloatingPaneElement>) -> R) -> R {
        let child = FlexElementMut { parent: self.element.reborrow_mut(), idx: self.idx };
        let ret = f(child);
        self.idx += 1;
        ret
    }

    fn delete<R>(&mut self, f: impl FnOnce(Mut<'_, FloatingPaneElement>) -> R) -> R {
        let ret = {
            let child = FlexElementMut { parent: self.element.reborrow_mut(), idx: self.idx };
            f(child)
        };
        widgets::FloatingPanes::remove_child(&mut self.element, self.idx);
        ret
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn index(&self) -> usize {
        self.idx
    }
}

/// An ordered sequence of views for a [`Flex`] view.
/// See [`ViewSequence`] for more technical details.
///
/// # Examples
///
/// ```
/// use xilem::view::{label, FlexSequence, FlexExt as _};
///
/// fn label_sequence<State: 'static>(
///     labels: impl Iterator<Item = &'static str>,
///     flex: f64,
/// ) -> impl FlexSequence<State> {
///     labels.map(|l| label(l).flex(flex)).collect::<Vec<_>>()
/// }
/// ```
pub trait FloatingPaneSequence<State, Action = ()>:
    ViewSequence<State, Action, ViewCtx, FloatingPaneElement>
{
}

impl<Seq, State, Action> FloatingPaneSequence<State, Action> for Seq where Seq: ViewSequence<State, Action, ViewCtx, FloatingPaneElement>
{}

/// A trait which extends a [`WidgetView`] with methods to provide parameters for a flex item, or being able to use it interchangeably with a spacer.
pub trait FloatingPaneExt<State, Action>: WidgetView<State, Action> {
    /// Applies [`impl Into<FlexParams>`](`FlexParams`) to this view, can be used as child of a [`Flex`] [`View`]
    ///
    /// # Examples
    /// ```
    /// use xilem::masonry::properties::types::AsUnit;
    /// use xilem::{view::{Axis, text_button, label, flex, CrossAxisAlignment, FlexSpacer, FlexExt}};
    /// # use xilem::{WidgetView};
    ///
    /// # fn view<State: 'static>() -> impl WidgetView<State> {
    /// flex(Axis::Vertical, (
    ///     text_button("click me", |_| ()).flex(2.0),
    ///     FlexSpacer::Fixed(2.px()),
    ///     label("a label").flex(CrossAxisAlignment::Fill),
    ///     FlexSpacer::Fixed(2.px()),
    /// ))
    /// # }
    ///
    /// ```
    fn floating_pane(self, params: impl Into<FloatingPaneParams>) -> FloatingPaneItem<Self, State, Action>
    where
        State: 'static,
        Action: 'static,
        Self: Sized,
    {
        floating_pane_item(self, params)
    }

    // /// Turns this [`WidgetView`] into an [`AnyFlexChild`],
    // /// which can be used interchangeably with an `FlexSpacer`, as child of a [`Flex`] [`View`]
    // ///
    // /// # Examples
    // /// ```
    // /// use xilem::masonry::properties::types::AsUnit;
    // /// use xilem::{view::{Axis, flex, label, FlexSpacer, FlexExt, AnyFlexChild}};
    // /// # use xilem::{WidgetView};
    // ///
    // /// # fn view<State: 'static>() -> impl WidgetView<State> {
    // /// flex(Axis::Vertical, [label("a label").into_any_flex(), AnyFlexChild::Spacer(FlexSpacer::Fixed(1.px()))])
    // /// # }
    // ///
    // /// ```
    // fn into_any_flex(self) -> AnyFlexChild<State, Action>
    // where
    //     State: 'static,
    //     Action: 'static,
    //     Self: Sized,
    // {
    //     AnyFlexChild::Item(floating_pane_item(self.boxed(), FloatingPaneParams::default()))
    // }
}

impl<State, Action, V: WidgetView<State, Action>> FloatingPaneExt<State, Action> for V {}

/// A `WidgetView` that can be used within a [`Flex`] [`View`].
pub struct FloatingPaneItem<V, State, Action> {
    view: V,
    params: FloatingPaneParams,
    phantom: PhantomData<fn() -> (State, Action)>,
}

/// Applies [`impl Into<FlexParams>`](`FlexParams`) to the [`View`] `V`, can be used as child of a [`Flex`] View.
///
/// # Examples
/// ```
/// use xilem::masonry::properties::types::AsUnit;
/// use xilem::view::{Axis, text_button, label, floating_pane_item, flex, CrossAxisAlignment, FlexSpacer};
/// # use xilem::{WidgetView};
///
/// # fn view<State: 'static>() -> impl WidgetView<State> {
/// flex(Axis::Vertical, (
///     floating_pane_item(text_button("click me", |_| ()), 2.0),
///     FlexSpacer::Fixed(2.px()),
///     floating_pane_item(label("a label"), CrossAxisAlignment::Fill),
///     FlexSpacer::Fixed(2.px()),
/// ))
/// # }
///
/// ```
pub fn floating_pane_item<V, State, Action>(
    view: V,
    params: impl Into<FloatingPaneParams>,
) -> FloatingPaneItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    FloatingPaneItem { params: params.into(), view, phantom: PhantomData }
}

// impl<State, Action, V> From<FlexItem<V, State, Action>> for AnyFlexChild<State, Action>
// where
//     State: 'static,
//     Action: 'static,
//     V: WidgetView<State, Action, ViewState: 'static>,
// {
//     fn from(value: FlexItem<V, State, Action>) -> Self {
//         Self::Item(floating_pane_item(value.view.boxed(), value.params))
//     }
// }

impl<V, State, Action> ViewMarker for FloatingPaneItem<V, State, Action> {}
impl<State, Action, V> View<State, Action, ViewCtx> for FloatingPaneItem<V, State, Action>
where
    State: 'static,
    Action: 'static,
    V: WidgetView<State, Action>,
{
    type Element = FloatingPaneElement;

    type ViewState = V::ViewState;

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let (pod, state) = self.view.build(ctx, app_state);
        (FloatingPaneElement { content: pod.erased(), params: self.params.clone() }, state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        {
            if self.params != prev.params {
                widgets::FloatingPanes::update_child_params(
                    &mut element.parent,
                    element.idx,
                    self.params.clone(),
                );
            }
            let mut child = widgets::FloatingPanes::child_mut(&mut element.parent, element.idx);
            self.view.rebuild(&prev.view, view_state, ctx, child.downcast(), app_state);
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        let mut child = widgets::FloatingPanes::child_mut(&mut element.parent, element.idx);
        self.view.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let mut child = widgets::FloatingPanes::child_mut(&mut element.parent, element.idx);
        self.view.message(view_state, message, child.downcast(), app_state)
    }
}

// /// A widget-type-erased flex child [`View`], can be used within a [`Flex`] [`View`]
// pub enum AnyFlexChild<State, Action = ()> {
//     /// A child widget.
//     Item(FlexItem<Box<AnyWidgetView<State, Action>>, State, Action>),
//     /// A spacer.
//     Spacer(FlexSpacer),
// }

// impl<State, Action, V> FlexItem<V, State, Action>
// where
//     State: 'static,
//     Action: 'static,
//     V: WidgetView<State, Action>,
// {
//     /// Turns this [`FlexItem`] into an [`AnyFlexChild`]
//     ///
//     /// # Examples
//     /// ```
//     /// use xilem::view::{Axis, flex, floating_pane_item, label};
//     /// # use xilem::{WidgetView};
//     ///
//     /// # fn view<State: 'static>() -> impl WidgetView<State> {
//     /// flex(Axis::Vertical, floating_pane_item(label("Industry"), 4.0).into_any_flex())
//     /// # }
//     ///
//     /// ```
//     pub fn into_any_flex(self) -> AnyFlexChild<State, Action> {
//         AnyFlexChild::Item(floating_pane_item(Box::new(self.view), self.params))
//     }
// }

// impl<State, Action> ViewMarker for AnyFlexChild<State, Action> {}
// impl<State, Action> View<State, Action, ViewCtx> for AnyFlexChild<State, Action>
// where
//     State: 'static,
//     Action: 'static,
// {
//     type Element = FlexElement;

//     type ViewState = AnyFlexChildState<State, Action>;

//     fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
//         let generation = 0;
//         let (element, view_state) = match self {
//             Self::Item(floating_pane_item) => {
//                 let (element, state) =
//                     ctx.with_id(ViewId::new(generation), |ctx| floating_pane_item.build(ctx, app_state));
//                 (element, Some(state))
//             }
//             Self::Spacer(spacer) => {
//                 // We know that the spacer doesn't need any id, as it doesn't receive or sends any messages
//                 // (Similar to `None` as a ViewSequence)
//                 let (element, ()) = View::<(), (), ViewCtx>::build(spacer, ctx, &mut ());
//                 (element, None)
//             }
//         };
//         (element, AnyFlexChildState { inner: view_state, generation })
//     }

//     fn rebuild(
//         &self,
//         prev: &Self,
//         view_state: &mut Self::ViewState,
//         ctx: &mut ViewCtx,
//         mut element: Mut<'_, Self::Element>,
//         app_state: &mut State,
//     ) {
//         match (prev, self) {
//             (Self::Item(prev), Self::Item(this)) => {
//                 ctx.with_id(ViewId::new(view_state.generation), |ctx| {
//                     this.rebuild(prev, view_state.inner.as_mut().unwrap(), ctx, element, app_state);
//                 });
//             }
//             (Self::Spacer(prev), Self::Spacer(this)) => {
//                 View::<(), (), ViewCtx>::rebuild(this, prev, &mut (), ctx, element, &mut ());
//             }
//             (Self::Item(prev_flex_item), Self::Spacer(new_spacer)) => {
//                 // Run teardown with the old path
//                 ctx.with_id(ViewId::new(view_state.generation), |ctx| {
//                     prev_flex_item.teardown(
//                         view_state.inner.as_mut().unwrap(),
//                         ctx,
//                         FlexElementMut { parent: element.parent.reborrow_mut(), idx: element.idx },
//                     );
//                 });
//                 widgets::FloatingPanes::remove_child(&mut element.parent, element.idx);
//                 // The Flex item view has just been destroyed, teardown the old view
//                 // We increment the generation only on the falling edge (new item `FlexSpacer`) by convention
//                 // This choice has no impact on functionality
//                 view_state.inner = None;

//                 // Overflow handling: u64 starts at 0, incremented by 1 always.
//                 // Can never realistically overflow, scale is too large.
//                 // If would overflow, wrap to zero. Would need async message sent
//                 // to view *exactly* `u64::MAX` versions of the view ago, which is implausible
//                 view_state.generation = view_state.generation.wrapping_add(1);
//                 let (spacer_element, ()) = View::<(), (), ViewCtx>::build(new_spacer, ctx, &mut ());
//                 match spacer_element {
//                     FlexElement::FixedSpacer(len) => {
//                         widgets::FloatingPanes::insert_spacer(&mut element.parent, element.idx, len);
//                     }
//                     FlexElement::FlexSpacer(len) => {
//                         widgets::FloatingPanes::insert_flex_spacer(&mut element.parent, element.idx, len);
//                     }
//                     FlexElement::Child(_, _) => unreachable!(),
//                 };
//             }
//             (Self::Spacer(prev_spacer), Self::Item(new_flex_item)) => {
//                 View::<(), (), ViewCtx>::teardown(
//                     prev_spacer,
//                     &mut (),
//                     ctx,
//                     FlexElementMut { parent: element.parent.reborrow_mut(), idx: element.idx },
//                 );
//                 widgets::FloatingPanes::remove_child(&mut element.parent, element.idx);

//                 let (flex_item_element, child_state) = ctx
//                     .with_id(ViewId::new(view_state.generation), |ctx| new_flex_item.build(ctx, app_state));
//                 view_state.inner = Some(child_state);
//                 if let FlexElement::Child(child, params) = flex_item_element {
//                     widgets::FloatingPanes::insert_flex_child(
//                         &mut element.parent,
//                         element.idx,
//                         child.new_widget,
//                         params,
//                     );
//                 } else {
//                     unreachable!("We just created a new flex item, this should not be reached")
//                 }
//             }
//         }
//     }

//     fn teardown(&self, view_state: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
//         match self {
//             Self::Item(floating_pane_item) => {
//                 floating_pane_item.teardown(view_state.inner.as_mut().unwrap(), ctx, element);
//             }
//             Self::Spacer(spacer) => {
//                 View::<(), (), ViewCtx>::teardown(spacer, &mut (), ctx, element);
//             }
//         }
//     }

//     fn message(
//         &self,
//         view_state: &mut Self::ViewState,
//         message: &mut MessageContext,
//         element: Mut<'_, Self::Element>,
//         app_state: &mut State,
//     ) -> MessageResult<Action> {
//         let start = message.take_first().expect("Id path has elements for AnyFlexChild");
//         if start.routing_id() != view_state.generation {
//             // The message was sent to a previous edition of the inner value
//             return MessageResult::Stale;
//         }
//         let Self::Item(floating_pane_item) = self else {
//             unreachable!("this should be unreachable as the generation was increased on the falling edge")
//         };

//         floating_pane_item.message(view_state.inner.as_mut().unwrap(), message, element, app_state)
//     }
// }
