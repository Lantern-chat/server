use std::path::PathBuf;

#[derive(Debug)]
pub struct CliOptions {
    pub verbose: Option<u8>,
    pub config_path: PathBuf,
    pub write_config: bool,
}

impl CliOptions {
    pub fn parse() -> Result<Self, anyhow::Error> {
        let mut pargs = pico_args::Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            print!("{}", HELP);
            std::process::exit(0);
        }

        if pargs.contains(["-V", "--version"]) {
            println!("Lantern Server {}", server::built::PKG_VERSION);
            std::process::exit(0);
        }

        // parse verbose parameter or fallback to environment variable
        let verbose: Option<u8> = match pargs.opt_value_from_str(["-v", "--verbose"])? {
            Some(v) => Some(v),
            None => std::env::var("LANTERN_VERBOSE").ok().and_then(|s| s.parse().ok()),
        };

        let mut config_path = PathBuf::from("./config.toml");
        if let Some(v) = pargs.opt_value_from_str::<_, String>(["-c", "--config"])? {
            config_path = PathBuf::from(v);
        }

        let write_config = pargs.contains("write-config");

        Ok(CliOptions {
            verbose,
            config_path,
            write_config,
        })
    }
}

static HELP: &'static str = "\
Lantern

USAGE:
    lantern [OPTIONS]

FLAGS:
    -h, --help      Prints help information
    -V, --version   Prints version information

OPTIONS:
        --config <path>     Lantern configuration file location
    -v, --verbose <level>   Logging level (0 = Info, 1 = Debug, 2 = Trace) [env LANTERN_VERBOSE]
";
