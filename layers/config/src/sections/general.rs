use super::util;

use uuid::Uuid;

const HALF_GIB_AS_KIB: u64 = crate::GIBIBYTE as u64 / (2 * 1024);

section! {
    #[serde(default)]
    pub struct General {
        /// Name of your service
        pub server_name: String = "Lantern Chat".to_owned() => "LANTERN_SERVER_NAME",

        /// Unique identifier for your service, across all instances
        ///
        /// This is like a domain name, but should remain the same if the domain ever changes
        pub service_uuid: Uuid = Uuid::new_v4() => "LANTERN_SERVICE_UUID" | util::parse[Uuid::new_v4()],

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
        pub cpu_limit: u64 = 0u64 as u64 => "LANTERN_CPU_LIMIT" | util::parse[0u64],
    }
}

impl General {
    pub fn configure(&mut self) {
        use schema::sf;

        unsafe {
            sf::INST = self.instance_id;
            sf::WORK = self.worker_id;
        }

        if self.cpu_limit == 0 {
            self.cpu_limit = num_cpus::get() as u64;

            log::info!("Setting CPU limit to {}", self.cpu_limit);
        }

        if self.memory_limit == 0 {
            self.memory_limit = match process_utils::get_sysinfo() {
                None => HALF_GIB_AS_KIB,
                // divide by 2 and convert to KiB
                Some(sysinfo) => sysinfo.total_memory / (2 * 1024),
            };

            log::info!("Setting memory limit to {} KiB", self.memory_limit);
        }
    }
}
