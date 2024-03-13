pub mod queue;
pub mod queue2;
pub mod strip;

pub use queue::{Queue, WorkItem};

pub struct Queues {
    pub embed_processing: Queue,
}

impl Default for Queues {
    fn default() -> Self {
        Queues {
            embed_processing: Queue::start(64),
        }
    }
}
