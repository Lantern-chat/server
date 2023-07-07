use schema::Snowflake;

pub struct Heart {
    pub beats: scc::HashIndex<Snowflake, u32>,
    pub clock: quanta::Clock,
    pub start: u64,
}

impl Default for Heart {
    fn default() -> Self {
        let clock = quanta::Clock::new();

        Heart {
            beats: Default::default(),
            start: clock.raw(),
            clock,
        }
    }
}

impl Heart {
    fn now(&self) -> u32 {
        (self.clock.delta_as_nanos(self.start, self.clock.raw()) / 1_000_000_000) as u32
    }

    pub async fn beat(&self, conn_id: Snowflake) {
        use scc::hash_index::ModifyAction;

        let ts = self.now();
        _ = self.beats.modify_async(&conn_id, |_, _| ModifyAction::Update(ts)).await;
    }

    pub async fn add(&self, conn_id: Snowflake) {
        _ = self.beats.insert_async(conn_id, self.now()).await;
    }

    pub async fn remove(&self, conn_id: Snowflake) {
        _ = self.beats.remove_async(&conn_id).await;
    }
}
