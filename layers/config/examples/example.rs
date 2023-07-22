extern crate tracing as log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(log::Level::TRACE.into()))
        .finish();

    log::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    config::Config::default().save("example_config.toml").await?;

    Ok(())
}
