#![allow(
    clippy::redundant_pattern_matching,
    clippy::field_reassign_with_default,
    clippy::large_enum_variant
)]

extern crate client_sdk as sdk;

pub mod config;
pub mod error;
pub mod extractors;
pub mod state;
pub mod util;

use config::Site;
use error::Error;
use state::WorkerState;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::post,
    Json,
};
use futures_util::FutureExt;
use std::{net::SocketAddr, str::FromStr, sync::Arc};

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Unable to use .env");

    let config = {
        let config_path = std::env::var("EMBEDW_CONFIG_PATH").unwrap_or_else(|_| "./config.toml".to_owned());
        let config_file = std::fs::read_to_string(config_path).expect("Unable to read config file");
        let parsed: config::ParsedConfig = toml::de::from_str(&config_file).expect("Unable to parse config file");

        parsed.build().expect("Unable to build config")
    };

    let state = Arc::new(WorkerState::new(
        config,
        std::env::var("CAMO_SIGNING_KEY").expect("CAMO_SIGNING_KEY not found"),
    ));

    for extractor in &state.extractors {
        extractor.setup(state.clone()).await.expect("Failed to setup extractor");
    }

    let addr = std::env::var("EMBEDW_BIND_ADDRESS").expect("EMBEDW_BIND_ADDRESS not found");
    let addr = SocketAddr::from_str(&addr).expect("Unable to parse bind address");

    println!("Starting...");

    axum::Server::bind(&addr)
        .serve(post(root).with_state(state).into_make_service())
        .with_graceful_shutdown(tokio::signal::ctrl_c().map(|_| ()))
        .await
        .expect("Unable to run embed-worker");

    println!("Goodbye.");
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Params {
    #[serde(rename = "l")]
    pub lang: Option<String>,
}

async fn root(
    State(state): State<Arc<WorkerState>>,
    Query(params): Query<Params>,
    body: String,
) -> Result<Json<extractors::EmbedWithExpire>, (StatusCode, String)> {
    let url = body; // to avoid confusion

    match inner(state, url, params).await {
        Ok(value) => Ok(Json(value)),
        Err(e) => Err({
            let code = match e {
                Error::InvalidUrl | Error::UrlError(_) => StatusCode::BAD_REQUEST,
                Error::InvalidMimeType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Error::Failure(code) => code,
                Error::ReqwestError(ref e) => match e.status() {
                    Some(status) => status,
                    None if e.is_connect() => StatusCode::REQUEST_TIMEOUT,
                    None => StatusCode::INTERNAL_SERVER_ERROR,
                },
                Error::JsonError(_) | Error::XMLError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };

            let msg = if code.is_server_error() { "Internal Server Error".to_owned() } else { e.to_string() };

            (code, msg)
        }),
    }
}

async fn inner(
    state: Arc<WorkerState>,
    url: String,
    params: Params,
) -> Result<extractors::EmbedWithExpire, Error> {
    let url = url::Url::parse(&url)?;

    for extractor in &state.extractors {
        if extractor.matches(&url) {
            return extractor.extract(state.clone(), url, params).await;
        }
    }

    Err(Error::Failure(StatusCode::NOT_FOUND))
}
