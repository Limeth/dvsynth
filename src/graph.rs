use crate::node::*;
use crate::*;
use iced::Element;
use std::cell::RefCell;
use vek::Vec2;

pub type NodeIndex = petgraph::graph::NodeIndex<u32>;
pub type Graph = StableGraph<
    NodeData, // Node Data
    EdgeData, // Edge Data
    Directed, // Edge Type
    u32,      // Node Index
>;

pub struct NodeData {
    pub title: String,
    pub element_state: NodeElementState,
    pub floating_pane_state: FloatingPaneState,
    pub floating_pane_behaviour_state: FloatingPaneBehaviourState,
    pub behaviour: Box<dyn NodeBehaviour>,
    pub configuration: NodeConfiguration,
    /// Output values computed during graph execution.
    pub execution_output_values: Option<RefCell<ChannelValues>>,
}

impl NodeData {
    pub fn new(
        title: impl ToString,
        position: impl Into<Vec2<f32>>,
        behaviour: Box<dyn NodeBehaviour>,
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

        result.update(NodeEvent::Update);

        result
    }

    pub fn update(&mut self, event: NodeEvent) {
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
    ) -> FloatingPane<'_, Message, Renderer, crate::widgets::node::FloatingPanesBehaviour<Message>>
    {
        let mut builder = NodeElement::builder(index, &mut self.element_state).node_behaviour_element(
            self.behaviour.view().map(Element::from).map(move |element| {
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

        FloatingPane::builder(
            node_element,
            &mut self.floating_pane_state,
            &mut self.floating_pane_behaviour_state,
            FloatingPaneBehaviourData { node_configuration: self.configuration.clone() },
        )
        .title(Some(&self.title))
        .title_size(Some(style::consts::TEXT_SIZE_TITLE))
        .title_margin(consts::SPACING)
        .style(Some(theme.floating_pane()))
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
