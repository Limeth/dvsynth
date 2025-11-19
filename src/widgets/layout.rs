use iced_core::{Point, Rectangle, widget::Tree};
use std::fmt::Debug;

pub trait TypedLayout: Clone + Copy + Debug {
    fn position(&self) -> Point;
    fn bounds(&self) -> Rectangle;
}

pub trait TypedTreeRef<'a>: Debug {
    fn tree_ref(&self) -> &Tree;
}

pub trait TypedTreeMut<'a>: TypedTreeRef<'a> {
    fn tree_mut(&mut self) -> &mut Tree;
}

/// A macro that facilitates type safety for layout traversal.
/// Generates a newtype (wrapper) for the [`::iced_runtime::Layout`] type and functions to access
/// this type from other typed layout types specified in `traverse` and `children_of`.
/// Use `Into` and `From` to convert into/from `iced_core::Layout`.
macro_rules! typed_layout {
    {
        type_name: $type_name:ident,
        $(
            traverse: [
                $(
                    {
                        parent_type_name: $traverse_parent_type_name:ident,
                        fn_name: $traverse_fn_name:ident,
                        fn_args: [$($traverse_fn_arg_name:ident: $traverse_fn_arg_ty:ty),*$(,)?],
                        layout_fn: $traverse_layout_fn:expr,
                        tree_ref_fn: $traverse_tree_ref_fn:expr,
                        tree_mut_fn: $traverse_tree_mut_fn:expr,
                    },
                )*
            ],
        )?
        $(
            children_of: {
                parent_type_name: $children_of_parent_type_name:ident,
                fn_name: $children_of_fn_name:ident,
            },
        )?
    } => {
        paste::item! {
            #[derive(Clone, Copy, Debug)]
            pub struct [< $type_name Layout >]<'a>(::iced_core::Layout<'a>);

            #[derive(Debug)]
            pub struct [< $type_name TreeRef >]<'a>(&'a ::iced_core::widget::Tree);

            #[derive(Debug)]
            pub struct [< $type_name TreeMut >]<'a>(&'a mut ::iced_core::widget::Tree);

            impl<'a> TypedLayout for [< $type_name Layout >]<'a> {
                fn position(&self) -> ::iced_core::Point {
                    self.0.position()
                }

                fn bounds(&self) -> ::iced_core::Rectangle {
                    self.0.bounds()
                }
            }

            impl<'a> TypedTreeRef<'a> for [< $type_name TreeRef >]<'a> {
                fn tree_ref(&self) -> &Tree {
                    &self.0
                }
            }

            impl<'a> TypedTreeRef<'a> for [< $type_name TreeMut >]<'a> {
                fn tree_ref(&self) -> &Tree {
                    &self.0
                }
            }

            impl<'a> TypedTreeMut<'a> for [< $type_name TreeMut >]<'a> {
                fn tree_mut(&mut self) -> &mut Tree {
                    &mut self.0
                }
            }

            // impl<'a> ::std::ops::Deref for [< $type_name Layout >]<'a> {
            //     type Target = ::iced_runtime::Layout<'a>;

            //     fn deref(&self) -> &Self::Target {
            //         &self.0
            //     }
            // }

            impl<'a> From<::iced_core::Layout<'a>> for [< $type_name Layout >]<'a> {
                fn from(layout: ::iced_core::Layout<'a>) -> Self {
                    Self(layout)
                }
            }

            impl<'a> From<[< $type_name Layout >]<'a>> for ::iced_core::Layout<'a> {
                fn from(layout: [< $type_name Layout >]<'a>) -> Self {
                    layout.0
                }
            }

            impl<'a> From<&'a ::iced_core::widget::Tree> for [< $type_name TreeRef >]<'a> {
                fn from(tree: &'a ::iced_core::widget::Tree) -> Self {
                    Self(tree)
                }
            }

            impl<'a> From<[< $type_name TreeRef >]<'a>> for &'a ::iced_core::widget::Tree {
                fn from(tree: [< $type_name TreeRef >]<'a>) -> Self {
                    tree.0
                }
            }

            impl<'a> From<&'a mut ::iced_core::widget::Tree> for [< $type_name TreeMut >]<'a> {
                fn from(tree: &'a mut ::iced_core::widget::Tree) -> Self {
                    Self(tree)
                }
            }

            impl<'a> From<[< $type_name TreeMut >]<'a>> for &'a mut ::iced_core::widget::Tree {
                fn from(tree: [< $type_name TreeMut >]<'a>) -> Self {
                    tree.0
                }
            }

            $(
                $(
                    impl<'a> [< $traverse_parent_type_name Layout >]<'a> {
                        pub fn [< $traverse_fn_name >](
                            self,
                            $($traverse_fn_arg_name: $traverse_fn_arg_ty, )*
                        ) -> [< $type_name Layout >]<'a> {
                            use ::iced_core::Layout;
                            // let [< $traverse_parent_type_name Layout >](parent) = self;
                            let parent = self.into();
                            let layout = ($traverse_layout_fn)(parent, $($traverse_fn_arg_name, )*);
                            [< $type_name Layout >]::from(layout)
                        }
                    }

                    impl<'a> [< $traverse_parent_type_name TreeRef >]<'a> {
                        pub fn [< $traverse_fn_name >](
                            self,
                            $($traverse_fn_arg_name: $traverse_fn_arg_ty, )*
                        ) -> [< $type_name TreeRef >]<'a> {
                            use ::iced_core::widget::Tree;
                            // let [< $traverse_parent_type_name TreeRef >](parent) = self;
                            let parent = self.into();
                            let tree = ($traverse_tree_ref_fn)(parent, $($traverse_fn_arg_name, )*);
                            [< $type_name TreeRef >]::from(tree)
                        }
                    }

                    impl<'a> [< $traverse_parent_type_name TreeMut >]<'a> {
                        pub fn [< $traverse_fn_name >](
                            self,
                            $($traverse_fn_arg_name: $traverse_fn_arg_ty, )*
                        ) -> [< $type_name TreeMut >]<'a> {
                            use ::iced_core::widget::Tree;
                            // let [< $traverse_parent_type_name TreeMut >](parent) = self;
                            let parent = self.into();
                            let tree = ($traverse_tree_mut_fn)(parent, $($traverse_fn_arg_name, )*);
                            [< $type_name TreeMut >]::from(tree)
                        }
                    }
                )*
            )?

            $(
                impl<'a> [< $children_of_parent_type_name Layout >]<'a> {
                    pub fn [< $children_of_fn_name >](
                        self,
                    ) -> impl Iterator<Item=[< $type_name Layout >]<'a>> {
                        let [< $children_of_parent_type_name Layout >](parent) = self;
                        parent.children().map(|layout| {
                            [< $type_name Layout >]::from(layout)
                        })
                    }
                }

                impl<'a> [< $children_of_parent_type_name TreeRef >]<'a> {
                    pub fn [< $children_of_fn_name >](
                        self,
                    ) -> impl Iterator<Item=[< $type_name TreeRef >]<'a>> {
                        let [< $children_of_parent_type_name TreeRef >](parent) = self;
                        parent.children.iter().map(|layout| {
                            [< $type_name TreeRef >]::from(layout)
                        })
                    }
                }

                impl<'a> [< $children_of_parent_type_name TreeMut >]<'a> {
                    pub fn [< $children_of_fn_name >](
                        self,
                    ) -> impl Iterator<Item=[< $type_name TreeMut >]<'a>> {
                        let [< $children_of_parent_type_name TreeMut >](parent) = self;
                        parent.children.iter_mut().map(|layout| {
                            [< $type_name TreeMut >]::from(layout)
                        })
                    }
                }
            )?
        }
    }
}
