use super::util;

section! {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct General {
        pub server_name: String = "Lantern Chat".to_owned() => "LANTERN_SERVER_NAME",
        pub instance_id: u16 = 0 => "LANTERN_INSTANCE_ID" | util::parse[0u16],
        pub worker_id: u16 = 0 => "LANTERN_WORKER_ID" | util::parse[0u16],
    }
}

impl General {
    pub fn configure(&self) {
        use schema::sf;

        unsafe {
            sf::INST = self.instance_id;
            sf::WORK = self.worker_id;
        }
    }
}
