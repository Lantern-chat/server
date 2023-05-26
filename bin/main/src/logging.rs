use std::path::PathBuf;

use log::Dispatch;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::{Layer, Subscriber},
    layer::SubscriberExt,
};

fn create_filter(verbose: Option<u8>, level: Option<LevelFilter>) -> anyhow::Result<EnvFilter> {
    let mut extreme_trace = false;

    #[allow(clippy::wildcard_in_or_patterns)]
    let level_filter = level.unwrap_or_else(|| match verbose {
        None | Some(0) => LevelFilter::INFO,
        Some(1) => LevelFilter::DEBUG,
        Some(2) => LevelFilter::TRACE,
        Some(3) | _ => {
            extreme_trace = true;
            LevelFilter::TRACE
        }
    });

    let mut filter = EnvFilter::from_default_env()
        .add_directive(level_filter.into())
        .add_directive("hyper::client::pool=info".parse()?)
        .add_directive("hyper::proto=info".parse()?)
        .add_directive("h2::proto=info".parse()?)
        .add_directive("tokio_util::codec=info".parse()?);

    if !extreme_trace {
        filter = filter.add_directive("server::tasks=debug".parse()?);
    }

    Ok(filter)
}

pub fn generate(
    verbose: Option<u8>,
    dir: Option<PathBuf>,
) -> Result<(Dispatch, Option<WorkerGuard>), anyhow::Error> {
    let filter = create_filter(verbose, None)?;

    Ok(match dir {
        None => (
            Dispatch::new(Subscriber::builder().with_env_filter(filter).with_writer(std::io::stdout).finish()),
            None,
        ),
        Some(dir) => {
            let file_appender = tracing_appender::rolling::daily(dir, "log");
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

            let file_logger = Layer::new().with_writer(non_blocking).with_ansi(false);
            let stdout_logger = Layer::new().with_writer(std::io::stdout);

            let collector = tracing_subscriber::registry().with(filter).with(file_logger).with(stdout_logger);

            (Dispatch::new(collector), Some(_guard))
        }
    })
}
