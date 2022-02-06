use std::{net::SocketAddr, str::FromStr};

#[derive(Debug)]
pub struct CliOptions {
    pub verbose: Option<u8>,
    pub bind: SocketAddr,
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

        // parse bind address or fallback to environment variable
        let bind: Option<String> = match pargs.opt_value_from_str("--bind")? {
            Some(v) => Some(v),
            None => std::env::var("LANTERN_BIND").ok(),
        };

        log::trace!("Parsing bind address...");
        let bind = match bind {
            Some(bind) => SocketAddr::from_str(&bind.replace("localhost", "127.0.0.1"))?,
            None => SocketAddr::from(([127, 0, 0, 1], 3030)),
        };

        Ok(CliOptions { verbose, bind })
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
        --bind <address>    Server bind address/port [env LANTERN_BIND]
    -v, --verbose <level>   Logging level (0 = Info, 1 = Debug, 2 = Trace) [env LANTERN_VERBOSE]
";
