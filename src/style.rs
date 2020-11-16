use crate::widgets::floating_panes;
use crate::Spacing;
use iced::{
    button, checkbox, container, pick_list, progress_bar, radio, rule, scrollable, slider, text_input, Color,
};

pub mod consts {
    use super::*;

    pub const TEXT_SIZE_REGULAR: u16 = 14;
    pub const TEXT_SIZE_TITLE: u16 = 16;
    pub const SPACING_VERTICAL: u16 = 4;
    pub const SPACING_HORIZONTAL: u16 = SPACING_VERTICAL * 2;
    pub const SPACING: Spacing = Spacing::from_axes(SPACING_HORIZONTAL, SPACING_VERTICAL);
}

// pub trait Themeable: Sized {
//     fn with_theme(self, theme: &dyn Theme) -> Self;
// }

pub trait Theme: std::fmt::Debug {
    fn container_pane(&self) -> Box<dyn container::StyleSheet>;
    fn pick_list(&self) -> Box<dyn pick_list::StyleSheet>;
    fn text_input(&self) -> Box<dyn text_input::StyleSheet>;
    fn floating_panes(&self) -> Box<dyn floating_panes::FloatingPanesStyleSheet>;
    fn floating_pane(&self) -> Box<dyn floating_panes::FloatingPaneStyleSheet>;
    fn checkbox(&self) -> Box<dyn checkbox::StyleSheet>;
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
                pub const PICK_LIST_ICON_SIZE: f32 = 0.5;
                pub const TEXT_INPUT_COLOR: Color = TEXT_COLOR;
                pub const TEXT_INPUT_COLOR_PLACEHOLDER: Color = COLORS[4];
                pub const TEXT_INPUT_COLOR_SELECTION: Color = COLORS[5];
                pub const TEXT_INPUT_COLOR_BACKGROUND: Color = COLORS[2];
                pub const FLOATING_PANE_TITLE_COLOR_BACKGROUND: Color = COLORS[4];
                pub const FLOATING_PANE_BODY_COLOR_BACKGROUND: Color = COLORS[3];
                pub const FLOATING_PANES_COLOR_BACKGROUND: Color = COLORS[1];
                pub const BORDER_WIDTH: u16 = 1;
                pub const BORDER_RADIUS: u16 = 2;
                pub const BORDER_COLOR_IDLE: Color = COLORS[1];
                pub const BORDER_COLOR_HOVERED: Color = COLORS[5];
                pub const BORDER_COLOR_FOCUSED: Color = COLORS[8];
                pub const BACKGROUND_COLOR_IDLE: Color = COLORS[2];
                pub const BACKGROUND_COLOR_HOVERED: Color = COLORS[2];
                pub const BACKGROUND_COLOR_FOCUSED: Color = COLORS[2];


                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub struct $theme_name_struct;

                impl Theme for $theme_name_struct {
                    fn checkbox(&self) -> Box<dyn checkbox::StyleSheet> {
                        pub struct Checkbox;

                        impl checkbox::StyleSheet for Checkbox {
                            fn active(&self, is_checked: bool) -> checkbox::Style {
                                checkbox::Style {
                                    background: BACKGROUND_COLOR_IDLE.into(),
                                    checkmark_color: TEXT_COLOR,
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_IDLE,
                                }
                            }

                            fn hovered(&self, is_checked: bool) -> checkbox::Style {
                                checkbox::Style {
                                    background: BACKGROUND_COLOR_HOVERED.into(),
                                    checkmark_color: TEXT_COLOR,
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_HOVERED,
                                }
                            }
                        }

                        Box::new(Checkbox)
                    }

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

                    fn pick_list(&self) -> Box<dyn pick_list::StyleSheet> {
                        pub struct PickList;

                        impl pick_list::StyleSheet for PickList {
                            fn active(&self) -> pick_list::Style {
                                pick_list::Style {
                                    text_color: TEXT_COLOR,
                                    background: BACKGROUND_COLOR_IDLE.into(),
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_IDLE,
                                    icon_size: PICK_LIST_ICON_SIZE,
                                }
                            }

                            fn hovered(&self) -> pick_list::Style {
                                pick_list::Style {
                                    text_color: TEXT_COLOR,
                                    background: BACKGROUND_COLOR_HOVERED.into(),
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_HOVERED,
                                    icon_size: PICK_LIST_ICON_SIZE,
                                }
                            }

                            fn menu(&self) -> pick_list::Menu {
                                pick_list::Menu {
                                    text_color: TEXT_COLOR,
                                    background: BACKGROUND_COLOR_FOCUSED.into(),
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_FOCUSED,
                                    selected_text_color: COLORS[COLORS.len() - 1],
                                    selected_background: COLORS[3].into(),
                                }
                            }
                        }

                        Box::new(PickList)
                    }

                    fn text_input(&self) -> Box<dyn text_input::StyleSheet> {
                        pub struct TextInput;

                        impl text_input::StyleSheet for TextInput {
                            fn placeholder_color(&self) -> Color {
                                TEXT_INPUT_COLOR_PLACEHOLDER
                            }

                            fn value_color(&self) -> Color {
                                TEXT_INPUT_COLOR
                            }

                            fn selection_color(&self) -> Color {
                                TEXT_INPUT_COLOR_SELECTION
                            }

                            fn active(&self) -> text_input::Style {
                                text_input::Style {
                                    background: BACKGROUND_COLOR_IDLE.into(),
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_IDLE,
                                    ..Default::default()
                                }
                            }

                            fn hovered(&self) -> text_input::Style {
                                text_input::Style {
                                    background: BACKGROUND_COLOR_HOVERED.into(),
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_HOVERED,
                                    ..Default::default()
                                }
                            }

                            fn focused(&self) -> text_input::Style {
                                text_input::Style {
                                    background: BACKGROUND_COLOR_FOCUSED.into(),
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_FOCUSED,
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
