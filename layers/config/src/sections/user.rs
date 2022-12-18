section! {
    #[serde(default)]
    pub struct User {
        /// How much "randomness" will be applied to skew relative times (default `0.1` for 10% skew)
        pub relative_time_random_factor: f32 = 0.1,
    }
}
