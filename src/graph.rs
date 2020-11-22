use crate::node::*;
use crate::style::{self, consts, Theme, Themeable};
use crate::widgets::{
    node::FloatingPanesBehaviour, FloatingPane, FloatingPaneBehaviourData, FloatingPaneBehaviourState,
    FloatingPaneState, NodeElement, NodeElementState,
};
use crate::ApplicationFlags;
use crate::Message;
use crate::NodeMessage;
use arc_swap::ArcSwapOption;
use dyn_clone::DynClone;
use iced::{Element, Settings};
use iced_futures::futures;
use iced_wgpu::wgpu;
use iced_winit::winit::event_loop::EventLoopWindowTarget;
use iced_winit::winit::platform::desktop::EventLoopExtDesktop;
use iced_winit::winit::window::{Window, WindowAttributes};
use petgraph::{stable_graph::StableGraph, visit::EdgeRef, Directed, Direction};
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use vek::Vec2;

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;
pub type Graph = StableGraph<
    NodeData, // Node Data
    EdgeData, // Edge Data
    Directed, // Edge Type
    u32,      // Node Index
>;

pub struct PreparedTask {
    pub node_index: NodeIndex,
    pub state: Option<Box<dyn NodeExecutorState>>,
    pub output_values: ChannelValues,
}

/// Data ready for the execution of a [`Schedule`].
/// Accessible by all render threads.
pub struct PreparedExecution {
    pub generation: usize,
    pub tasks: Box<[RwLock<PreparedTask>]>,
}

static_assertions::assert_impl_all!(Arc<PreparedExecution>: Send, Sync);

