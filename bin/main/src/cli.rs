use std::path::PathBuf;

/// Lantern server
#[derive(Debug, argh::FromArgs)]
pub struct CliOptions {
    /// print version information and exit
    #[argh(switch, short = 'V')]
    pub version: bool,

    /// logging level (0 = Info, 1 = Debug, 2 = Trace) [env LANTERN_VERBOSE]
    #[argh(option, short = 'v')]
    pub verbose: Option<u8>,

    /// specify Lantern configuration file location
    #[argh(option, default = "PathBuf::from(\"./config.toml\")", short = 'c')]
    pub config: PathBuf,

    /// writes out the configuration file with overrides having been applied
    #[argh(switch)]
    pub write_config: bool,
}

impl CliOptions {
    pub fn parse() -> Result<Self, anyhow::Error> {
        let mut args: CliOptions = argh::from_env();

        if args.version {
            println!("Lantern Server {}", server::built::PKG_VERSION);
            std::process::exit(0);
        }

        if args.verbose.is_none() {
            if let Ok(verbose) = std::env::var("LANTERN_VERBOSE") {
                if let Ok(verbose) = verbose.parse() {
                    args.verbose = Some(verbose);
                }
            }
        }

        Ok(args)
    }
}
