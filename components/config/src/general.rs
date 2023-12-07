use crate::util;

const HALF_GIB_AS_KIB: u64 = crate::GIBIBYTE as u64 / (2 * 1024);

crate::section! {
    #[serde(default)]
    pub struct General {
        /// ID of the sharded instance, just keep it at 0 if using one instance
        pub instance_id: u16 = 0 => "LANTERN_INSTANCE_ID" | util::parse[0u16],

        /// ID of worker machine, just keep it at 0 if using one machine
        pub worker_id: u16 = 0 => "LANTERN_WORKER_ID" | util::parse[0u16],

        /// Memory limit (in kibibytes) for core tasks, does NOT including image encoding
        ///
        /// Default value for this is half of the available system memory
        pub memory_limit: u64 = 0u64 => "LANTERN_MEMORY_LIMIT" | util::parse[0u64],

        /// The maximum number of CPU-intensive tasks that can run in parallel,
        /// defaults to the number of system threads.
        ///
        /// Setting this to 0 will use the default value.
        pub cpu_limit: u64 = 0u64 => "LANTERN_CPU_LIMIT" | util::parse[0u64],
    }

    impl Extra {
        fn configure(&mut self) {
            if self.cpu_limit == 0 {
                self.cpu_limit = num_cpus::get() as u64;

                tracing::info!("Setting CPU limit to {}", self.cpu_limit);
            }

            if self.memory_limit == 0 {
                self.memory_limit = match process_utils::get_sysinfo() {
                    None => HALF_GIB_AS_KIB,
                    // divide by 2 and convert to KiB
                    Some(sysinfo) => sysinfo.total_memory / (2 * 1024),
                };

                tracing::info!("Setting memory limit to {} KiB", self.memory_limit);
            }
        }
    }
}
