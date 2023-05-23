use futures_util::FutureExt;
use reqwest::{header::HeaderName, Client};

use axum::{
    body::{Body, StreamBody},
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    response::IntoResponse,
    routing::get,
};
use std::{net::SocketAddr, str::FromStr, sync::Arc};

use hmac::{digest::Key, Mac};
type Hmac = hmac::SimpleHmac<sha1::Sha1>;

struct CamoState {
    signing_key: Key<Hmac>,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Unable to use .env");

    let state = Arc::new(CamoState {
        signing_key: {
            let hex_key = std::env::var("CAMO_SIGNING_KEY").expect("CAMO_SIGNING_KEY not found");
            let mut raw_key = Key::<Hmac>::default();
            // keys are allowed to be shorter than the entire raw key. Will be padded internally.
            hex::decode_to_slice(&hex_key, &mut raw_key[..hex_key.len() / 2])
                .expect("Could not parse signing key!");

            raw_key
        },

        client: reqwest::ClientBuilder::new()
            .no_gzip()
            .no_deflate()
            .no_brotli()
            .redirect(reqwest::redirect::Policy::limited(1))
            .connect_timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(false)
            .http2_adaptive_window(true)
            .build()
            .expect("Unable to build primary client"),
    });

    let addr = std::env::var("CAMO_BIND_ADDRESS").expect("CAMO_BIND_ADDRESS not found");
    let addr = SocketAddr::from_str(&addr).expect("Unable to parse bind address");

    axum::Server::bind(&addr)
        .serve(get(root).with_state(state).into_make_service())
        .with_graceful_shutdown(tokio::signal::ctrl_c().map(|_| ()))
        .await
        .expect("Unable to run camo-worker");
}

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

async fn root(State(state): State<Arc<CamoState>>, req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path();

    // very early filtering for requests that start with /camo/http (base64)
    if !path.starts_with("/camo/aHR0c") {
        return Err((StatusCode::NOT_FOUND, "Not Found"));
    }

    // separate encoded url and encoded signature
    let Some((raw_url, raw_sig)) = path["/camo/".len()..].split_once('/') else {
        return Err((StatusCode::BAD_REQUEST, "Missing signature"));
    };

    // skip anything after the signature
    let Some(raw_sig) = raw_sig.split('/').next() else {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "This shouldn't happen"));
    };

    // decode url
    let url = match URL_SAFE_NO_PAD.decode(raw_url) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(url) => url,
            Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid UTF-8")),
        },
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid Encoding")),
    };

    // decode signature
    let Ok(sig) = URL_SAFE_NO_PAD.decode(raw_sig) else {
        return Err((StatusCode::BAD_REQUEST, "Invalid Encoding"));
    };

    if Hmac::new(&state.signing_key).chain_update(&url).verify_slice(&sig).is_err() {
        return Err((StatusCode::UNAUTHORIZED, "Incorrect Signature"));
    };

    Ok(proxy(&state.client, &url, req).await)
}

const BAD_REQUEST_HEADERS: [HeaderName; 3] = [
    HeaderName::from_static("host"),
    HeaderName::from_static("cookie"),
    HeaderName::from_static("referer"),
];

const BAD_RESPONSE_HEADERS: [HeaderName; 1] = [HeaderName::from_static("set-cookie")];

async fn proxy(client: &Client, url: &str, mut req: Request<Body>) -> impl IntoResponse {
    let mut headers = std::mem::take(req.headers_mut());

    for name in &BAD_REQUEST_HEADERS {
        headers.remove(name);
    }

    match client.get(url).headers(headers).send().await {
        Err(e) => {
            let code = match e.status() {
                Some(code) => code,
                _ if e.is_redirect() => StatusCode::LOOP_DETECTED,
                _ => StatusCode::NOT_FOUND,
            };

            (code, HeaderMap::new(), Err(()))
        }
        Ok(mut resp) => {
            let status = resp.status();
            let mut headers = std::mem::take(resp.headers_mut());
            let body = StreamBody::new(resp.bytes_stream());

            for name in &BAD_RESPONSE_HEADERS {
                headers.remove(name);
            }

            (status, headers, Ok(body))
        }
    }
}
