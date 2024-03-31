use std::process::{Command, Stdio};

#[derive(argh::FromArgs)]
/// Build Lantern components
pub struct CliArgs {
    /// projects to build
    #[argh(option, short = 'p')]
    project: String,

    /// how many processes to use to build
    #[argh(option, short = 'j', default = "0")]
    jobs: usize,

    /// target triple
    #[argh(option)]
    target: String,
}

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

#[rustfmt::skip]
impl Target {
    const MAPPING: &'static [(&'static str, Target)] = &[
        ("x86_64_v3_unknown_linux_musl", Target::x86_64_v3_unknown_linux_musl),
        ("aarch64_unknown_linux_musl_cortex_a57", Target::aarch64_unknown_linux_musl_cortex_a57),
        ("x86_64_v3_pc_windows_msvc", Target::x86_64_v3_pc_windows_msvc),
    ];

    fn parse(target: &str) -> Option<Target> {
        let target = target.replace('-', "_");
        Self::MAPPING.iter().find(|(k, _)| *k == target).map(|(_, v)| *v)
    }

    fn target_rustflags(self) -> (&'static str, &'static str) {
        match self {
            Target::x86_64_v3_unknown_linux_musl => (
                "x86_64-unknown-linux-musl",
                "-C target-cpu=x86-64-v3 -C target-feature=+aes",
            ),
            Target::aarch64_unknown_linux_musl_cortex_a57 => (
                "aarch64-unknown-linux-musl",
                "-C target-cpu=cortex-a57 -C target-feature=-outline-atomics",
            ),
            Target::x86_64_v3_pc_windows_msvc => (
                "x86_64-pc-windows-msvc",
                "-C target-cpu=x86-64-v3 -C target-feature=+aes",
            ),
        }
    }

    fn err() -> ! {
        eprintln!("Invalid target, expected one of:");
        for (k, _) in Target::MAPPING {
            eprintln!("    {k}");
        }
        std::process::exit(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Project {
    Nexus,
    Gateway,
    Process,
    Camo2,
    EmbedWorker,
    GifProbe,
}

impl Project {
    const MAPPING: &'static [(&'static str, Project)] = &[
        ("nexus", Project::Nexus),
        ("gateway", Project::Gateway),
        ("process", Project::Process),
        ("camo2", Project::Camo2),
        ("embed_worker", Project::EmbedWorker),
        ("gif_probe", Project::GifProbe),
    ];

    fn all() -> Vec<Project> {
        Self::MAPPING.iter().map(|(_, v)| *v).collect()
    }

    fn parse(project: &str) -> Option<Project> {
        Self::MAPPING.iter().find(|(k, _)| *k == project).map(|(_, p)| *p)
    }

    #[rustfmt::skip]
    fn args(self) -> &'static str {
        match self {
            Project::Nexus => r#"--bin nexus -p nexus"#,
            Project::Gateway => r#"--bin gateway -p gateway"#,
            Project::Process => r#"--bin process -p process --features binary"#,
            Project::Camo2 => r#"--bin camo2 -p camo-worker --config lib.crate-type=["bin"] --no-default-features --features standalone"#,
            Project::EmbedWorker => todo!(),
            Project::GifProbe => r#"--bin gif_probe -p gif_probe"#,
        }
    }

    fn err(verb: &str) -> ! {
        eprintln!("{verb} project, expected one or more of:");
        for (k, _) in Project::MAPPING {
            eprintln!("    {k}");
        }
        std::process::exit(1);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: CliArgs = argh::from_env();

    let Some(target) = Target::parse(&args.target) else {
        Target::err();
    };

    let projects = if args.project.contains("all") {
        Project::all()
    } else {
        let mut projects = Vec::new();

        for p in args.project.split_whitespace() {
            let Some(project) = Project::parse(p) else {
                Project::err("Unknown");
            };

            projects.push(project);
        }

        if projects.is_empty() {
            Project::err("Missing");
        }

        projects.sort_unstable();
        projects.dedup();

        projects
    };

    let (target, rustflags) = target.target_rustflags();

    let jobs = if args.jobs > 0 { format!("-j {}", args.jobs) } else { String::new() };

    let shared = format!(
        r#"{jobs} --config build.rustflags=["{}"] --config profile.release.strip=true --target {target} --release"#,
        rustflags.split_whitespace().collect::<Vec<&str>>().join(r#"",""#)
    );

    for project in projects {
        let mut cmd = Command::new("cross");
        cmd.arg("build").args(shared.split_whitespace()).args(project.args().split_whitespace());
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        println!("Building {project:?}");
        println!("Running {cmd:?}");

        cmd.spawn()?.wait()?;
    }

    Ok(())
}
