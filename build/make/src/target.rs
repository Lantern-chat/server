#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Target {
    /// Generic modern Linux
    x86_64_v3_unknown_linux_musl,
    /// Windows PC
    x86_64_v3_pc_windows_msvc,
    /// Raspberry Pi 3B+
    aarch64_unknown_linux_musl_cortex_a57,
}

impl Target {
    #[rustfmt::skip]
    const MAPPING: &'static [(&'static str, Target)] = &[
        ("x86_64_v3_unknown_linux_musl", Target::x86_64_v3_unknown_linux_musl),
        ("aarch64_unknown_linux_musl_cortex_a57", Target::aarch64_unknown_linux_musl_cortex_a57),
        ("x86_64_v3_pc_windows_msvc", Target::x86_64_v3_pc_windows_msvc),
    ];

    pub fn parse(target: &str) -> Option<Target> {
        let target = target.replace('-', "_");
        Self::MAPPING.iter().find(|(k, _)| *k == target).map(|(_, v)| *v)
    }

    pub fn target_rustflags(self) -> (&'static str, &'static str) {
        match self {
            Target::x86_64_v3_unknown_linux_musl => (
                "x86_64-unknown-linux-musl",
                "-C target-cpu=x86-64-v3 -C target-feature=+aes -C opt-level=3 -C codegen-units=1",
            ),
            Target::x86_64_v3_pc_windows_msvc => (
                "x86_64-pc-windows-msvc",
                "-C target-cpu=x86-64-v3 -C target-feature=+aes -C opt-level=3 -C codegen-units=1",
            ),
            Target::aarch64_unknown_linux_musl_cortex_a57 => (
                "aarch64-unknown-linux-musl",
                "-C target-cpu=cortex-a57 -C target-feature=-outline-atomics -C opt-level=3 -C codegen-units=1",
            ),
        }
    }

    pub fn err() -> ! {
        eprintln!("Invalid target, expected one of:");
        for (k, _) in Target::MAPPING {
            eprintln!("    {k}");
        }
        std::process::exit(1);
    }
}