impl PreparedExecution {
    fn from(schedule: &Schedule, context: &mut ExecutionContext, mut previous: Option<Self>) -> Self {
        let previous_node_index_map: Option<HashMap<NodeIndex, usize>> =
            previous.as_ref().map(|prepared_execution| {
                prepared_execution
                    .tasks
                    .iter()
                    .enumerate()
                    .map(|(enumeration_index, task)| (task.read().unwrap().node_index, enumeration_index))
                    .collect()
            });

        Self {
            generation: schedule.generation,
            tasks: schedule
                .tasks
                .iter()
                .map(|task| {
                    let state = previous_node_index_map
                        .as_ref()
                        .and_then(|previous_node_index_map| previous_node_index_map.get(&task.node_index))
                        .and_then(|task_index| {
                            let previous_task =
                                &mut previous.as_mut().unwrap().tasks[*task_index].write().unwrap();

                            previous_task.state.take()
                        })
                        .or_else(|| {
                            task.state_initializer
                                .as_ref()
                                .map(|state_initializer| (state_initializer)(&context))
                        });

                    RwLock::new(PreparedTask {
                        node_index: task.node_index,
                        state,
                        output_values: ChannelValues::zeroed(&task.configuration.channels_output),
                    })
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

impl PreparedExecution {
    pub fn execute(&mut self, schedule: &Schedule, context: &mut ExecutionContext) {
        for (task_index, task) in schedule.tasks.iter().enumerate() {
            let (tasks_preceding, tasks_following) = self.tasks.split_at_mut(task_index);
            let input_value_guards = task
                .inputs
                .iter()
                .map(|input| tasks_preceding[input.source_task_index].read().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let input_values = ChannelValueRefs {
                values: input_value_guards
                    .iter()
                    .zip(&*task.inputs)
                    .map(|(input_value_guard, input)| {
                        input_value_guard.output_values.values[input.source_channel_index]
                            .as_channel_value_ref()
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            };

            let current_task: &mut PreparedTask = &mut tasks_following[0].write().unwrap();
            let output_values = &mut current_task.output_values;
            let task_state = current_task.state.as_mut().map(|state| state.as_mut());

            (task.executor)(&context, task_state, &input_values, output_values);
        }
    }
}

#[derive(Clone, Debug)]
pub struct TaskInput {
    pub source_task_index: usize,
    pub source_channel_index: usize,
}

#[derive(Clone)]
pub struct Task {
    pub node_index: NodeIndex,
    pub configuration: NodeConfiguration,
    pub state_initializer: Option<Arc<NodeStateInitializerContainer>>,
    pub inputs: Box<[TaskInput]>,
    pub executor: Arc<NodeExecutorContainer>,
}

// impl Clone for Task {
//     fn clone(&self) -> Self {
//         Self {
//             node_index: self.node_index.clone(),
//             configuration: self.configuration.clone(),
//             init_state: self.init_state.as_ref().map(|state| dyn_clone::clone_box(&**state)),
//             inputs: self.inputs.clone(),
//             executor: self.executor.clone(),
//         }
//     }
// }

impl Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("node_index", &self.node_index)
            .field("configuration", &self.configuration)
            .field("state_initializer.is_some()", &self.state_initializer.is_some())
            .field("inputs", &self.inputs)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct Schedule {
    /// Used to check whether the schedule has been updated
    pub generation: usize,
    // FIXME: implement proper multithreaded scheduling
    pub tasks: Box<[Task]>,
}

pub struct ExecutionGraph {
    pub graph: Graph,
    pub active_schedule: Arc<ArcSwapOption<Schedule>>,
}

impl ExecutionGraph {
    pub fn is_graph_complete(&self) -> bool {
        for node_index in self.node_indices() {
            let node = self.node_weight(node_index);
            let node = node.as_ref().unwrap();
            let mut input_channels =
                (0..node.configuration.channels_input.len()).into_iter().collect::<HashSet<_>>();

            for edge_ref in self.edges_directed(node_index, Direction::Incoming) {
                let edge = edge_ref.weight();
                let source_index = edge_ref.source();
                let source_node: &NodeData = self.node_weight(source_index).unwrap();
                let source_channel =
                    source_node.configuration.channel(ChannelDirection::Out, edge.channel_index_from);
                let target_channel = node.configuration.channel(ChannelDirection::In, edge.channel_index_to);

                if source_channel.ty.is_abi_compatible(&target_channel.ty) {
                    input_channels.remove(&edge.channel_index_to);
                }
            }

            if !input_channels.is_empty() {
                return false;
            }
        }

        true
    }

    fn create_schedule(&mut self) -> Result<Schedule, ()> {
        if !self.is_graph_complete() {
            return Err(());
        }

        let ordered_node_indices = match petgraph::algo::toposort(&self.graph, None) {
            Ok(ordered_node_indices) => ordered_node_indices,
            Err(cycle) => {
                return Err(());
            }
        };

        let node_index_map: HashMap<NodeIndex, usize> = ordered_node_indices
            .iter()
            .enumerate()
            .map(|(enumeration_index, node_index)| (*node_index, enumeration_index))
            .collect();

        let tasks = ordered_node_indices
            .into_iter()
            .map(|node_index| {
                {
                    let mut node = self.node_weight_mut(node_index);
                    let node = node.as_mut().unwrap();

                    node.ready_output_values();
                }

                let node = self.node_weight(node_index);
                let node = node.as_ref().unwrap();
                let inputs = {
                    let mut inputs: Vec<Option<TaskInput>> =
                        vec![None; node.configuration.channels_input.len()];

                    for edge_index in self.edge_indices() {
                        let (from_index, to_index) = self.edge_endpoints(edge_index).unwrap();

                        if to_index == node_index {
                            let edge = self.edge_weight(edge_index).unwrap();

                            inputs[edge.channel_index_to] = Some(TaskInput {
                                source_task_index: *node_index_map.get(&from_index).unwrap(),
                                source_channel_index: edge.channel_index_from,
                            });
                        }
                    }

                    inputs
                        .into_iter()
                        .map(|value| value.expect("An input channel is missing a value."))
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                };

                Task {
                    node_index,
                    configuration: node.configuration.clone(),
                    state_initializer: node.behaviour.create_state_initializer(),
                    inputs,
                    executor: node.behaviour.create_executor(),
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Schedule {
            generation: self
                .active_schedule
                .load()
                .as_ref()
                .map(|schedule| schedule.generation.wrapping_add(1))
                .unwrap_or(0),
            tasks,
        })
    }

    pub fn update_schedule(&mut self) -> Result<(), ()> {
        match self.create_schedule() {
            Ok(schedule) => {
                self.active_schedule.store(Some(Arc::new(schedule)));
                Ok(())
            }
            Err(e) => {
                self.active_schedule.store(None);
                Err(e)
            }
        }
    }
}

impl From<Graph> for ExecutionGraph {
    fn from(graph: Graph) -> Self {
        Self { graph, active_schedule: Default::default() }
    }
}

impl Deref for ExecutionGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}

impl DerefMut for ExecutionGraph {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graph
    }
}

pub struct Renderer {
    pub instance: Arc<wgpu::Instance>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl Renderer {
    pub fn new(settings: &Settings<ApplicationFlags>) -> Self {
        let instance = Arc::new(wgpu::Instance::new(wgpu::BackendBit::PRIMARY));
        let (device, queue) = {
            let adapter =
                futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: if !settings.antialiasing {
                        wgpu::PowerPreference::Default
                    } else {
                        wgpu::PowerPreference::HighPerformance
                    },
                    compatible_surface: None,
                }))
                .expect("No wgpu compatible adapter available.");

            let (device, queue) = futures::executor::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits { max_bind_groups: 2, ..wgpu::Limits::default() },
                    shader_validation: false,
                },
                None,
            ))
            .expect("No wgpu compatible device available.");

            (Arc::new(device), Arc::new(queue))
        };

        Self { instance, device, queue }
    }
}

pub enum TextureAllocation {
    TextureView(wgpu::TextureView),
    SwapchainFrame(wgpu::SwapChainFrame),
}

impl Deref for TextureAllocation {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &Self::Target {
        match self {
            TextureAllocation::TextureView(texture_view) => texture_view,
            TextureAllocation::SwapchainFrame(swapchain_frame) => &swapchain_frame.output.view,
        }
    }
}

pub struct ListAllocation {
    item_type: ChannelType,
    data: Vec<u8>,
    item_size: usize,
}

impl ListAllocation {
    pub fn new(item_type: impl Into<ChannelType>) -> Self {
        let item_type = item_type.into();
        Self { item_size: item_type.value_size(), item_type, data: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.data.len() / self.item_size
    }

    pub fn push(&mut self, item: &[u8]) {
        assert_eq!(item.len(), self.item_size);
        self.data.extend_from_slice(item);
    }

    pub fn pop(&mut self) -> Result<(), ()> {
        if self.data.len() > 0 {
            self.data.truncate(self.data.len() - self.item_size);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn get(&self, index: usize) -> Option<&[u8]> {
        let start_index = index * self.item_size;
        let end_index = (index + 1) * self.item_size;

        if end_index >= self.data.len() {
            Some(&self.data[start_index..end_index])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        let start_index = index * self.item_size;
        let end_index = (index + 1) * self.item_size;

        if end_index >= self.data.len() {
            Some(&mut self.data[start_index..end_index])
        } else {
            None
        }
    }
}

impl Deref for ListAllocation {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for ListAllocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Eq, PartialEq, Debug)]
#[repr(C)]
pub struct AllocationPointer {
    index: u32,
}

impl AllocationPointer {
    fn as_u32(&self) -> u32 {
        self.index
    }

    fn as_usize(&self) -> usize {
        self.index as usize
    }
}

impl From<usize> for AllocationPointer {
    fn from(index: usize) -> Self {
        Self { index: index as u32 }
    }
}

struct Allocation {
    ptr: Box<dyn Any>,
    refcount: AtomicUsize,
}

impl Allocation {
    pub fn new<T: Any>(value: T) -> Self {
        Self { ptr: Box::new(value), refcount: AtomicUsize::new(1) }
    }
}

pub struct Allocator {
    allocations: Vec<Option<Allocation>>,
    freed_allocation_indices: Vec<usize>,
}

impl Allocator {
    pub fn allocate<T: Any>(&mut self, allocation: T) -> AllocationPointer {
        let allocation = Some(Allocation::new(allocation));

        if let Some(freed_allocation_index) = self.freed_allocation_indices.pop() {
            self.allocations[freed_allocation_index] = allocation;
            freed_allocation_index
        } else {
            self.allocations.push(allocation);
            self.allocations.len() - 1
        }
        .into()
    }

    pub fn deallocate(&mut self, allocation_ptr: AllocationPointer) {
        self.allocations[allocation_ptr.as_usize()] = None;
        self.freed_allocation_indices.push(allocation_ptr.as_usize());
    }

    /// Add `delta` to refcount and deallocate, if zero.
    /// Returns `Ok(true)` when the allocation has been freed,
    /// `Ok(false)` resulting refcount is larger than 0,
    /// or `Err` if no such allocation exists.
    pub fn refcount_update(&mut self, allocation_ptr: AllocationPointer, delta: isize) -> Result<bool, ()> {
        if let Some(allocation) = self.allocations[allocation_ptr.as_usize()].as_ref() {
            let refcount = &allocation.refcount;
            if delta > 0 {
                refcount.fetch_add(delta as usize, Ordering::SeqCst);
                Ok(false)
            } else if delta < 0 {
                let mut refcount_before_swap = refcount.load(Ordering::SeqCst);
                let refcount_new;

                loop {
                    refcount_new = refcount_before_swap.saturating_sub((-delta) as usize);
                    let refcount_during_swap =
                        refcount.compare_and_swap(refcount_before_swap, refcount_new, Ordering::SeqCst);

                    if refcount_during_swap == refcount_before_swap {
                        break;
                    } else {
                        refcount_before_swap = refcount_during_swap;
                    }
                }

                if refcount_before_swap > 0 && refcount_new == 0 {
                    self.deallocate(allocation_ptr);
                    Ok(true)
                } else {
                    // Deallocation was already performed (before_swap == 0) or was not necessary (new > 0).
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Err(())
        }
    }

    pub fn deref(&self, allocation_ptr: AllocationPointer) -> Option<&Box<dyn Any>> {
        self.allocations
            .get(allocation_ptr.as_usize())
            .and_then(|allocation| allocation.as_ref().map(|allocation| &allocation.ptr))
    }

    pub fn deref_mut(&mut self, allocation_ptr: AllocationPointer) -> Option<&mut Box<dyn Any>> {
        self.allocations
            .get_mut(allocation_ptr.as_usize())
            .and_then(|allocation| allocation.as_mut().map(|allocation| &mut allocation.ptr))
    }
}

pub struct ExecutionContext {
    pub main_thread_task_sender: Sender<Box<MainThreadTask>>,
    pub renderer: Renderer,
    pub allocator: Allocator,
}

impl ExecutionContext {
    pub fn new(renderer: Renderer) -> (Self, Receiver<Box<MainThreadTask>>) {
        let (main_thread_task_sender, main_thread_task_receiver) = mpsc::channel();
        let context = Self { main_thread_task_sender, renderer, allocators: Defualt::default() };
        (context, main_thread_task_receiver)
    }

    pub fn from_settings(settings: &Settings<ApplicationFlags>) -> (Self, Receiver<Box<MainThreadTask>>) {
        Self::new(Renderer::new(settings))
    }
}

pub struct GraphExecutor {
    execution_context: ExecutionContext,
    active_schedule: Arc<ArcSwapOption<Schedule>>,
}

impl GraphExecutor {
    pub fn new(execution_context: ExecutionContext, active_schedule: Arc<ArcSwapOption<Schedule>>) -> Self {
        Self { active_schedule, execution_context }
    }

    pub fn spawn(
        execution_context: ExecutionContext,
        active_schedule: Arc<ArcSwapOption<Schedule>>,
    ) -> std::thread::JoinHandle<()>
    {
        thread::spawn(move || Self::new(execution_context, active_schedule).run())
    }

    pub fn run(mut self) {
        let mut prepared_execution: Option<PreparedExecution> = None;
        let mut last_prepared_execution: Option<PreparedExecution> = None;

        loop {
            if let Some(active_schedule) = self.active_schedule.load().as_ref() {
                if prepared_execution.is_none()
                    || prepared_execution.as_ref().unwrap().generation != active_schedule.generation
                {
                    prepared_execution = Some(PreparedExecution::from(
                        &active_schedule,
                        &mut self.execution_context,
                        prepared_execution.or(last_prepared_execution.take()),
                    ));
                }

                let prepared_execution = prepared_execution.as_mut().unwrap();

                prepared_execution.execute(active_schedule, &mut self.execution_context);
            } else {
                if let Some(prepared_execution) = prepared_execution.take() {
                    last_prepared_execution = Some(prepared_execution);
                }
            }
        }
    }
}

pub struct NodeData {
    pub title: String,
    pub element_state: NodeElementState,
    pub floating_pane_state: FloatingPaneState,
    pub floating_pane_behaviour_state: FloatingPaneBehaviourState,
    pub behaviour: Box<dyn NodeBehaviourContainer>,
    pub configuration: NodeConfiguration,
    /// Output values computed during graph execution.
    pub execution_output_values: Option<RefCell<ChannelValues>>,
}

impl NodeData {
    pub fn new(
        title: impl ToString,
        position: impl Into<Vec2<f32>>,
        behaviour: Box<dyn NodeBehaviourContainer>,
    ) -> Self
    {
        let mut result = Self {
            title: title.to_string(),
            element_state: Default::default(),
            floating_pane_state: FloatingPaneState::new().with_position(position).with_width(200),
            floating_pane_behaviour_state: Default::default(),
            configuration: Default::default(),
            behaviour,
            execution_output_values: None,
        };

        result.update(NodeEventContainer::Update);

        result
    }

    pub fn update(&mut self, event: NodeEventContainer) {
        for command in self.behaviour.update(event) {
            match command {
                NodeCommand::Configure(configuration) => self.configuration = configuration,
            }
        }
    }

    pub fn ready_output_values(&mut self) {
        self.execution_output_values =
            Some(RefCell::new(ChannelValues::zeroed(&self.configuration.channels_output)));
    }

    pub fn view(
        &mut self,
        index: NodeIndex,
        theme: &dyn Theme,
    ) -> FloatingPane<'_, Message, iced_wgpu::Renderer, FloatingPanesBehaviour<Message>>
    {
        let mut builder = NodeElement::builder(index, &mut self.element_state).node_behaviour_element(
            self.behaviour.view(theme).map(Element::from).map(move |element| {
                element.map(move |message| Message::NodeMessage {
                    node: index,
                    message: NodeMessage::NodeBehaviourMessage(message),
                })
            }),
        );

        for input_channel in &self.configuration.channels_input {
            builder = builder.push_input_channel(input_channel);
        }

        for output_channel in &self.configuration.channels_output {
            builder = builder.push_output_channel(output_channel);
        }

        let node_element = builder.build(/*|index, new_value| {
            Message::NodeMessage {
                node: index,
                message: NodeMessage::UpdateTextInput(new_value),
            }
        }*/);

        Themeable::theme(
            FloatingPane::builder(
                node_element,
                &mut self.floating_pane_state,
                &mut self.floating_pane_behaviour_state,
                FloatingPaneBehaviourData { node_configuration: self.configuration.clone() },
            ),
            theme,
        )
        .title(Some(&self.title))
        .title_size(Some(style::consts::TEXT_SIZE_TITLE))
        .title_margin(consts::SPACING)
        .width_resizeable(true)
        .min_width(128.0)
        .build()
    }
}

pub struct EdgeData {
    pub channel_index_from: usize,
    pub channel_index_to: usize,
}

impl EdgeData {
    pub fn get_channel_index(&self, direction: ChannelDirection) -> usize {
        match direction {
            ChannelDirection::In => self.channel_index_from,
            ChannelDirection::Out => self.channel_index_to,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelIdentifier {
    pub node_index: NodeIndex,
    pub channel_direction: ChannelDirection,
    pub channel_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection(pub [(NodeIndex, usize); 2]);

impl Connection {
    pub fn try_from_identifiers([a, b]: [ChannelIdentifier; 2]) -> Option<Connection> {
        if a.channel_direction == b.channel_direction {
            None
        } else {
            Some(Self(if a.channel_direction == ChannelDirection::Out {
                [(a.node_index, a.channel_index), (b.node_index, b.channel_index)]
            } else {
                [(b.node_index, b.channel_index), (a.node_index, a.channel_index)]
            }))
        }
    }

    pub fn contains_channel(&self, channel: ChannelIdentifier) -> bool {
        let index = match channel.channel_direction {
            ChannelDirection::In => 1,
            ChannelDirection::Out => 0,
        };
        let current = &self.0[index];

        current.0 == channel.node_index && current.1 == channel.channel_index
    }

    pub fn channel(&self, direction: ChannelDirection) -> ChannelIdentifier {
        let index = match direction {
            ChannelDirection::In => 1,
            ChannelDirection::Out => 0,
        };
        ChannelIdentifier {
            node_index: self.0[index].0,
            channel_direction: direction,
            channel_index: self.0[index].1,
        }
    }

    pub fn to(&self) -> ChannelIdentifier {
        self.channel(ChannelDirection::In)
    }

    pub fn from(&self) -> ChannelIdentifier {
        self.channel(ChannelDirection::Out)
    }
}
