use std::process::{Command, Stdio};

pub mod project;
pub mod target;

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

use self::{project::Project, target::Target};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: CliArgs = argh::from_env();

    let Some(target) = Target::parse(&args.target) else {
        Target::err();
    };

    let projects = 'projects: {
        let mut projects = Vec::new();

        for p in args.project.split_whitespace() {
            if p == "all" {
                break 'projects Project::all();
            }

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
