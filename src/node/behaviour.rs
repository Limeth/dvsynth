use crate::graph::{ApplicationContext, NodeIndex};
use crate::node::{ChannelValueRefs, ChannelValues, DynTypeTrait, NodeConfiguration};
use crate::style::Theme;
use downcast_rs::{impl_downcast, Downcast};
use iced::Element;
use iced_winit::winit::event_loop::EventLoopWindowTarget;
use std::fmt::Debug;
use std::marker::PhantomData;

pub use array_constructor::*;
pub use binary_op::*;
pub use constant::*;
pub use counter::*;
pub use debug::*;
pub use list_constructor::*;
pub use window::*;

use super::{OwnedRefMut, SizedTypeExt, TypeEnum, TypeTrait};

pub struct Input {
    pub data: Box<[u8]>,
    pub ty: TypeEnum,
}

pub struct Inputs {}

pub struct Outputs {}

pub enum NodeCommand {
    Configure(NodeConfiguration),
}

pub trait NodeBehaviourMessage: Downcast + Debug + Send {
    fn dyn_clone(&self) -> Box<dyn NodeBehaviourMessage>;
}

impl_downcast!(NodeBehaviourMessage);

macro_rules! impl_node_behaviour_message {
    ($($target_type:tt)*) => {
        impl crate::node::behaviour::NodeBehaviourMessage for $($target_type)* {
            fn dyn_clone(&self) -> Box<dyn crate::node::behaviour::NodeBehaviourMessage> {
                Box::new(self.clone())
            }
        }
    };
}

impl_node_behaviour_message!(());

impl Clone for Box<dyn NodeBehaviourMessage> {
    fn clone(&self) -> Self {
        NodeBehaviourMessage::dyn_clone(self.as_ref())
    }
}

#[derive(Debug, Clone)]
pub enum NodeEvent<M> {
    Update,
    Message(M),
}

pub type NodeEventContainer = NodeEvent<Box<dyn NodeBehaviourMessage>>;

impl<M: NodeBehaviourMessage> NodeEvent<M> {
    pub fn from_container(container: NodeEventContainer) -> Result<Self, ()> {
        Ok(match container {
            NodeEvent::Message(message) => match message.downcast::<M>() {
                Ok(message) => NodeEvent::Message(*message),
                Err(_) => return Err(()),
            },
            NodeEvent::Update => NodeEvent::Update,
        })
    }

    pub fn into_container(self) -> NodeEventContainer {
        self.map_message(|message| Box::new(message) as Box<dyn NodeBehaviourMessage>)
    }

    pub fn map_message<R>(self, map: impl FnOnce(M) -> R) -> NodeEvent<R> {
        match self {
            NodeEvent::Message(message) => NodeEvent::Message((map)(message)),
            NodeEvent::Update => NodeEvent::Update,
        }
    }
}

// FIXME: Maybe just store `Box<dyn NodeExecutor<'static>>` instead?
pub struct NodeExecutorStateContainer<'state> {
    ptr: Box<dyn NodeExecutor<'state> + 'state>,
}

impl<'state> NodeExecutorStateContainer<'state> {
    pub fn from<T: NodeBehaviour>(state: T::State<'state>) -> Self {
        Self { ptr: Box::new(state) as Box<dyn NodeExecutor<'state> + 'state> }
    }

    /// Safety: The returned value must not outlive self.
    unsafe fn as_trait_object(&mut self) -> std::raw::TraitObject {
        let raw: *mut dyn NodeExecutor<'state> = &mut *self.ptr as *mut _;

        std::mem::transmute(raw)
    }

    unsafe fn downcast_mut<T: NodeBehaviour>(&mut self) -> &mut T::State<'state> {
        let trait_object = self.as_trait_object();
        &mut *(trait_object.data as *mut T::State<'state>)
    }

    pub fn update<'invocation, T: NodeBehaviour>(
        &'invocation mut self,
        context: &'invocation ApplicationContext,
        behaviour: &T,
    ) where
        'state: 'invocation,
    {
        let state = unsafe { self.downcast_mut::<T>() };

        state.update(context, behaviour)
    }

    pub fn execute<'invocation>(&'invocation mut self, context: ExecutionContext<'invocation, 'state>)
    where 'state: 'invocation {
        self.ptr.execute(context);
    }
}

