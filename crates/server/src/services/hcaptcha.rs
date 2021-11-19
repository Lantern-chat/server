/// https://docs.hcaptcha.com/
pub struct HCaptchaClient {
    client: reqwest::Client,
}

#[derive(Debug, thiserror::Error, Deserialize)]
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

    #[serde(skip)]
    #[error("Request Error: {0}")]
    Request(#[from] reqwest::Error),

    #[serde(skip)]
    #[error("Json Error: {0}")]
    Json(#[from] serde_json::Error),

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

impl Default for HCaptchaParameters<'_> {
    fn default() -> Self {
        HCaptchaParameters {
            secret: "",
            response: "",
            remoteip: None,
            sitekey: None,
        }
    }
}

use timestamp::Timestamp;

use crate::ctrl::Error;

#[derive(Debug, Deserialize)]
struct RawHCaptchaResponse {
    pub success: bool,

    pub challenge_ts: Timestamp,

    #[serde(default)]
    pub credit: bool,

    #[serde(default, rename = "error-codes")]
    pub error_codes: Vec<HCaptchaError>,

    #[serde(default)]
    pub score: Option<f32>,

    #[serde(default)]
    pub score_reason: Vec<String>,
}

impl HCaptchaClient {
    pub fn new() -> Result<HCaptchaClient, reqwest::Error> {
        Ok(HCaptchaClient {
            client: super::create_service_client()?,
        })
    }

    pub async fn verify<'a>(&self, params: HCaptchaParameters<'a>) -> Result<bool, HCaptchaError> {
        log::debug!("Sending hCaptcha verification");

        let res = self
            .client
            .post("https://hcaptcha.com/siteverify")
            .form(&params)
            .send()
            .await?;

        let full = res.bytes().await?;

        if cfg!(debug_assertions) {
            match std::str::from_utf8(&full) {
                Ok(full) => log::trace!("hCaptcha response: {}", full),
                Err(_) => log::warn!("Invalid UTF8 in hCaptcha response"),
            }
        }

        let mut response: RawHCaptchaResponse = serde_json::from_slice(&full)?;

        log::debug!("hCaptcha verified: {}", response.success);

        if response.success && !response.error_codes.is_empty() {
            return Err(response.error_codes.swap_remove(0));
        }

        Ok(response.success)
    }
}

impl From<HCaptchaError> for Error {
    fn from(err: HCaptchaError) -> Self {
        match err {
            HCaptchaError::BadRequest => Error::BadRequest,
            HCaptchaError::Request(err) => Error::RequestError(err),
            HCaptchaError::Unknown => Error::InternalErrorStatic("Unknown hCaptcha Error"),
            HCaptchaError::Json(err) => Error::JsonError(err),
            _ => Error::BadCaptcha,
        }
    }
}
