#![allow(clippy::identity_op)]

pub extern crate paste;
pub extern crate serde;
pub extern crate tracing;

pub mod util;

pub const KIBIBYTE: i64 = 1024;
pub const MIBIBYTE: i64 = KIBIBYTE * 1024;
pub const GIBIBYTE: i64 = MIBIBYTE * 1024;

#[macro_export]
macro_rules! section {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {$(
            $(#[$field_meta:meta])*
            $field_vis:vis $field_name:ident : $field_ty:ty = $field_default:expr
                $(=> $field_env:literal
                    $(| $func:path
                        $([  $($param:expr),* ])?
                    )?
                )?
        ),*$(,)?}

        $(impl Extra { $($extra:tt)+ })?
    ) => { $crate::paste::paste! {
        #[derive(Debug, $crate::serde::Deserialize)]
        $(#[$meta])*
        #[serde(deny_unknown_fields)]
        $vis struct $name {$(
            $(#[$field_meta])*
            $(
                #[doc = ""]
                #[doc = "**Overridden by the `" $field_env "` environment variable.**"]
            )?
            $field_vis $field_name: $field_ty,
        )*}

        impl Default for $name {
            #[inline]
            fn default() -> Self {
                $name {$(
                    $field_name: $field_default,
                )*}
            }
        }

        impl $crate::ConfigExtra for $name {
            $($($extra)+)?
        }

        impl $crate::Configuration for $name {
            fn configure(&mut self) {
                $($(
                    if let Ok(value) = std::env::var($field_env) {
                        $crate::tracing::debug!("Applying environment overwrite for {}.{}=>{}", stringify!($name), stringify!($field_name), $field_env);
                        self.$field_name = ($($func(&value $( $(,$param)* )? ),)? value , ).0.into();
                    }
                )?)*

                $crate::ConfigExtra::configure(self);
            }
        }
    }};
}

#[macro_export]
macro_rules! config {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {$(
            $(#[$field_meta:meta])*
            $field:ident: $field_ty:ty
        ),*$(,)?}
    ) => {
        $(#[$meta])*
        #[derive(Default, Debug, $crate::serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        #[cfg_attr(not(feature = "strict"), serde(default))]
        pub struct $name {
            $($(#[$field_meta])* pub $field: $field_ty,)*
        }

        impl $crate::Configuration for $name {
            fn configure(&mut self) {
                $($crate::Configuration::configure(&mut self.$field);)*
            }
        }
    };
}

pub trait ConfigExtra: Configuration {
    fn configure(&mut self) {}
}

pub trait Configuration: serde::de::DeserializeOwned {
    /// Applies any environmental overrides and adjustments
    fn configure(&mut self);
}

// #[derive(Default, serde::Deserialize)]
// pub struct Config<C: Configuration> {
//     value: C,
// }