pub trait NodeExecutor<'state>: Debug + Send + Sync {
    fn execute<'invocation>(&'invocation mut self, context: ExecutionContext<'invocation, 'state>)
    where 'state: 'invocation;
}

pub trait NodeExecutorState<'state>: NodeExecutor<'state> {
    type Behaviour: NodeBehaviour;

    fn update<'invocation>(
        &'invocation mut self,
        context: &'invocation ApplicationContext,
        behaviour: &Self::Behaviour,
    ) where
        'state: 'invocation;
}

pub trait TransientTrait = Send + Sync;
pub trait ExecutorClosureConstructor<'state, T, Transient = ()> = Fn(&T, &ApplicationContext, &mut Transient) -> Box<dyn ExecutorClosure<'state, Transient> + 'state>
    + Send
    + Sync
where Transient: TransientTrait + 'state;
pub trait ExecutorClosure<'state, Transient = ()> =
    for<'i> FnMut(ExecutionContext<'i, 'state>, &mut Transient) + Send + Sync
    where Transient: TransientTrait + 'state;

/// A `NodeExecutorState`, such that is created using:
/// * The `create_closure` executor constructor, which constructs the executor using `&T` and `&ApplicationContext`;
/// * The `transient` data, which is the state persisted across calls to `create_closure`.
pub struct NodeExecutorStateClosure<'state, T, Transient = ()>
where
    T: NodeBehaviour,
    Transient: TransientTrait + 'state,
{
    create_closure: Box<dyn ExecutorClosureConstructor<'state, T, Transient> + 'state>,
    execute: Box<dyn ExecutorClosure<'state, Transient> + 'state>,
    transient: Transient,
}

impl<'state, T, Transient> NodeExecutorStateClosure<'state, T, Transient>
where
    T: NodeBehaviour,
    Transient: TransientTrait + 'state,
{
    pub fn new<'invocation>(
        behaviour: &'invocation T,
        context: &'invocation ApplicationContext,
        transient: Transient,
        create_closure: impl ExecutorClosureConstructor<'state, T, Transient> + 'state,
    ) -> Self
    where
        'state: 'invocation,
    {
        Self::from_box(behaviour, context, transient, Box::new(create_closure))
    }

    fn from_box<'invocation>(
        behaviour: &'invocation T,
        context: &'invocation ApplicationContext,
        mut transient: Transient,
        create_closure: Box<dyn ExecutorClosureConstructor<'state, T, Transient> + 'state>,
    ) -> Self
    where
        'state: 'invocation,
    {
        Self { execute: (create_closure)(behaviour, context, &mut transient), create_closure, transient }
    }
}

impl<'state, T, Transient> Debug for NodeExecutorStateClosure<'state, T, Transient>
where
    T: NodeBehaviour,
    Transient: TransientTrait + 'state,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NodeExecutorStateClosure")
    }
}

impl<'state, T, Transient> NodeExecutorState<'state> for NodeExecutorStateClosure<'state, T, Transient>
where
    T: NodeBehaviour,
    Transient: TransientTrait + 'state,
{
    type Behaviour = T;

    fn update<'invocation>(
        &'invocation mut self,
        context: &'invocation ApplicationContext,
        behaviour: &Self::Behaviour,
    ) where
        'state: 'invocation,
    {
        self.execute = (self.create_closure)(behaviour, context, &mut self.transient)
    }
}

impl<'state, T, Transient> NodeExecutor<'state> for NodeExecutorStateClosure<'state, T, Transient>
where
    T: NodeBehaviour,
    Transient: TransientTrait + 'state,
{
    fn execute<'invocation>(&'invocation mut self, context: ExecutionContext<'invocation, 'state>)
    where 'state: 'invocation {
        (self.execute)(context, &mut self.transient)
    }
}

