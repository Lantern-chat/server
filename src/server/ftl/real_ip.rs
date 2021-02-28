use std::net::{AddrParseError, IpAddr};
use std::str::FromStr;

use http::header::ToStrError;
use tokio_postgres::Socket;

use super::*;

#[derive(Debug, thiserror::Error)]
pub enum GetRealIpError {
    #[error("Missing IP headers or information")]
    MissingAddress,

    #[error(transparent)]
    ToStrError(#[from] ToStrError),

    #[error(transparent)]
    AddrParseError(#[from] AddrParseError),
}

pub fn get_real_ip(route: &Route) -> Result<IpAddr, GetRealIpError> {
    let headers = route.req.headers();

    if let Some(Ok(mut proxies)) = route.forwarded_for() {
        if let Some(first) = proxies.next() {
            return Ok(first?);
        }
    }

    if let Some(x_real_ip) = headers.get("x-real-ip") {
        return Ok(IpAddr::from_str(x_real_ip.to_str()?.trim())?);
    }

    Ok(route.addr.ip())
}
