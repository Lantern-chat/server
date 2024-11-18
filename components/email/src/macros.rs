macro_rules! decl_scenarios {
    (
        $(
            $(#[$attr:meta])*
            $name:ident use $path:literal {
                $(
                    $(#[$field_attr:meta])*
                    $field:ident: $ty:ty
                ),* $(,)?
            }
        ),* $(,)?
    ) => {
        #[derive(Debug)]
        pub enum Scenario {
            $($name($name)),*
        }

        impl Scenario {
            /// Returns the path to the template file for the email.
            pub const fn path(&self) -> &'static str {
                match self {
                    $(Self::$name(_) => $path),*
                }
            }
        }

        $(
            $(#[$attr])*
            #[derive(Debug, Content)]
            pub struct $name {
                $(
                    $(#[$field_attr])*
                    pub $field: $ty,
                )*
            }

            impl $name {
                #[allow(clippy::new_without_default)]
                pub fn new($($field: impl Into<$ty>),*) -> Self {
                    Self { $($field: $field.into()),* }
                }
            }

            impl From<$name> for Scenario {
                fn from(email: $name) -> Self {
                    Scenario::$name(email)
                }
            }
        )*

        // Delegate Content impls to the inner structs
        impl Content for Scenario {
            #[inline(always)]
            fn is_truthy(&self) -> bool {
                match self { $(Self::$name(variant) => variant.is_truthy()),* }
            }

            #[inline(always)]
            fn capacity_hint(&self, tpl: &Template) -> usize {
                match self { $(Self::$name(variant) => variant.capacity_hint(tpl)),* }
            }

            #[inline(always)]
            fn render_unescaped<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
                match self { $(Self::$name(variant) => variant.render_unescaped(encoder)),* }
            }

            #[inline(always)]
            fn render_section<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
            where
                C: ContentSequence,
                E: Encoder,
            { match self { $(Self::$name(variant) => variant.render_section(section, encoder)),* } }

            #[inline(always)]
            fn render_inverse<C, E>(&self, section: Section<C>, encoder: &mut E) -> Result<(), E::Error>
            where
                C: ContentSequence,
                E: Encoder,
            { match self { $(Self::$name(variant) => variant.render_inverse(section, encoder)),* } }

            #[inline(always)]
            fn render_field_escaped<E: Encoder>(
                &self,
                hash: u64,
                name: &str,
                encoder: &mut E,
            ) -> Result<bool, E::Error>
            { match self { $(Self::$name(variant) => variant.render_field_escaped(hash, name, encoder)),* } }

            #[inline(always)]
            fn render_field_unescaped<E: Encoder>(
                &self,
                hash: u64,
                name: &str,
                encoder: &mut E,
            ) -> Result<bool, E::Error>
            { match self { $(Self::$name(variant) => variant.render_field_unescaped(hash, name, encoder)),* } }

            #[inline(always)]
            fn render_field_section<C, E>(
                &self,
                hash: u64,
                name: &str,
                section: Section<C>,
                encoder: &mut E,
            ) -> Result<bool, E::Error>
            where
                C: ContentSequence,
                E: Encoder,
            { match self { $(Self::$name(variant) => variant.render_field_section(hash, name, section, encoder)),* } }

            #[inline(always)]
            fn render_field_inverse<C, E>(
                &self,
                hash: u64,
                name: &str,
                section: Section<C>,
                encoder: &mut E,
            ) -> Result<bool, E::Error>
            where
                C: ContentSequence,
                E: Encoder,
            { match self { $(Self::$name(variant) => variant.render_field_inverse(hash, name, section, encoder)),* } }
        }
    };
}
