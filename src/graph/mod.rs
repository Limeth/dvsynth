use crate::graph::alloc::AllocationInner;
use crate::node::behaviour::{
    AllocatorHandle, ExecutionContext, MainThreadTask, NodeBehaviourContainer, NodeCommand,
    NodeEventContainer, NodeStateContainer,
};
use crate::node::ty::{BorrowedRef, BorrowedRefMut, OptionRefExt, OptionType, TypeEnum, TypeExt};
use crate::node::{
    ChannelDirection, ChannelPassBy, ChannelRef, ChannelValueRefs, ChannelValues, ConnectionPassBy,
    DynTypeTrait, ListDescriptor, NodeConfiguration, NodeStateRefcounter, OptionRefMutExt, RefAnyExt,
};
use crate::style::{self, consts, Theme, Themeable};
use crate::widgets::{
    node::FloatingPanesBehaviour, FloatingPane, FloatingPaneBehaviourData, FloatingPaneBehaviourState,
    FloatingPaneState, NodeElement, NodeElementState,
};
use crate::ApplicationFlags;
use crate::Message;
use crate::NodeMessage;
use alloc::Allocator;
use arc_swap::ArcSwapOption;
use iced::{Element, Settings};
use iced_futures::futures;
use iced_wgpu::wgpu;
use petgraph::{stable_graph::StableGraph, visit::EdgeRef, Directed, Direction};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use vek::Vec2;

pub mod alloc;

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;
pub type Graph = StableGraph<
    NodeData, // Node Data
    EdgeData, // Edge Data
    Directed, // Edge Type
    u32,      // Node Index
>;

pub struct PreparedTask {
    pub node_index: NodeIndex,
    /// Set to `None` only during the preparation of the next schedule, for the previous schedule's
    /// tasks.
    pub state: Option<NodeStateContainer<'static>>,
    /// A list of `OptionType`-wrapped outputs.
    ///
    /// Provided as inputs by:
    /// * move:              BorrowedRefMut<OptionType<T>> (allows for T to be taken out of the Option)
    /// * mutable reference: BorrowedRefMut<T>
    /// * shared reference:  BorrowedRef<T>
    ///
    /// Provided as outputs by move (BorrowedRefMut<OptionType<T>>). After the task has finished
    /// executing, the value must be present.
    pub output_values: Box<[RwLock<AllocationInner>]>,
}

