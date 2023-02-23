section! {
    #[serde(default)]
    pub struct Services {
        pub hcaptcha_secret: String     = "0x0000000000000000000000000000000000000000".to_owned()   => "HCAPTCHA_SECRET",
        pub hcaptcha_sitekey: String    = "10000000-ffff-ffff-ffff-000000000001".to_owned()         => "HCAPTCHA_SITEKEY",

        pub b2_app: String              = String::default() => "B2_APP",
        pub b2_key: String              = String::default() => "B2_KEY",

        pub embed_worker_uri: String    = "http://127.0.0.1:8766".to_owned() => "EMBED_WORKER_URI",
    }
}
