use hmac::{digest::Key, Mac};
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    utils::set_panic_hook();

    if req.method() != Method::Get {
        return Response::error("Method Not Allowed", 405);
    }

    let path = req.path();

    // very early filtering for requests that start with /camo/http (base64)
    if !path.starts_with("/camo/aHR0c") {
        return Response::error("Not Found", 404);
    }

    // separate encoded url and encoded signature
    let (raw_url, raw_sig) = match path["/camo/".len()..].split_once('/') {
        Some((url, sig)) => (url, sig),
        None => return Response::error("Missing signature", 400),
    };

    // decode url
    let url = match base64::decode_config(&raw_url, base64::URL_SAFE_NO_PAD) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(url) => url,
            Err(_) => return Response::error("Invalid UTF-8", 400),
        },
        Err(_) => return Response::error("Invalid Encoding", 400),
    };

    // early check for non-http urls
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Response::error("Not Found", 404);
    }

    // decode signature
    let sig = match base64::decode_config(&raw_sig, base64::URL_SAFE_NO_PAD) {
        Ok(sig) => sig,
        Err(_) => return Response::error("Invalid Encoding", 400),
    };

    // parse key and build hmac
    let hmac = {
        type Hmac = hmac::SimpleHmac<sha1::Sha1>;

        let hex_key = env.secret("CAMO_SIGNING_KEY")?.to_string();
        let mut raw_key = Key::<Hmac>::default();

        // keys are allowed to be shorter than the entire raw key. Will be padded internally.
        if let Err(_) = hex::decode_to_slice(&hex_key, &mut raw_key[..hex_key.len() / 2]) {
            return Response::error("", 500);
        }

        Hmac::new(&raw_key)
    };

    if let Err(_) = hmac.chain_update(&url).verify_slice(&sig) {
        return Response::error("Incorrect Signature", 401);
    };

    log_request(&req);
    Fetch::Request(Request::new_with_init(
        &url,
        &RequestInit {
            headers: req.headers().clone(),
            ..Default::default()
        },
    )?)
    .send()
    .await
}
