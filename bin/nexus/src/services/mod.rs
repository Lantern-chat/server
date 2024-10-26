use crate::prelude::*;

use reqwest::{Client, Error as ReqwestError};

pub fn create_service_client() -> Result<Client, ReqwestError> {
    #[allow(unused_mut)]
    let mut builder = reqwest::ClientBuilder::new()
        // TODO: Use server name and base URL from config for this?
        .user_agent("Lantern/1.0 (bot; +https://github.com/Lantern-chat)")
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .zstd(true)
        .redirect(reqwest::redirect::Policy::limited(1))
        .connect_timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(false)
        .http2_adaptive_window(true);

    builder.build()
}

pub mod embed;
pub mod hcaptcha;

pub struct Services {
    pub hcaptcha: hcaptcha::HCaptchaClient,
    pub embed: embed::EmbedClient,
}

impl Services {
    pub fn start() -> Result<Services, Error> {
        Ok(Services {
            hcaptcha: hcaptcha::HCaptchaClient::new()?,
            embed: embed::EmbedClient::new()?,
        })
    }
}
