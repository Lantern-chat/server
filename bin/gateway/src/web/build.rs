use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BuildInfo {
    pub server: &'static str,
    pub target: &'static str,
    pub debug: bool,
    pub time: &'static str,
    //pub commit: Option<&'static str>,
    //pub authors: &'static str,
}

const BUILD_INFO: BuildInfo = BuildInfo {
    server: crate::built::PKG_VERSION,
    target: crate::built::TARGET,
    debug: crate::built::DEBUG,
    time: crate::built::BUILT_TIME_UTC,
    //commit: crate::built::GIT_VERSION,
    //authors: crate::built::PKG_AUTHORS,
};

pub async fn build_info() -> impl IntoResponse {
    // ZST to avoid allocating when boxing the deferred response
    struct BuildInfoSerializer;

    impl serde::Serialize for BuildInfoSerializer {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            BUILD_INFO.serialize(serializer)
        }
    }

    Deferred::new(BuildInfoSerializer)
}
