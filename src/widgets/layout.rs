use iced_native::{Point, Rectangle};
use std::fmt::Debug;

pub trait TypedLayout: Clone + Copy + Debug {
    fn position(&self) -> Point;
    fn bounds(&self) -> Rectangle;
}

/// A macro that facilitates type safety for layout traversal.
/// Generates a newtype (wrapper) for the [`::iced_native::Layout`] type and functions to access
/// this type from other typed layout types specified in `traverse` and `children_of`.
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
                        fn: $traverse_fn:expr,
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
            pub struct [< $type_name Layout >]<'a>(::iced_native::Layout<'a>);

            impl<'a> TypedLayout for [< $type_name Layout >]<'a> {
                fn position(&self) -> ::iced_native::Point {
                    self.0.position()
                }

                fn bounds(&self) -> ::iced_native::Rectangle {
                    self.0.bounds()
                }
            }

            // impl<'a> ::std::ops::Deref for [< $type_name Layout >]<'a> {
            //     type Target = ::iced_native::Layout<'a>;

            //     fn deref(&self) -> &Self::Target {
            //         &self.0
            //     }
            // }

            impl<'a> From<::iced_native::Layout<'a>> for [< $type_name Layout >]<'a> {
                fn from(layout: ::iced_native::Layout<'a>) -> Self {
                    Self(layout)
                }
            }

            impl<'a> From<[< $type_name Layout >]<'a>> for ::iced_native::Layout<'a> {
                fn from(layout: [< $type_name Layout >]<'a>) -> Self {
                    layout.0
                }
            }

            $(
                $(
                    impl<'a> [< $traverse_parent_type_name Layout >]<'a> {
                        pub fn [< $traverse_fn_name >](
                            self,
                            $($traverse_fn_arg_name: $traverse_fn_arg_ty, )*
                        ) -> [< $type_name Layout >]<'a> {
                            use ::iced_native::Layout;
                            // let [< $traverse_parent_type_name Layout >](parent) = self;
                            let parent = self.into();
                            let layout = ($traverse_fn)(parent, $($traverse_fn_arg_name, )*);
                            [< $type_name Layout >]::from(layout)
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
            )?
        }
    }
}
