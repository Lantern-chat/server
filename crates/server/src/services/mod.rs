use reqwest::{Client, Error as ReqwestError};

pub fn create_service_client() -> Result<Client, ReqwestError> {
    reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (compatible; Lantern Bot; +https://lantern.chat)")
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .redirect(reqwest::redirect::Policy::limited(1))
        .connect_timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(false)
        .http2_adaptive_window(true)
        .build()
}

pub mod hcaptcha;
pub mod oembed;

pub struct Services {
    pub hcaptcha: hcaptcha::HCaptchaClient,
    pub embed: oembed::OEmbedClient,
}

impl Services {
    pub fn start() -> Result<Services, crate::ctrl::Error> {
        Ok(Services {
            hcaptcha: hcaptcha::HCaptchaClient::new()?,
            embed: oembed::OEmbedClient::new()?,
        })
    }
}