/// Makes it possible for tasks (nodes) to dynamically allocate data
/// that can be shared with other tasks via channels.
#[derive(Clone, Copy)]
pub struct AllocatorHandle<'invocation, 'state: 'invocation> {
    pub(crate) node: NodeIndex,
    __marker: PhantomData<(&'invocation (), &'state ())>,
}

impl<'invocation, 'state: 'invocation> AllocatorHandle<'invocation, 'state> {
    pub(crate) unsafe fn with_node_index(node: NodeIndex) -> Self {
        Self { node, __marker: Default::default() }
    }
}

/// Must not be `Send`, because then the `'invocation` lifetime would not be enforced.
/// The user could send the handle to another thread, letting the current invocation complete
/// and the handle outlive the completed invocation, which would be a bug.
impl !Send for AllocatorHandle<'_, '_> {}
impl !Sync for AllocatorHandle<'_, '_> {}

impl<'invocation, 'state: 'invocation> AllocatorHandle<'invocation, 'state> {
    pub fn allocate_object<T: DynTypeTrait>(self, descriptor: T::Descriptor) -> OwnedRefMut<'state, T> {
        OwnedRefMut::allocate_object(descriptor, self)
    }

    pub fn allocate_bytes<T: TypeTrait + SizedTypeExt>(self, ty: T) -> OwnedRefMut<'state, T> {
        OwnedRefMut::allocate_bytes(ty, self)
    }
}

pub struct ExecutionContext<'invocation, 'state: 'invocation> {
    pub application_context: &'invocation ApplicationContext,
    pub allocator_handle: AllocatorHandle<'invocation, 'state>,
    pub inputs: &'invocation ChannelValueRefs<'invocation>,
    pub outputs: &'invocation mut ChannelValues,
}

pub type MainThreadTask = dyn Send + FnOnce(&EventLoopWindowTarget<crate::Message>);

pub trait NodeBehaviourContainer: dyn_clone::DynClone + std::fmt::Debug + Send + Sync + 'static {
    fn name(&self) -> &str;
    fn update(&mut self, event: NodeEventContainer) -> Vec<NodeCommand>;
    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>>;
    // fn create_executor(&self) -> Arc<NodeExecutorContainer>;
    // fn create_state_initializer(&self) -> Option<Arc<dyn NodeStateInitializerContainer>>;
    fn create_state<'state>(&self, context: &ApplicationContext) -> NodeExecutorStateContainer<'state>;
    fn update_state<'state>(
        &self,
        context: &ApplicationContext,
        state: &mut NodeExecutorStateContainer<'state>,
    );
}

dyn_clone::clone_trait_object!(NodeBehaviourContainer);

pub trait NodeBehaviour: std::fmt::Debug + Clone + Send + Sync + 'static {
    type Message: NodeBehaviourMessage;
    type State<'state>: NodeExecutorState<'state, Behaviour = Self>;

    fn name(&self) -> &str;
    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand>;
    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Self::Message>>;
    fn create_state<'state>(&self, context: &ApplicationContext) -> Self::State<'state>;
}

impl<T: NodeBehaviour> NodeBehaviourContainer for T {
    fn name(&self) -> &str {
        NodeBehaviour::name(self)
    }

    fn update(&mut self, event: NodeEventContainer) -> Vec<NodeCommand> {
        NodeBehaviour::update(self, NodeEvent::from_container(event).unwrap())
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>> {
        NodeBehaviour::view(self, theme)
            .map(|element| element.map(|message| Box::new(message) as Box<dyn NodeBehaviourMessage>))
    }

    fn create_state<'state>(&self, context: &ApplicationContext) -> NodeExecutorStateContainer<'state> {
        let state = <Self as NodeBehaviour>::create_state(self, context);

        NodeExecutorStateContainer::from::<Self>(state)
    }

    fn update_state<'state>(
        &self,
        context: &ApplicationContext,
        state: &mut NodeExecutorStateContainer<'state>,
    )
    {
        state.update::<Self>(context, self)
    }
}

pub mod array_constructor;
pub mod binary_op;
pub mod constant;
pub mod counter;
pub mod debug;
pub mod list_constructor;
pub mod window;
