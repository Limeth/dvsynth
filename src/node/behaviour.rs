use crate::graph::ApplicationContext;
use crate::node::NodeConfiguration;
use crate::node::{ChannelValueRefs, ChannelValues};
use crate::style::Theme;
use downcast_rs::{impl_downcast, Downcast};
use iced::Element;
use iced_winit::winit::event_loop::EventLoopWindowTarget;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

pub use array_constructor::*;
pub use binary_op::*;
pub use constant::*;
pub use counter::*;
pub use debug::*;
pub use list_constructor::*;
pub use window::*;

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

pub trait NodeExecutorState: Downcast + Debug + Send + Sync {}
impl<T> NodeExecutorState for T where T: Downcast + Debug + Send + Sync {}
impl_downcast!(NodeExecutorState);

#[derive(Default, Copy, Clone)]
pub struct AllocatorHandle<'a> {
    __marker: PhantomData<&'a ()>,
}

static_assertions::const_assert_eq!(std::mem::size_of::<AllocatorHandle<'_>>(), 0);

pub struct ExecutionContext<'a, S: ?Sized> {
    pub application_context: &'a ApplicationContext,
    pub allocator_handle: AllocatorHandle<'a>,
    pub state: Option<&'a mut S>,
    pub inputs: &'a ChannelValueRefs<'a>,
    pub outputs: &'a mut ChannelValues,
}

impl<'a, S: ?Sized> ExecutionContext<'a, S> {
    pub fn map_state<R: ?Sized + 'a>(
        self,
        map: impl FnOnce(&'a mut S) -> &'a mut R,
    ) -> ExecutionContext<'a, R>
    {
        ExecutionContext {
            application_context: self.application_context,
            allocator_handle: self.allocator_handle,
            state: self.state.map(|state| (map)(state)),
            inputs: self.inputs,
            outputs: self.outputs,
        }
    }
}

pub type ExecutionContextContainer<'a> = ExecutionContext<'a, dyn NodeExecutorState>;

pub type NodeExecutorContainer = dyn Send + Sync + Fn(ExecutionContextContainer);

pub trait NodeExecutor<S>: 'static + Send + Sync {
    fn execute(&self, context: ExecutionContext<'_, S>);
}

pub type BoxNodeExecutor<S> = Box<dyn Send + Sync + for<'a> Fn(ExecutionContext<'a, S>)>;

impl<S: 'static> NodeExecutor<S> for BoxNodeExecutor<S> {
    fn execute(&self, context: ExecutionContext<'_, S>) {
        (self)(context)
    }
}

pub type NodeStateInitializerContainer =
    dyn Send + Sync + Fn(&ApplicationContext) -> Box<dyn NodeExecutorState>;

pub trait NodeStateInitializer<S>: 'static + Send + Sync {
    fn initialize_state(&self, context: &ApplicationContext) -> S;
}

pub type BoxNodeStateInitializer<S> = Box<dyn Send + Sync + Fn(&ApplicationContext) -> S>;

impl<S: 'static> NodeStateInitializer<S> for BoxNodeStateInitializer<S> {
    fn initialize_state(&self, context: &ApplicationContext) -> S {
        (self)(context)
    }
}

pub type MainThreadTask = dyn Send + FnOnce(&EventLoopWindowTarget<crate::Message>);

pub trait NodeBehaviourContainer {
    fn name(&self) -> &str;
    fn update(&mut self, event: NodeEventContainer) -> Vec<NodeCommand>;
    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>>;
    fn create_executor(&self) -> Arc<NodeExecutorContainer>;
    fn create_state_initializer(&self) -> Option<Arc<NodeStateInitializerContainer>>;
}

pub trait NodeBehaviour {
    type Message: NodeBehaviourMessage;
    type State: NodeExecutorState;
    type FnStateInitializer: NodeStateInitializer<Self::State> = BoxNodeStateInitializer<Self::State>;
    type FnExecutor: NodeExecutor<Self::State> = BoxNodeExecutor<Self::State>;

    fn name(&self) -> &str;
    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand>;
    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Self::Message>>;
    fn create_executor(&self) -> Self::FnExecutor;

    fn create_state_initializer(&self) -> Option<Self::FnStateInitializer> {
        None
    }
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

    fn create_state_initializer(&self) -> Option<Arc<NodeStateInitializerContainer>> {
        NodeBehaviour::create_state_initializer(self).map(|initializer| {
            Arc::new(move |context: &ApplicationContext| {
                let state = initializer.initialize_state(context);

                Box::new(state) as Box<dyn NodeExecutorState>
            }) as Arc<NodeStateInitializerContainer>
        })
    }

    fn create_executor(&self) -> Arc<NodeExecutorContainer> {
        let typed_executor = NodeBehaviour::create_executor(self);

        Arc::new(move |context: ExecutionContextContainer<'_>| {
            let context =
                context.map_state(|state| state.downcast_mut::<<Self as NodeBehaviour>::State>().unwrap());

            typed_executor.execute(context)
        })
    }
}

pub mod array_constructor;
pub mod binary_op;
pub mod constant;
pub mod counter;
pub mod debug;
pub mod list_constructor;
pub mod window;
