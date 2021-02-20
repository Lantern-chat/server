use std::net::{AddrParseError, IpAddr};
use std::str::FromStr;

use http::header::ToStrError;
use tokio_postgres::Socket;

use super::service::Route;

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

    Ok(if let Some(x_real_ip) = headers.get("x-real-ip") {
        IpAddr::from_str(x_real_ip.to_str()?.trim())?
    } else if let Some(x_forwarded_for) = headers.get("x-forwarded-for") {
        IpAddr::from_str(
            x_forwarded_for
                .to_str()?
                .split(',')
                .next()
                .expect("at least one client or proxy")
                .trim(),
        )?
    } else {
        route.addr.ip()
    })
}
