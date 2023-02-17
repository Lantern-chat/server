extern crate client_sdk as sdk;

#[cfg(feature = "cf")]
pub mod cf;

pub static AVOID_OEMBED: phf::Set<&'static str> = phf::phf_set! {
    // gives more generic information than the meta tags, so should be avoided
    "fxtwitter.com"
};

pub static USER_AGENTS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    // TODO: Add Lantern's user-agent to vxtwitter main
    // https://github.com/dylanpdx/BetterTwitFix/blob/7a1c00ebdb6479afbfcca6d84450039d29029a75/twitfix.py#L35
    "vxtwitter.com" => "test",
    "d.vx" => "test",
};
