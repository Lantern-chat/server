/// Heartbeat struct to keep track of the time since the gateway started
/// and calculate the current elapsed time in seconds.
///
/// This is used to determine
/// if a connection has timed out, e.g. missed a heartbeat.
///
/// The times are not guaranteed to be accurate to real time, but are
/// relatively accurate to the time since the gateway started.
pub struct Heart {
    clock: quanta::Clock,
    start: u64,
}

impl Default for Heart {
    fn default() -> Self {
        let clock = quanta::Clock::new();
        Heart { start: clock.raw(), clock }
    }
}

impl Heart {
    /// Calculate the current time since the gateway started, in seconds.
    pub fn now(&self) -> u32 {
        (self.clock.delta_as_nanos(self.start, self.clock.raw()) / 1_000_000_000) as u32
    }
}
