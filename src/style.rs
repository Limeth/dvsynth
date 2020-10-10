use iced::{
    button, checkbox, container, progress_bar, radio, rule, scrollable,
    slider, text_input, Color,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    // Custom(CustomTheme), TODO
}

impl Theme {
    pub const VALUES: [Theme; 2] = [Theme::Light, Theme::Dark];
}

impl Default for Theme {
    fn default() -> Theme {
        Theme::Dark
    }
}

impl From<Theme> for Box<dyn container::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => dark::Container.into(),
            Theme::Light => light::Container.into(),
        }
    }
}

impl From<Theme> for Box<dyn text_input::StyleSheet> {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => dark::TextInput.into(),
            Theme::Light => light::TextInput.into(),
        }
    }
}

mod dark {
    use super::*;

    const TEXT_COLOR: Color = Color::WHITE;
    const TEXT_INPUT_COLOR: Color = TEXT_COLOR;
    const TEXT_INPUT_COLOR_PLACEHOLDER: Color = Color::from_rgb(
        0x7F as f32 / 0xFF as f32,
        0x7F as f32 / 0xFF as f32,
        0x7F as f32 / 0xFF as f32,
    );
    const TEXT_INPUT_COLOR_SELECTION: Color = Color::from_rgb(
        0x4F as f32 / 0xFF as f32,
        0x4F as f32 / 0xFF as f32,
        0x4F as f32 / 0xFF as f32,
    );
    const TEXT_INPUT_COLOR_BACKGROUND: Color = Color::from_rgb(
        0x2F as f32 / 0xFF as f32,
        0x2F as f32 / 0xFF as f32,
        0x2F as f32 / 0xFF as f32,
    );
    const NODE_TITLE_COLOR_BACKGROUND: Color = Color::from_rgb(
        0x3F as f32 / 0xFF as f32,
        0x3F as f32 / 0xFF as f32,
        0x3F as f32 / 0xFF as f32,
    );

    pub struct Container;

    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: NODE_TITLE_COLOR_BACKGROUND.into(),
                text_color: TEXT_COLOR.into(),
                ..container::Style::default()
            }
        }
    }

    pub struct TextInput;

    impl text_input::StyleSheet for TextInput {
        fn active(&self) -> text_input::Style {
            text_input::Style {
                background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                ..Default::default()
            }
        }

        fn focused(&self) -> text_input::Style {
            text_input::Style {
                background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                ..Default::default()
            }
        }

        fn placeholder_color(&self) -> Color {
            TEXT_INPUT_COLOR_PLACEHOLDER
        }

        fn value_color(&self) -> Color {
            TEXT_INPUT_COLOR
        }

        fn selection_color(&self) -> Color {
            TEXT_INPUT_COLOR_SELECTION
        }

        fn hovered(&self) -> text_input::Style {
            text_input::Style {
                background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                ..Default::default()
            }
        }
    }
}

mod light {
    use super::*;

    const TEXT_COLOR: Color = Color::BLACK;
    const TEXT_INPUT_COLOR: Color = TEXT_COLOR;
    const TEXT_INPUT_COLOR_PLACEHOLDER: Color = Color::from_rgb(
        0x7F as f32 / 0xFF as f32,
        0x7F as f32 / 0xFF as f32,
        0x7F as f32 / 0xFF as f32,
    );
    const TEXT_INPUT_COLOR_SELECTION: Color = Color::from_rgb(
        0x4F as f32 / 0xFF as f32,
        0x4F as f32 / 0xFF as f32,
        0x4F as f32 / 0xFF as f32,
    );
    const TEXT_INPUT_COLOR_BACKGROUND: Color = Color::from_rgb(
        0x2F as f32 / 0xFF as f32,
        0x2F as f32 / 0xFF as f32,
        0x2F as f32 / 0xFF as f32,
    );
    const NODE_TITLE_COLOR_BACKGROUND: Color = Color::from_rgb(
        0xEE as f32 / 0xFF as f32,
        0xEE as f32 / 0xFF as f32,
        0xEE as f32 / 0xFF as f32,
    );

    pub struct Container;

    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: NODE_TITLE_COLOR_BACKGROUND.into(),
                text_color: TEXT_COLOR.into(),
                ..container::Style::default()
            }
        }
    }

    pub struct TextInput;

    impl text_input::StyleSheet for TextInput {
        fn active(&self) -> text_input::Style {
            text_input::Style {
                background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                ..Default::default()
            }
        }

        fn focused(&self) -> text_input::Style {
            text_input::Style {
                background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                ..Default::default()
            }
        }

        fn placeholder_color(&self) -> Color {
            TEXT_INPUT_COLOR_PLACEHOLDER
        }

        fn value_color(&self) -> Color {
            TEXT_INPUT_COLOR
        }

        fn selection_color(&self) -> Color {
            TEXT_INPUT_COLOR_SELECTION
        }

        fn hovered(&self) -> text_input::Style {
            text_input::Style {
                background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                ..Default::default()
            }
        }
    }
}
