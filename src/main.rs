mod options;

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::Context;
use clap::{CommandFactory, Parser};
use humantime::format_duration;
use options::Options;

fn main() -> anyhow::Result<()> {
    pretty_env_logger::try_init().context("failed to initialize logger")?;

    let options = Options::parse();
    match options.command {
        options::Commands::Run(run_opts) => run(run_opts),
        options::Commands::Completion(gen_options) => generate_shell_completions(gen_options),
    }
}

enum Reason {
    Expired { target: PathBuf, elapsed: Duration },
    TargetNotFound,
}

fn run(run_opts: options::RunOptions) -> anyhow::Result<()> {
    let now = SystemTime::now();
    for path in &run_opts.directory {
        let directory =
            fs::read_dir(path).with_context(|| format!("failed to open directory {path:?}"))?;
        for entry in directory {
            let link = entry.with_context(|| {
                format!("failed to read directory entry from directory {path:?}")
            })?;
            let link_path = link.path();
            match check(&run_opts, &link_path, now)? {
                Some(reason) => {
                    println!("rm {link_path:?}: {}", reason);
                    if !run_opts.dry_run {
                        fs::remove_file(&link_path)
                            .with_context(|| format!("failed to remove {link_path:?}"))?;
                    }
                }
                None => log::debug!("keep {link_path:?}"),
            }
        }
    }
    Ok(())
}

fn check<P: AsRef<Path>>(
    run_opts: &options::RunOptions,
    link_path: P,
    now: SystemTime,
) -> anyhow::Result<Option<Reason>> {
    let link_path = link_path.as_ref();
    let target = fs::read_link(link_path)
        .with_context(|| format!("failed to read symbolic link {link_path:?}"))?;
    log::debug!("processing {link_path:?} -> {target:?}");
    let metadata = match fs::symlink_metadata(&target) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Some(Reason::TargetNotFound)),
        e => e.with_context(|| format!("failed to read metadata of file {target:?}"))?,
    };
    let target_mtime = metadata
        .modified()
        .with_context(|| format!("failed to get modified time of file {target:?}"))?;
    let elapsed = now
        .duration_since(target_mtime)
        .unwrap_or_else(|_| Duration::new(0, 0));
    log::debug!("elapsed: {}", humantime::format_duration(elapsed));
    if elapsed > run_opts.period {
        Ok(Some(Reason::Expired { target, elapsed }))
    } else {
        Ok(None)
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reason::Expired { target, elapsed } => write!(
                f,
                "target {target:?} was last modified {} ago",
                format_duration(*elapsed)
            ),
            Reason::TargetNotFound => write!(f, "target not found"),
        }
    }
}

fn generate_shell_completions(gen_options: options::CompletionOptions) -> anyhow::Result<()> {
    let mut cli = options::Options::command();
    let mut stdout = std::io::stdout();
    clap_complete::generate(gen_options.shell, &mut cli, "angrr", &mut stdout);
    Ok(())
}
