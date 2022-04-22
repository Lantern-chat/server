
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Tasks {
    pub max_parallel_tasks: usize,
}

impl Default for Tasks {
    fn default() -> Tasks {
        Tasks {
            max_parallel_tasks: num_cpus::get(),
        }
    }
}