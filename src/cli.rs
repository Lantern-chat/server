#[derive(Debug, structopt::StructOpt)]
pub struct CliOptions {
    /// Logging level (0 = Info, 1 = Debug, 2 = Trace)
    #[structopt(short, long, env = "LANTERN_VERBOSE")]
    pub verbose: Option<u8>,

    #[structopt(long, env = "LANTERN_BIND")]
    pub bind: Option<String>,
}

impl CliOptions {
    pub fn prepare(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut bind) = self.bind {
            *bind = bind.replace("localhost", "127.0.0.1");
        }

        Ok(())
    }
}
