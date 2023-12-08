use tokio::sync::Semaphore;

use crate::prelude::*;

/// https://docs.hcaptcha.com/
pub struct HCaptchaClient {
    client: reqwest::Client,
    limit: Semaphore,
}

#[derive(Debug, Clone, Copy, thiserror::Error, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HCaptchaError {
    #[error("Missing Input Secret")]
    MissingInputSecret,
    #[error("Invalid Input Secret")]
    InvalidInputSecret,
    #[error("Missing Input Response")]
    MissingInputResponse,
    #[error("Invalid Input Response")]
    InvalidInputResponse,
    #[error("Bad Request")]
    BadRequest,
    #[error("Invalid Or Already Seen Response")]
    InvalidOrAlreadySeenResponse,
    #[error("Not Using Dummy Passcode")]
    NotUsingDummyPasscode,
    #[error("Sitekey Secret Mismatch")]
    SitekeySecretMismatch,

    #[error("JSON Response Parse Error")]
    JsonParseError,

    #[serde(other)]
    #[error("Unknown hCaptcha Error")]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct HCaptchaParameters<'a> {
    pub secret: &'a str,
    pub response: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub remoteip: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sitekey: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct RawHCaptchaResponse {
    pub success: bool,

    #[serde(default, rename = "error-codes")]
    pub error_codes: Vec<HCaptchaError>,
    // TODO: Decide whether to use these or not
    /*
    pub challenge_ts: Timestamp,

    #[serde(default)]
    pub hostname: Option<SmolStr>,

    #[serde(default)]
    pub credit: bool,

    #[serde(default)]
    pub score: Option<f32>,

    #[serde(default)]
    pub score_reason: Vec<String>,
    */
}

impl HCaptchaClient {
    pub fn new() -> Result<HCaptchaClient, Error> {
        Ok(HCaptchaClient {
            client: super::create_service_client()?,
            limit: Semaphore::new(num_cpus::get() * 16),
        })
    }

    pub async fn verify<'a>(&self, params: HCaptchaParameters<'a>) -> Result<bool, Error> {
        let _guard = self.limit.acquire().await?;

        log::debug!("Sending hCaptcha verification");

        let res = self.client.post("https://hcaptcha.com/siteverify").form(&params).send().await?;

        let full = res.bytes().await?;

        if cfg!(debug_assertions) {
            match std::str::from_utf8(&full) {
                Ok(full) => log::trace!("hCaptcha response: {full}"),
                Err(_) => log::warn!("Invalid UTF8 in hCaptcha response"),
            }
        }

        let response: RawHCaptchaResponse =
            serde_json::from_slice(&full).map_err(|_| HCaptchaError::JsonParseError)?;

        drop(_guard);

        log::debug!("hCaptcha verified: {}", response.success);

        match (response.success, response.error_codes.first()) {
            (true, _) => Ok(true),
            (false, Some(&err)) => Err(err.into()),
            (false, None) => Err(HCaptchaError::Unknown.into()),
        }
    }
}
