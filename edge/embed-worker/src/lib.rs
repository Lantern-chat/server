extern crate client_sdk as sdk;

#[cfg(feature = "cf")]
pub mod cf;

pub static AVOID_OEMBED: phf::Set<&'static str> = phf::phf_set!("fxtwitter.com");

// TODO: Add Lantern's user-agent to vxtwitter main
pub static USER_AGENTS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "vxtwitter.com" => "test",
};
