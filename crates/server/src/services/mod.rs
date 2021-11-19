use reqwest::{Client, Error as ReqwestError};

pub fn create_service_client() -> Result<Client, ReqwestError> {
    reqwest::ClientBuilder::new()
        .user_agent("Mozzila/5.0 (compatible; Lantern Bot; +https://lantern.chat)")
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .redirect(reqwest::redirect::Policy::limited(1))
        .connect_timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(false)
        .build()
}

pub mod hcaptcha;
pub mod oembed;

pub struct Services {
    pub hcaptcha: hcaptcha::HCaptchaClient,
}

impl Services {
    pub fn start() -> Result<Services, ReqwestError> {
        Ok(Services {
            hcaptcha: hcaptcha::HCaptchaClient::new()?,
        })
    }
}
