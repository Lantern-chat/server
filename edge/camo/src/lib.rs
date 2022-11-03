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
pub async fn main(req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    utils::set_panic_hook();

    if req.method() != Method::Get {
        return Response::error("Method Not Allowed", 405);
    }

    let path = req.path();

    if !path.starts_with("/camo/aHR0c") {
        return Response::error("Not Found", 404);
    }

    let url = match base64::decode_config(&path["/camo/".len()..], base64::URL_SAFE_NO_PAD) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(url) => url,
            Err(_) => return Response::error("Invalid UTF-8", 400),
        },
        Err(_) => return Response::error("Invalid Encoding", 400),
    };

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Response::error("Not Found", 404);
    }

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
