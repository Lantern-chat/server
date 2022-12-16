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

// configure shared base64 engine
static BASE64_ENGINE: base64::engine::fast_portable::FastPortable =
    base64::engine::fast_portable::FastPortable::from(
        &base64::alphabet::URL_SAFE,
        base64::engine::fast_portable::NO_PAD,
    );

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    if req.method() != Method::Get {
        return Response::error("Method Not Allowed", 405);
    }

    let path = req.path();

    // very early filtering for requests that start with /camo/http (base64)
    if !path.starts_with("/camo/aHR0c") {
        return Response::error("Not Found", 404);
    }

    // separate encoded url and encoded signature
    let Some((raw_url, raw_sig)) = path["/camo/".len()..].split_once('/') else {
        return Response::error("Missing signature", 400);
    };

    utils::set_panic_hook();

    // decode url
    let url = match base64::decode_engine(&raw_url, &BASE64_ENGINE) {
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
    let Ok(sig) = base64::decode_engine(&raw_sig, &BASE64_ENGINE) else {
        return Response::error("Invalid Encoding", 400);
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
