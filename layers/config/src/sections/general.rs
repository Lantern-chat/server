use super::util;

use uuid::Uuid;

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

        //pub memory_limit: u64 = crate::GIBIBYTE as u64 => "LANTERN_MEMORY_LIMIT" | util::parse[crate::GIBIBYTE as u64],

        /// The maximum number of CPU-intensive tasks that can run in parallel,
        /// defaults to the number of system threads.
        ///
        /// Setting this to 0 will use the default value.
        pub cpu_limit: u64 = num_cpus::get() as u64 => "LANTERN_CPU_LIMIT" | util::parse[num_cpus::get() as u64],
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
        }
    }
}
