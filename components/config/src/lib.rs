#![allow(clippy::identity_op)]

pub extern crate paste;
pub extern crate serde;
pub extern crate tracing;

pub mod general;
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
                #[doc = "**Set by the `" $field_env "` environment variable.**"]
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

use futures::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct Config<C: Configuration> {
    config: arc_swap::ArcSwap<C>,
    /// Triggered when the config is reloaded
    pub config_change: Notify,
    /// when triggered, should reload the config file
    pub config_reload: Notify,
}

impl<C: Configuration> Config<C> {
    pub fn trigger_reload(&self) {
        self.config_reload.notify_waiters();
    }

    pub fn set(&self, config: Arc<C>) {
        self.config.store(config);
        self.config_change.notify_waiters();
    }

    #[inline]
    pub fn load(&self) -> arc_swap::Guard<Arc<C>> {
        self.config.load()
    }
}

pub trait HasConfig<C: Configuration> {
    fn raw(&self) -> &Config<C>;

    fn config(&self) -> arc_swap::Guard<Arc<C>> {
        self.raw().load()
    }
}

/// Returns an infinite stream that yields a reference to the config only when it changes
///
/// The first value returns immediately
pub fn config_stream<C, I>(state: &C) -> impl Stream<Item = arc_swap::Guard<Arc<I>>>
where
    C: Clone + HasConfig<I>,
    I: Configuration + 'static,
{
    use futures::stream::{iter, repeat};

    // NOTE: `iter` has less overhead than `once`
    let first = iter([state.raw().config.load()]);

    // TODO: Figure out how to avoid cloning on every item, maybe convert to stream::poll_fn
    let rest = repeat(state.clone()).then(|state| async move {
        let raw = state.raw();
        raw.config_change.notified().await;
        raw.load()
    });

    first.chain(rest)
}
