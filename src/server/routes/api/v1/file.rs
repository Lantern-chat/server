use std::sync::Arc;

use bytes::{Buf, BufMut};
use futures::StreamExt;
use warp::{hyper::Server, reject::Reject, Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimitKey, ServerState},
};

use aes_gcm_siv::aead::{generic_array::GenericArray, Aead, NewAead};
use aes_gcm_siv::Aes256GcmSiv;

const KEY: &[u8] = b"an example very very secret key.";
const NONCE: &[u8] = b"unique nonce";

#[derive(Serialize, Deserialize)]
pub struct GetUpload {
    id: Snowflake,
}

#[derive(Deserialize)]
pub struct PostChunk {
    crc32: u32,
    data: Vec<u8>,
}

fn routes() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("upload").and(balanced_or_tree!(get()))
}

fn get() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::get().map(|| {
        warp::reply::json(&GetUpload {
            id: Snowflake::null(),
        })
    })
}

// 8MiB
const MAX_BODY_SIZE: u64 = 1024 * 1000 * 8;

fn post() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post()
        // POST api/v1/upload/:id:/:chunk:
        .and(warp::path!(Snowflake / u32))
        .and(warp::body::content_length_limit(MAX_BODY_SIZE))
        .and(warp::multipart::form())
        .map(|_, _, _| warp::reply())
    /*/.and_then(
        |file_id, chunk_id, form: warp::multipart::FormData| async move {
            let mut crc32: Option<u32> = None;
            let mut file: Option<()> = None;

            while let Some(part) = form.next().await {
                let mut part = part?;
                match part.name() {
                    "crc32" => {
                        let mut buffer = Vec::with_capacity(4);
                        while let Some(buf) = part.data().await {
                            let mut buf = buf?;
                            buffer.put(&mut buf);
                            if buffer.len() > 4 {
                                panic!("");
                            }
                        }
                        crc32 = Some(buffer.as_slice().get_u32());
                    }
                }
            }

            Ok("")
        },
    ) // TODO */
}

/*
let key = GenericArray::from_slice(KEY);
let cipher = Aes256GcmSiv::new(key);

let nonce = GenericArray::from_slice(NONCE); // 96-bits; unique per message

let ciphertext = cipher
    .encrypt(nonce, b"plaintext message".as_ref())
    .expect("encryption failure!"); // NOTE: handle this error to avoid panics!
*/
