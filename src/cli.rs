#[derive(Debug, structopt::StructOpt)]
pub struct CliOptions {
    /// Logging level (0 = Info, 1 = Debug, 2 = Trace)
    #[structopt(short, long, env = "LANTERN_VERBOSE")]
    pub verbose: Option<u8>,

    #[structopt(long, env = "LANTERN_BIND")]
    pub bind: Option<String>,
}