impl PreparedTask {
    pub fn from(task: &Task, state: NodeStateContainer<'static>) -> Self {
        Self {
            node_index: task.node_index,
            state: Some(state),
            output_values: task
                .configuration
                .output_channels_by_value
                .iter()
                .map(|channel| {
                    RwLock::new(
                        AllocationInner::from_enum_if_sized(
                            OptionType::from_enum_if_sized(channel.ty.clone()).unwrap(),
                        )
                        .unwrap(),
                    )
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

/// Data ready for the execution of a [`Schedule`].
/// Accessible by all render threads.
pub struct PreparedExecution {
    pub generation: usize,
    pub tasks: Box<[Option<RwLock<PreparedTask>>]>,
}

static_assertions::assert_impl_all!(Arc<PreparedExecution>: Send, Sync);

impl PreparedExecution {
    fn from(schedule: &Schedule, context: &mut ApplicationContext, mut previous: Option<Self>) -> Self {
        Allocator::get().prepare_for_schedule(schedule);
        let previous_node_index_map: Option<HashMap<NodeIndex, usize>> =
            previous.as_ref().map(|prepared_execution| {
                prepared_execution
                    .tasks
                    .iter()
                    .enumerate()
                    .filter_map(|(enumeration_index, task)| {
                        task.as_ref().map(|task| (enumeration_index, task))
                    })
                    .map(|(enumeration_index, task)| (task.read().unwrap().node_index, enumeration_index))
                    .collect()
            });

        Self {
            generation: schedule.generation,
            tasks: schedule
                .tasks
                .iter()
                .map(|task| {
                    task.as_ref().map(|task| {
                        let state = previous_node_index_map
                            .as_ref()
                            .and_then(|previous_node_index_map| previous_node_index_map.get(&task.node_index))
                            .map(|task_index| {
                                let previous_task = &mut previous.as_mut().unwrap().tasks[*task_index]
                                    .as_ref()
                                    .unwrap()
                                    .write()
                                    .unwrap();
                                let mut state = previous_task
                                    .state
                                    .take()
                                    .expect("Attempt to duplicate reused state during schedule preparation.");

                                task.behaviour.update_state(context, &mut state);

                                state
                            })
                            .unwrap_or_else(|| task.behaviour.create_state(context));

                        RwLock::new(PreparedTask::from(task, state))
                    })
                    // RwLock::new(PreparedTask {
                    //     node_index: task.node_index,
                    //     state: Some(state),
                    //     output_values: ChannelValues::zeroed(&task.configuration.channels_output),
                    // })
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    pub fn execute(&mut self, schedule: &Schedule, context: &mut ApplicationContext) {
        for (task_index, task) in schedule.tasks.iter().enumerate() {
            // Process enabled tasks only
            let task = if let Some(task) = task {
                task
            } else {
                continue;
            };

            let (tasks_preceding, tasks_following) = self.tasks.split_at_mut(task_index);
            let current_task: &mut PreparedTask = &mut tasks_following[0].as_ref().unwrap().write().unwrap();

            {
                // Borrows
                let borrow_value_guards = task
                    .borrows
                    .iter()
                    .map(|input| tasks_preceding[input.task_index].as_ref().unwrap().read().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let borrow_value_guards = borrow_value_guards
                    .iter()
                    .zip(&*task.borrows)
                    .map(|(task_preceding, input)| {
                        task_preceding.output_values[input.output_value_channel_index].read().unwrap()
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let input_borrows = borrow_value_guards
                    .iter()
                    .map(|borrow_value_guard| {
                        let input_typed_bytes = borrow_value_guard.as_ref(&());
                        let input_ref_option =
                            unsafe { BorrowedRef::<OptionType>::from_unchecked_type(input_typed_bytes) };
                        input_ref_option
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let input_borrow_refs = input_borrows
                    .iter()
                    .map(|input_ref_option| input_ref_option.get().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();

                // Mutable borrows
                let mut mutable_borrow_value_guards = task
                    .mutable_borrows
                    .iter()
                    .map(|input| tasks_preceding[input.task_index].as_ref().unwrap().read().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let mut mutable_borrow_value_guards = mutable_borrow_value_guards
                    .iter()
                    .zip(&*task.mutable_borrows)
                    .map(|(task_preceding, input)| {
                        task_preceding.output_values[input.output_value_channel_index].write().unwrap()
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let mut rcs = vec![(); mutable_borrow_value_guards.len()];
                let mut input_mutable_borrows = mutable_borrow_value_guards
                    .iter_mut()
                    .zip(rcs.iter_mut())
                    .map(|(mutable_borrow_value_guard, rc)| {
                        let input_typed_bytes = mutable_borrow_value_guard.as_mut(rc);
                        let input_ref_option =
                            unsafe { BorrowedRefMut::<OptionType>::from_unchecked_type(input_typed_bytes) };
                        input_ref_option
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let mut input_mutable_borrow_refs = input_mutable_borrows
                    .iter_mut()
                    .map(|input_ref_option| input_ref_option.get_mut().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();

                // Input values
                let mut input_value_guards = task
                    .inputs
                    .iter()
                    .map(|input| tasks_preceding[input.task_index].as_ref().unwrap().read().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let mut input_value_guards = input_value_guards
                    .iter()
                    .zip(&*task.inputs)
                    .map(|(task_preceding, input)| {
                        task_preceding.output_values[input.output_value_channel_index].write().unwrap()
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let mut rcs = vec![(); input_value_guards.len()];
                let mut input_values = input_value_guards
                    .iter_mut()
                    .zip(rcs.iter_mut())
                    .map(|(input_value_guard, rc)| {
                        let input_typed_bytes = input_value_guard.as_mut(rc);
                        let input_ref_option =
                            unsafe { BorrowedRefMut::<OptionType>::from_unchecked_type(input_typed_bytes) };
                        input_ref_option
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();

                // Output values
                let mut output_value_guards = current_task
                    .output_values
                    .iter_mut()
                    .map(|output_value| output_value.write().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let mut rcs = vec![(); output_value_guards.len()];
                let mut output_values = output_value_guards
                    .iter_mut()
                    .zip(rcs.iter_mut())
                    .map(|(output_value, rc)| {
                        let output_typed_bytes = output_value.as_mut(rc);
                        unsafe { BorrowedRefMut::<OptionType>::from_unchecked_type(output_typed_bytes) }
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                // let ref_guards = HashMap::new();
                let allocator_handle = unsafe { AllocatorHandle::with_node_index(task.node_index) };

                {
                    let execution_context = ExecutionContext {
                        application_context: &context,
                        allocator_handle,
                        borrows: &*input_borrow_refs,
                        mutable_borrows: &mut *input_mutable_borrow_refs,
                        inputs: &mut *input_values,
                        outputs: &mut *output_values,
                    };

                    // Execute task
                    let borrow = current_task.state.as_mut().unwrap();
                    borrow.execute(execution_context);
                    drop(borrow);
                    // (task.executor)(execution_context);
                }
            }

            // Borrows
            let borrow_value_guards = task
                .borrows
                .iter()
                .map(|input| tasks_preceding[input.task_index].as_ref().unwrap().read().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let borrow_value_guards = borrow_value_guards
                .iter()
                .zip(&*task.borrows)
                .map(|(task_preceding, input)| {
                    task_preceding.output_values[input.output_value_channel_index].read().unwrap()
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let input_borrows = borrow_value_guards
                .iter()
                .map(|borrow_value_guard| {
                    let input_typed_bytes = borrow_value_guard.as_ref(&());
                    let input_ref_option =
                        unsafe { BorrowedRef::<OptionType>::from_unchecked_type(input_typed_bytes) };
                    input_ref_option
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let input_borrow_refs = input_borrows
                .iter()
                .map(|input_ref_option| input_ref_option.get().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();

            // Mutable borrows
            let mut mutable_borrow_value_guards = task
                .mutable_borrows
                .iter()
                .map(|input| tasks_preceding[input.task_index].as_ref().unwrap().read().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let mut mutable_borrow_value_guards = mutable_borrow_value_guards
                .iter()
                .zip(&*task.mutable_borrows)
                .map(|(task_preceding, input)| {
                    task_preceding.output_values[input.output_value_channel_index].write().unwrap()
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let mut rcs = vec![(); mutable_borrow_value_guards.len()];
            let mut input_mutable_borrows = mutable_borrow_value_guards
                .iter_mut()
                .zip(rcs.iter_mut())
                .map(|(mutable_borrow_value_guard, rc)| {
                    let input_typed_bytes = mutable_borrow_value_guard.as_mut(rc);
                    let input_ref_option =
                        unsafe { BorrowedRefMut::<OptionType>::from_unchecked_type(input_typed_bytes) };
                    input_ref_option
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let mut input_mutable_borrow_refs = input_mutable_borrows
                .iter_mut()
                .map(|input_ref_option| input_ref_option.get_mut().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();

            // Input values
            let mut input_value_guards = task
                .inputs
                .iter()
                .map(|input| tasks_preceding[input.task_index].as_ref().unwrap().read().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let mut input_value_guards = input_value_guards
                .iter()
                .zip(&*task.inputs)
                .map(|(task_preceding, input)| {
                    task_preceding.output_values[input.output_value_channel_index].write().unwrap()
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let mut rcs = vec![(); input_value_guards.len()];
            let mut input_values = input_value_guards
                .iter_mut()
                .zip(rcs.iter_mut())
                .map(|(input_value_guard, rc)| {
                    let input_typed_bytes = input_value_guard.as_mut(rc);
                    let input_ref_option =
                        unsafe { BorrowedRefMut::<OptionType>::from_unchecked_type(input_typed_bytes) };
                    input_ref_option
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();

            // Output values
            let mut output_value_guards = current_task
                .output_values
                .iter_mut()
                .map(|output_value| output_value.write().unwrap())
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let mut rcs = vec![(); output_value_guards.len()];
            let mut output_values = output_value_guards
                .iter_mut()
                .zip(rcs.iter_mut())
                .map(|(output_value, rc)| {
                    let output_typed_bytes = output_value.as_mut(rc);
                    unsafe { BorrowedRefMut::<OptionType>::from_unchecked_type(output_typed_bytes) }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();

            // Apply refcount deltas
            let rc = NodeStateRefcounter(task.node_index);
            output_values.iter().for_each(|output| unsafe { output.refcount_increment_recursive_for(&rc) });
            input_borrow_refs.iter().for_each(|input| unsafe { input.refcount_decrement_recursive_for(&rc) });
            input_mutable_borrow_refs
                .iter()
                .for_each(|input| unsafe { input.refcount_decrement_recursive_for(&rc) });
            input_values.iter().for_each(|input| unsafe { input.refcount_decrement_recursive_for(&rc) });

            // Free allocations that are no longer needed.
            unsafe { Allocator::get().apply_owned_and_output_refcounts(task.node_index).unwrap() }
        }
    }
}

/// Refers to the output value storage of a task.
#[derive(Clone, Debug)]
pub struct TaskInput {
    /// The source task index.
    pub task_index: usize,
    /// The channel index of type `ChannelPassBy::Value`.
    pub output_value_channel_index: usize,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub node_index: NodeIndex,
    pub configuration: NodeConfiguration,
    pub borrows: Box<[TaskInput]>,
    pub mutable_borrows: Box<[TaskInput]>,
    pub inputs: Box<[TaskInput]>,
    pub behaviour: Box<dyn NodeBehaviourContainer>,
}

// impl Debug for Task {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Task")
//             .field("node_index", &self.node_index)
//             .field("configuration", &self.configuration)
//             .field("inputs", &self.inputs)
//             // FIXME: Add behaviour
//             .finish()
//     }
// }

#[derive(Clone, Debug)]
pub struct Schedule {
    /// Used to check whether the schedule has been updated
    pub generation: usize,
    // FIXME: implement proper multithreaded scheduling
    // `None` if the task is disabled.
    pub tasks: Box<[Option<Task>]>,
}

pub struct ExecutionGraph {
    pub graph: Graph,
    pub active_schedule: Arc<ArcSwapOption<Schedule>>,
}

impl ExecutionGraph {
    pub fn get_connections(&self) -> Vec<Connection> {
        let mut connections = Vec::with_capacity(self.graph.edge_count());

        connections.extend(self.graph.edge_indices().map(|edge_index| {
            let edge_data = &self.graph[edge_index];
            let (index_from, index_to) = self.graph.edge_endpoints(edge_index).unwrap();
            let undirected_channel_id_from = edge_data.endpoint_from.into_undirected_identifier(index_from);
            let undirected_channel_id_to = edge_data.endpoint_to.into_undirected_identifier(index_to);
            Connection([undirected_channel_id_from, undirected_channel_id_to])
        }));

        connections
    }

    pub fn check_graph_validity(&self) -> Result<(), ()> {
        for node_index in self.node_indices() {
            let node = self.node_weight(node_index);
            let node = node.as_ref().unwrap();
            let mut input_channels = node
                .configuration
                .channels(ChannelDirection::In)
                .map(|channel_ref| channel_ref.edge_endpoint)
                .collect::<HashSet<EdgeEndpoint>>();
            let mut used = false;

            for edge_ref in self.edges_directed(node_index, Direction::Incoming) {
                let edge = edge_ref.weight();

                input_channels.remove(&edge.endpoint_to);
                used = true;
            }

            if used && !input_channels.is_empty() {
                return Err(());
            }
        }

        let connections = self.get_connections();

        for edge_index in self.edge_indices() {
            let edge = &self[edge_index];
            let (node_index_from, node_index_to) = self.edge_endpoints(edge_index).unwrap();
            let connection = Connection([
                edge.endpoint_from.into_undirected_identifier(node_index_from),
                edge.endpoint_to.into_undirected_identifier(node_index_to),
            ]);

            let is_aliased = |channel: ChannelIdentifier| {
                connections.iter().filter(|connection| connection.from() == channel).count() > 1
            };
            let get_channel = |channel: ChannelIdentifier| {
                let node = &self[channel.node_index];

                node.configuration.channel(channel.channel_direction, channel.into())
            };

            if !connection.is_valid(&is_aliased, &get_channel) {
                return Err(());
            }
        }

        Ok(())
    }

    fn create_schedule(&mut self) -> Result<Schedule, ()> {
        self.check_graph_validity()?;

        let ordered_node_indices = match petgraph::algo::toposort(&self.graph, None) {
            Ok(ordered_node_indices) => ordered_node_indices,
            Err(_cycle) => {
                return Err(());
            }
        };

        let node_index_map: HashMap<NodeIndex, usize> = ordered_node_indices
            .iter()
            .enumerate()
            .map(|(enumeration_index, node_index)| (*node_index, enumeration_index))
            .collect();

        let mut tasks = Vec::<Option<Task>>::with_capacity(ordered_node_indices.len());

        for node_index in ordered_node_indices {
            let node = self.node_weight(node_index);
            let node = node.as_ref().unwrap();
            let optional_task = 'optional_task: loop {
                let mut borrows: Vec<Option<TaskInput>> =
                    vec![None; node.configuration.channels_by_shared_reference.len()];
                let mut mutable_borrows: Vec<Option<TaskInput>> =
                    vec![None; node.configuration.channels_by_mutable_reference.len()];
                let mut inputs: Vec<Option<TaskInput>> =
                    vec![None; node.configuration.input_channels_by_value.len()];
                let mut used = borrows.is_empty() && mutable_borrows.is_empty() && inputs.is_empty();

                for edge_ref in self.edges_directed(node_index, Direction::Incoming) {
                    let edge = edge_ref.weight();
                    let global_input_channel_index =
                        node.configuration.get_global_channel_index(edge.endpoint_to);
                    let immediate_source_task_index = *node_index_map.get(&edge_ref.source()).unwrap();

                    // If the input is a reference, transitively derive the value storage.
                    let task_input = if edge.endpoint_from.pass_by == ChannelPassBy::Value {
                        TaskInput {
                            task_index: immediate_source_task_index,
                            output_value_channel_index: edge.endpoint_from.channel_index,
                        }
                    } else {
                        let source_task =
                            if let Some(source_task) = tasks[immediate_source_task_index].as_mut() {
                                source_task
                            } else {
                                break 'optional_task None;
                            };

                        let source_node = self.node_weight(source_task.node_index).unwrap();
                        let global_output_channel_index =
                            source_node.configuration.get_global_channel_index(edge.endpoint_from);

                        let transitive_task_inputs = match edge.endpoint_from.pass_by {
                            ChannelPassBy::SharedReference => &mut source_task.borrows,
                            ChannelPassBy::MutableReference => &mut source_task.mutable_borrows,
                            ChannelPassBy::Value => &mut source_task.inputs,
                        };

                        transitive_task_inputs[global_output_channel_index].clone()
                    };

                    let task_inputs = match edge.endpoint_to.pass_by {
                        ChannelPassBy::SharedReference => &mut borrows,
                        ChannelPassBy::MutableReference => &mut mutable_borrows,
                        ChannelPassBy::Value => &mut inputs,
                    };

                    task_inputs[global_input_channel_index] = Some(task_input);
                    used = true;
                }

                break 'optional_task if used {
                    let borrows = borrows
                        .into_iter()
                        .map(|value| value.expect("An input channel is missing a value."))
                        .collect::<Vec<_>>()
                        .into_boxed_slice();
                    let mutable_borrows = mutable_borrows
                        .into_iter()
                        .map(|value| value.expect("An input channel is missing a value."))
                        .collect::<Vec<_>>()
                        .into_boxed_slice();
                    let inputs = inputs
                        .into_iter()
                        .map(|value| value.expect("An input channel is missing a value."))
                        .collect::<Vec<_>>()
                        .into_boxed_slice();

                    Some(Task {
                        node_index,
                        configuration: node.configuration.clone(),
                        behaviour: node.behaviour.clone(),
                        borrows,
                        mutable_borrows,
                        inputs,
                    })
                } else {
                    None
                };
            };

            tasks.push(optional_task);
        }

        Ok(Schedule {
            generation: self
                .active_schedule
                .load()
                .as_ref()
                .map(|schedule| schedule.generation.wrapping_add(1))
                .unwrap_or(0),
            tasks: tasks.into_boxed_slice(),
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

#[derive(Debug)]
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

pub struct ApplicationContext {
    pub main_thread_task_sender: Sender<Box<MainThreadTask>>,
    pub renderer: Renderer,
}

impl ApplicationContext {
    pub fn new(renderer: Renderer) -> (Self, Receiver<Box<MainThreadTask>>) {
        let (main_thread_task_sender, main_thread_task_receiver) = mpsc::channel();
        let context = Self { main_thread_task_sender, renderer };
        (context, main_thread_task_receiver)
    }

    pub fn from_settings(settings: &Settings<ApplicationFlags>) -> (Self, Receiver<Box<MainThreadTask>>) {
        Self::new(Renderer::new(settings))
    }
}

pub struct GraphExecutor {
    application_context: ApplicationContext,
    active_schedule: Arc<ArcSwapOption<Schedule>>,
}

impl GraphExecutor {
    pub fn new(
        application_context: ApplicationContext,
        active_schedule: Arc<ArcSwapOption<Schedule>>,
    ) -> Self {
        Self { active_schedule, application_context }
    }

    pub fn spawn(
        application_context: ApplicationContext,
        active_schedule: Arc<ArcSwapOption<Schedule>>,
    ) -> std::thread::JoinHandle<()> {
        thread::spawn(move || Self::new(application_context, active_schedule).run())
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
                        &mut self.application_context,
                        prepared_execution.or(last_prepared_execution.take()),
                    ));
                }

                let prepared_execution = prepared_execution.as_mut().unwrap();

                prepared_execution.execute(active_schedule, &mut self.application_context);
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
}

impl NodeData {
    pub fn new(
        title: impl ToString,
        position: impl Into<Vec2<f32>>,
        behaviour: Box<dyn NodeBehaviourContainer>,
    ) -> Self {
        let mut result = Self {
            title: title.to_string(),
            element_state: Default::default(),
            floating_pane_state: FloatingPaneState::new().with_position(position).with_width(200),
            floating_pane_behaviour_state: Default::default(),
            configuration: Default::default(),
            behaviour,
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

    pub fn view(
        &mut self,
        index: NodeIndex,
        theme: &dyn Theme,
    ) -> FloatingPane<'_, Message, iced_wgpu::Renderer, FloatingPanesBehaviour<Message>> {
        let mut builder = NodeElement::builder(index, &mut self.element_state).node_behaviour_element(
            self.behaviour.view(theme).map(Element::from).map(move |element| {
                element.map(move |message| Message::NodeMessage {
                    node: index,
                    message: NodeMessage::NodeBehaviourMessage(message),
                })
            }),
        );

        for input_channel in self.configuration.channels(ChannelDirection::In) {
            builder = builder.push_input_channel(input_channel);
        }

        for output_channel in self.configuration.channels(ChannelDirection::Out) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeEndpoint {
    pub channel_index: usize,
    pub pass_by: ChannelPassBy,
}

impl EdgeEndpoint {
    pub fn into_undirected_identifier(self, node_index: NodeIndex) -> UndirectedChannelIdentifier {
        let Self { channel_index, pass_by } = self;
        UndirectedChannelIdentifier { channel_index, pass_by, node_index }
    }
}

impl<T> From<T> for EdgeEndpoint
where T: Into<UndirectedChannelIdentifier>
{
    fn from(from: T) -> Self {
        let UndirectedChannelIdentifier { channel_index, pass_by, .. } = from.into().into();
        Self { channel_index, pass_by }
    }
}

pub struct EdgeData {
    pub endpoint_from: EdgeEndpoint,
    pub endpoint_to: EdgeEndpoint,
}

impl EdgeData {
    pub fn get_endpoint(&self, direction: ChannelDirection) -> EdgeEndpoint {
        match direction {
            ChannelDirection::In => self.endpoint_from,
            ChannelDirection::Out => self.endpoint_to,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndirectedChannelIdentifier {
    pub node_index: NodeIndex,
    pub channel_index: usize,
    pub pass_by: ChannelPassBy,
}

impl UndirectedChannelIdentifier {
    pub fn from_edge_endpoint(edge_endpoint: EdgeEndpoint, node_index: NodeIndex) -> Self {
        edge_endpoint.into_undirected_identifier(node_index)
    }

    pub fn into_directed(self, channel_direction: ChannelDirection) -> ChannelIdentifier {
        let Self { node_index, channel_index, pass_by } = self;
        ChannelIdentifier { node_index, channel_index, pass_by, channel_direction }
    }
}

impl From<ChannelIdentifier> for UndirectedChannelIdentifier {
    fn from(id: ChannelIdentifier) -> Self {
        let ChannelIdentifier { node_index, channel_index, pass_by, .. } = id;
        Self { node_index, channel_index, pass_by }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelIdentifier {
    pub node_index: NodeIndex,
    pub channel_direction: ChannelDirection,
    pub channel_index: usize,
    pub pass_by: ChannelPassBy,
}

impl ChannelIdentifier {
    pub fn from_undirected(
        undirected: UndirectedChannelIdentifier,
        channel_direction: ChannelDirection,
    ) -> Self {
        undirected.into_directed(channel_direction)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection(pub [UndirectedChannelIdentifier; 2]);

impl From<[UndirectedChannelIdentifier; 2]> for Connection {
    fn from(array: [UndirectedChannelIdentifier; 2]) -> Self {
        Self(array)
    }
}

impl Connection {
    pub fn is_valid<'a>(
        &self,
        is_aliased: &dyn Fn(ChannelIdentifier) -> bool,
        get_channel: &'a dyn Fn(ChannelIdentifier) -> ChannelRef<'a>,
    ) -> bool {
        let from = self.from();
        let to = self.to();
        from.node_index != to.node_index
            && ConnectionPassBy::derive_output_connection_pass_by(&is_aliased, from)
                .can_be_downgraded_to(ConnectionPassBy::derive_input_connection_pass_by(to))
            && {
                let channel_from = get_channel(from);
                let channel_to = get_channel(to);

                TypeEnum::is_abi_compatible(&channel_from.ty, &channel_to.ty)
            }
    }

    pub fn try_from_identifiers([a, b]: [ChannelIdentifier; 2]) -> Option<Connection> {
        if a.channel_direction == b.channel_direction {
            None
        } else {
            Some(Self(if a.channel_direction == ChannelDirection::Out {
                [a.into(), b.into()]
            } else {
                [b.into(), a.into()]
            }))
        }
    }

    pub fn contains_channel(&self, channel: ChannelIdentifier) -> bool {
        let index = match channel.channel_direction {
            ChannelDirection::In => 1,
            ChannelDirection::Out => 0,
        };
        let current = &self.0[index];

        *current == UndirectedChannelIdentifier::from(channel)
    }

    pub fn channel(&self, direction: ChannelDirection) -> ChannelIdentifier {
        let index = match direction {
            ChannelDirection::In => 1,
            ChannelDirection::Out => 0,
        };
        self.0[index].clone().into_directed(direction)
    }

    pub fn to(&self) -> ChannelIdentifier {
        self.channel(ChannelDirection::In)
    }

    pub fn from(&self) -> ChannelIdentifier {
        self.channel(ChannelDirection::Out)
    }
}
