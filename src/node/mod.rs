#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDirection {
    In,
    Out,
}

impl ChannelDirection {
    pub fn inverse(self) -> Self {
        match self {
            ChannelDirection::In => ChannelDirection::Out,
            ChannelDirection::Out => ChannelDirection::In,
        }
    }
}

pub struct Channel {
    pub title: String,
    pub description: Option<String>,
}

impl Channel {
    pub fn new(title: impl ToString) -> Self {
        Self { title: title.to_string(), description: None }
    }

    pub fn with_description(mut self, description: impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }
}

pub struct ChannelSlice<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
}

impl<'a> From<&'a Channel> for ChannelSlice<'a> {
    fn from(other: &'a Channel) -> Self {
        Self { title: &other.title, description: other.description.as_ref().map(String::as_str) }
    }
}
