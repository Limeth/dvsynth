use crate::util::rgb;
use crate::widgets::{floating_panes, node};
use crate::Spacing;
use iced::{checkbox, container, pick_list, text_input, widget, Color};

pub mod consts {
    use super::*;

    pub const TEXT_SIZE_REGULAR: u16 = 14;
    pub const TEXT_SIZE_TITLE: u16 = 16;
    pub const SPACING_VERTICAL: u16 = 4;
    pub const SPACING_HORIZONTAL: u16 = SPACING_VERTICAL * 2;
    pub const SPACING: Spacing = Spacing::from_axes(SPACING_HORIZONTAL, SPACING_VERTICAL);
}

pub trait Themeable: Sized {
    fn theme(self, theme: &dyn Theme) -> Self;
}

pub trait StyleSheetProvider: std::fmt::Debug {
    fn container(&self) -> Box<dyn container::StyleSheet>;
    fn pick_list(&self) -> Box<dyn pick_list::StyleSheet>;
    fn text_input(&self) -> Box<dyn text_input::StyleSheet>;
    fn checkbox(&self) -> Box<dyn checkbox::StyleSheet>;
    fn floating_panes(&self) -> Box<dyn floating_panes::FloatingPanesStyleSheet>;
    fn floating_pane(&self) -> Box<dyn floating_panes::FloatingPaneStyleSheet>;
    fn tooltip(&self) -> Box<dyn node::TooltipStyleSheet>;
}

pub trait Theme: StyleSheetProvider {}
impl<T> Theme for T where T: StyleSheetProvider {}

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
                pub const FLOATING_PANE_TITLE_COLOR_BACKGROUND_IDLE: Color = COLORS[4];
                pub const FLOATING_PANE_TITLE_COLOR_BACKGROUND_HOVERED: Color = COLORS[5];
                pub const FLOATING_PANE_TITLE_COLOR_BACKGROUND_FOCUSED: Color = COLORS[6];
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

                impl StyleSheetProvider for $theme_name_struct {
                    fn checkbox(&self) -> Box<dyn checkbox::StyleSheet> {
                        pub struct Checkbox;

                        impl checkbox::StyleSheet for Checkbox {
                            fn active(&self, _is_checked: bool) -> checkbox::Style {
                                checkbox::Style {
                                    background: BACKGROUND_COLOR_IDLE.into(),
                                    checkmark_color: TEXT_COLOR,
                                    border_radius: BORDER_RADIUS,
                                    border_width: BORDER_WIDTH,
                                    border_color: BORDER_COLOR_IDLE,
                                }
                            }

                            fn hovered(&self, _is_checked: bool) -> checkbox::Style {
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

                    fn container(&self) -> Box<dyn container::StyleSheet> {
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
                            fn style(&self, title_bar_status: InteractionStatus) -> floating_panes::FloatingPaneStyle {
                                floating_panes::FloatingPaneStyle {
                                    title_background_color: match title_bar_status {
                                        InteractionStatus::Idle => FLOATING_PANE_TITLE_COLOR_BACKGROUND_IDLE,
                                        InteractionStatus::Hovered => FLOATING_PANE_TITLE_COLOR_BACKGROUND_HOVERED,
                                        InteractionStatus::Focused => FLOATING_PANE_TITLE_COLOR_BACKGROUND_FOCUSED,
                                    },
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

                    fn tooltip(&self) -> Box<dyn node::TooltipStyleSheet> {
                        pub struct Tooltip;

                        impl node::TooltipStyleSheet for Tooltip {
                            fn style(&self) -> node::TooltipStyle {
                                node::TooltipStyle {
                                    container: {
                                        pub struct Container;

                                        impl container::StyleSheet for Container {
                                            fn style(&self) -> container::Style {
                                                container::Style {
                                                    background: {
                                                        let mut color = COLORS[1];
                                                        color.a = 0.9;
                                                        color.into()
                                                    },
                                                    text_color: rgb(0xFF0000).into(),
                                                    ..container::Style::default()
                                                }
                                            }
                                        }

                                        Box::new(Container)
                                    }
                                }
                            }
                        }

                        Box::new(Tooltip)
                    }
                }
            }
        )*
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum InteractionStatus {
    Idle,
    Hovered,
    Focused,
}

impl Default for InteractionStatus {
    fn default() -> Self {
        InteractionStatus::Idle
    }
}

impl<'a, M> Themeable for container::Container<'a, M> {
    fn theme(self, theme: &dyn Theme) -> Self {
        self.style(theme.container())
    }
}

impl<'a, M> Themeable for widget::Row<'a, M> {
    fn theme(self, _theme: &dyn Theme) -> Self {
        self.spacing(consts::SPACING_HORIZONTAL)
    }
}

impl<'a, M> Themeable for widget::Column<'a, M> {
    fn theme(self, _theme: &dyn Theme) -> Self {
        self.spacing(consts::SPACING_VERTICAL)
    }
}

impl<'a, T, M> Themeable for pick_list::PickList<'a, T, M>
where
    T: ToString + Eq,
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn theme(self, theme: &dyn Theme) -> Self {
        self.style(theme.pick_list()).text_size(consts::TEXT_SIZE_REGULAR).padding(consts::SPACING_VERTICAL)
    }
}

impl<'a, M: Clone> Themeable for text_input::TextInput<'a, M> {
    fn theme(self, theme: &dyn Theme) -> Self {
        self.style(theme.text_input()).size(consts::TEXT_SIZE_REGULAR).padding(consts::SPACING_VERTICAL)
    }
}

impl<M> Themeable for checkbox::Checkbox<M> {
    fn theme(self, theme: &dyn Theme) -> Self {
        self.style(theme.checkbox())
            .size(consts::TEXT_SIZE_REGULAR)
            .text_size(consts::TEXT_SIZE_REGULAR)
            .spacing(consts::SPACING_HORIZONTAL)
    }
}

impl<'a, M, R, C> Themeable for floating_panes::FloatingPanes<'a, M, R, C>
where
    M: 'a,
    R: 'a + floating_panes::WidgetRenderer,
    C: 'a + floating_panes::FloatingPanesBehaviour<'a, M, R>,
    <R as floating_panes::WidgetRenderer>::StyleFloatingPanes:
        From<Box<dyn floating_panes::FloatingPanesStyleSheet>>,
{
    fn theme(self, theme: &dyn Theme) -> Self {
        self.style(theme.floating_panes())
    }
}

impl<'a, M, R, C> Themeable for floating_panes::FloatingPaneBuilder<'a, M, R, C>
where
    M: 'a,
    R: 'a + floating_panes::WidgetRenderer,
    C: 'a + floating_panes::FloatingPanesBehaviour<'a, M, R>,
    <R as floating_panes::WidgetRenderer>::StyleFloatingPane:
        From<Box<dyn floating_panes::FloatingPaneStyleSheet>>,
{
    fn theme(self, theme: &dyn Theme) -> Self {
        self.style(Some(theme.floating_pane()))
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
