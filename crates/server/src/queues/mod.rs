pub mod embed;
pub mod queue;
pub mod strip;

pub use queue::{Queue, WorkFunction, WorkItem};

pub struct Queues {
    pub embed_processing: Queue,
}

impl Default for Queues {
    fn default() -> Self {
        Queues {
            embed_processing: Queue::start(16),
        }
    }
}

impl Queues {
    pub fn stop(&self) {
        self.embed_processing.stop();
    }
}
