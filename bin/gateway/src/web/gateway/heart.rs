pub struct Heart {
    pub clock: quanta::Clock,
    pub start: u64,
}

impl Default for Heart {
    fn default() -> Self {
        let clock = quanta::Clock::new();
        Heart { start: clock.raw(), clock }
    }
}

impl Heart {
    pub fn now(&self) -> u32 {
        (self.clock.delta_as_nanos(self.start, self.clock.raw()) / 1_000_000_000) as u32
    }
}
