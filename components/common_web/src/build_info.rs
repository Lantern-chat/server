#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct GenericBuildInfo {
    pub server: &'static str,
    pub target: &'static str,
    pub debug: bool,
    pub time: &'static str,
    //pub commit: Option<&'static str>,
    //pub authors: &'static str,
}

#[allow(clippy::crate_in_macro_def)] // intentional
#[macro_export]
macro_rules! decl_build_info {
    ($name:ident) => {
        #[derive(serde::Serialize)]
        #[serde(transparent)]
        pub struct $name($crate::build_info::GenericBuildInfo);

        impl ftl::body::deferred::StaticValue for $name {
            fn value() -> &'static Self {
                const {
                    &$name($crate::build_info::GenericBuildInfo {
                        server: crate::built::PKG_VERSION,
                        target: crate::built::TARGET,
                        debug: crate::built::DEBUG,
                        time: crate::built::BUILT_TIME_UTC,
                        //commit: crate::built::GIT_VERSION,
                        //authors: crate::built::PKG_AUTHORS,
                    })
                }
            }
        }
    };
}
