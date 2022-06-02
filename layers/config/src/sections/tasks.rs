section! {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct Tasks {
        pub max_parallel_tasks: usize = num_cpus::get(),
    }
}
