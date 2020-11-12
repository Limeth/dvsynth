use crate::widgets::floating_panes;
use crate::Spacing;
use iced::{button, checkbox, container, progress_bar, radio, rule, scrollable, slider, text_input, Color};

pub mod consts {
    use super::*;

    pub const TEXT_SIZE_REGULAR: u16 = 14;
    pub const TEXT_SIZE_TITLE: u16 = 16;
    pub const SPACING_VERTICAL: u16 = 4;
    pub const SPACING_HORIZONTAL: u16 = SPACING_VERTICAL * 2;
    pub const SPACING: Spacing = Spacing::from_axes(SPACING_HORIZONTAL, SPACING_VERTICAL);
}

pub trait Theme: std::fmt::Debug {
    fn container_pane(&self) -> Box<dyn container::StyleSheet>;
    fn text_input(&self) -> Box<dyn text_input::StyleSheet>;
    fn floating_panes(&self) -> Box<dyn floating_panes::FloatingPanesStyleSheet>;
    fn floating_pane(&self) -> Box<dyn floating_panes::FloatingPaneStyleSheet>;
}

macro_rules! themes {
    {
        $(
            $theme_name_struct:ident, $theme_name_mod:ident {
                $(
                    const $field_name:ident: $field_ty:ty = $field_value:expr;
                )*
            }
        )*
    } => {
        $(
            pub use $theme_name_mod::$theme_name_struct;

            mod $theme_name_mod {
                use super::*;

                $(
                    const $field_name: $field_ty = $field_value;
                )*

                pub const TEXT_COLOR: Color = COLORS[10];
                pub const TEXT_INPUT_COLOR: Color = TEXT_COLOR;
                pub const TEXT_INPUT_COLOR_PLACEHOLDER: Color = COLORS[4];
                pub const TEXT_INPUT_COLOR_SELECTION: Color = COLORS[5];
                pub const TEXT_INPUT_COLOR_BACKGROUND: Color = COLORS[2];
                pub const TEXT_INPUT_ACTIVE_COLOR_BORDER: Color = TEXT_INPUT_COLOR_BACKGROUND;
                pub const TEXT_INPUT_HOVERED_COLOR_BORDER: Color = COLORS[5];
                pub const TEXT_INPUT_FOCUSED_COLOR_BORDER: Color = COLORS[8];
                pub const FLOATING_PANE_TITLE_COLOR_BACKGROUND: Color = COLORS[4];
                pub const FLOATING_PANE_BODY_COLOR_BACKGROUND: Color = COLORS[3];
                pub const FLOATING_PANES_COLOR_BACKGROUND: Color = COLORS[1];


                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub struct $theme_name_struct;

                impl Theme for $theme_name_struct {
                    fn container_pane(&self) -> Box<dyn container::StyleSheet> {
                        pub struct Container;

                        impl container::StyleSheet for Container {
                            fn style(&self) -> container::Style {
                                Default::default()
                                // container::Style {
                                //     background: NODE_TITLE_COLOR_BACKGROUND.into(),
                                //     text_color: TEXT_COLOR.into(),
                                //     ..container::Style::default()
                                // }
                            }
                        }

                        Box::new(Container)
                    }

                    fn text_input(&self) -> Box<dyn text_input::StyleSheet> {
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

                        Box::new(TextInput)
                    }

                    fn floating_pane(&self) -> Box<dyn floating_panes::FloatingPaneStyleSheet> {
                        pub struct FloatingPane;

                        impl floating_panes::FloatingPaneStyleSheet for FloatingPane {
                            fn style(&self) -> floating_panes::FloatingPaneStyle {
                                floating_panes::FloatingPaneStyle {
                                    title_background_color: FLOATING_PANE_TITLE_COLOR_BACKGROUND,
                                    title_text_color: TEXT_COLOR,
                                    body_background_color: FLOATING_PANE_BODY_COLOR_BACKGROUND,
                                }
                            }
                        }

                        Box::new(FloatingPane)
                    }

                    fn floating_panes(&self) -> Box<dyn floating_panes::FloatingPanesStyleSheet> {
                        pub struct FloatingPanes;

                        impl floating_panes::FloatingPanesStyleSheet for FloatingPanes {
                            fn style(&self) -> floating_panes::FloatingPanesStyle {
                                floating_panes::FloatingPanesStyle {
                                    background_color: FLOATING_PANES_COLOR_BACKGROUND,
                                }
                            }
                        }

                        Box::new(FloatingPanes)
                    }
                }
            }
        )*
    }
}

themes! {
    Dark, dark {
        const COLORS: [Color; 11] = [
            rgb(0x100c06),
            rgb(0x191510),
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

    Light, light {
        const COLORS: [Color; 11] = [
            rgb(0xfefefd),
            rgb(0xf0efed),
            rgb(0xdbd9d6),
            rgb(0xbfbcb8),
            rgb(0x93908b),
            rgb(0x6e6b66),
            rgb(0x4b4641),
            rgb(0x393530),
            rgb(0x221f1a),
            rgb(0x191510),
            rgb(0x100c06),
        ];
    }
}

/// Convert an RGBA integer (0xRRGGBBAA) into Color
const fn rgba(rgba: u32) -> Color {
    Color::from_rgba(
        ((rgba >> 24) & 0xFF) as f32 / 0xFF as f32,
        ((rgba >> 16) & 0xFF) as f32 / 0xFF as f32,
        ((rgba >> 8) & 0xFF) as f32 / 0xFF as f32,
        ((rgba >> 0) & 0xFF) as f32 / 0xFF as f32,
    )
}

/// Convert an RGB integer (0xRRGGBB) into Color
const fn rgb(rgb: u32) -> Color {
    Color::from_rgb(
        ((rgb >> 16) & 0xFF) as f32 / 0xFF as f32,
        ((rgb >> 8) & 0xFF) as f32 / 0xFF as f32,
        ((rgb >> 0) & 0xFF) as f32 / 0xFF as f32,
    )
}
