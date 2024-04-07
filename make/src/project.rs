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

    pub fn all() -> Vec<Project> {
        Self::MAPPING.iter().map(|(_, v)| *v).collect()
    }

    pub fn parse(project: &str) -> Option<Project> {
        Self::MAPPING.iter().find(|(k, _)| *k == project).map(|(_, p)| *p)
    }

    #[rustfmt::skip]
    pub fn args(self) -> &'static str {
        match self {
            Project::Nexus => r#"--bin nexus -p nexus"#,
            Project::Gateway => r#"--bin gateway -p gateway"#,
            Project::Process => r#"--bin process -p process --features binary"#,
            Project::Camo2 => r#"--bin camo2 -p camo-worker --config lib.crate-type=["bin"] --no-default-features --features standalone"#,
            Project::EmbedWorker => todo!(),
            Project::GifProbe => r#"--bin gif_probe -p gif_probe"#,
        }
    }

    pub fn err(verb: &str) -> ! {
        eprintln!("{verb} project, expected one or more of:");
        for (k, _) in Project::MAPPING {
            eprintln!("    {k}");
        }
        std::process::exit(1);
    }
}
