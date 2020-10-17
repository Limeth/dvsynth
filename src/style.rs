use iced::{
    button, checkbox, container, progress_bar, radio, rule, scrollable,
    slider, text_input, Color,
};
use crate::Spacing;

pub mod consts {
    use super::*;

    pub const SPACING_VERTICAL: u16 = 4;
    pub const SPACING_HORIZONTAL: u16 = SPACING_VERTICAL * 2;
    pub const SPACING: Spacing = Spacing::from_axes(SPACING_HORIZONTAL, SPACING_VERTICAL);
}

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

macro_rules! theme {
    {
        $theme_name:ident {
            $(
                const $field_name:ident: $field_ty:ty = $field_value:expr;
            )*
        }
    } => {
        mod $theme_name {
            use super::*;

            $(
                const $field_name: $field_ty = $field_value;
            )*

            const TEXT_COLOR: Color = COLORS[9];
            const TEXT_INPUT_COLOR: Color = TEXT_COLOR;
            const TEXT_INPUT_COLOR_PLACEHOLDER: Color = COLORS[3];
            const TEXT_INPUT_COLOR_SELECTION: Color = COLORS[4];
            const TEXT_INPUT_COLOR_BACKGROUND: Color = COLORS[1];
            const TEXT_INPUT_ACTIVE_COLOR_BORDER: Color = TEXT_INPUT_COLOR_BACKGROUND;
            const TEXT_INPUT_HOVERED_COLOR_BORDER: Color = COLORS[4];
            const TEXT_INPUT_FOCUSED_COLOR_BORDER: Color = COLORS[7];
            const NODE_TITLE_COLOR_BACKGROUND: Color = COLORS[2];

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
                        border_radius: 2,
                        border_width: 1,
                        border_color: TEXT_INPUT_ACTIVE_COLOR_BORDER,
                        ..Default::default()
                    }
                }

                fn focused(&self) -> text_input::Style {
                    text_input::Style {
                        background: TEXT_INPUT_COLOR_BACKGROUND.into(),
                        border_radius: 2,
                        border_width: 1,
                        border_color: TEXT_INPUT_FOCUSED_COLOR_BORDER,
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
                        border_radius: 2,
                        border_width: 1,
                        border_color: TEXT_INPUT_HOVERED_COLOR_BORDER,
                        ..Default::default()
                    }
                }
            }
        }
    }
}

theme! {
    dark {
        const COLORS: [Color; 10] = [
            rgb(0x100c06),
            rgb(0x221f1a),
            rgb(0x393530),
            rgb(0x4b4641),
            rgb(0x6e6b66),
            rgb(0x93908b),
            rgb(0xbfbcb8),
            rgb(0xdbd9d6),
            rgb(0xf0efed),
            rgb(0xfefefd),
        ];
    }
}

theme! {
    light {
        const COLORS: [Color; 10] = [
            rgb(0xfefefd),
            rgb(0xf0efed),
            rgb(0xdbd9d6),
            rgb(0xbfbcb8),
            rgb(0x93908b),
            rgb(0x6e6b66),
            rgb(0x4b4641),
            rgb(0x393530),
            rgb(0x221f1a),
            rgb(0x100c06),
        ];
    }
}

/// Convert an RGBA integer (0xRRGGBBAA) into Color
const fn rgba(rgba: u32) -> Color {
    Color::from_rgba(
        ((rgba >> 24) & 0xFF) as f32 / 0xFF as f32,
        ((rgba >> 16) & 0xFF) as f32 / 0xFF as f32,
        ((rgba >>  8) & 0xFF) as f32 / 0xFF as f32,
        ((rgba >>  0) & 0xFF) as f32 / 0xFF as f32,
    )
}

/// Convert an RGB integer (0xRRGGBB) into Color
const fn rgb(rgb: u32) -> Color {
    Color::from_rgb(
        ((rgb >> 16) & 0xFF) as f32 / 0xFF as f32,
        ((rgb >>  8) & 0xFF) as f32 / 0xFF as f32,
        ((rgb >>  0) & 0xFF) as f32 / 0xFF as f32,
    )
}
