#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BuildInfo {
    pub server: &'static str,

    /// The target system the program was built for
    ///
    /// This is the TARGET field from the build info,
    /// since we don't care what platform it was actually built on,
    /// just built for.
    pub host: &'static str,
    pub debug: bool,
    pub time: &'static str,
    pub commit: Option<&'static str>,
    pub authors: &'static str,
}

pub const BUILD_INFO: BuildInfo = BuildInfo {
    server: crate::built::PKG_VERSION,
    host: crate::built::TARGET,
    debug: crate::built::DEBUG,
    time: crate::built::BUILT_TIME_UTC,
    commit: crate::built::GIT_VERSION,
    authors: crate::built::PKG_AUTHORS,
};

use warp::{Filter, Rejection, Reply};

pub fn route() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("build").map(move || warp::reply::json(&BUILD_INFO))
}
