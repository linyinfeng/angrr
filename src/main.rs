mod options;

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::Context;
use clap::{CommandFactory, Parser};
use console::Term;
use dialoguer::Confirm;
use humantime::format_duration;
use options::{Interactive, Options};

fn main() -> anyhow::Result<()> {
    pretty_env_logger::try_init().context("failed to initialize logger")?;

    let options = Options::parse();
    match options.command {
        options::Commands::Run(run_opts) => run(run_opts),
        options::Commands::Completion(gen_options) => generate_shell_completions(gen_options),
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Remove,
    AboutToRemove,
}

#[derive(Debug, Clone)]
enum Reason {
    Expired { target: PathBuf, elapsed: Duration },
    TargetNotFound,
}

#[derive(Debug, Clone)]
struct ToRemove {
    link_path: PathBuf,
    reason: Reason,
}

fn run(run_opts: options::RunOptions) -> anyhow::Result<()> {
    let mut term = Term::stderr();
    let mut waiting = Vec::new();

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
                    let to_remove = ToRemove { link_path, reason };
                    match run_opts.interactive {
                        Interactive::Always => {
                            to_remove.notify(&run_opts, &mut term, Action::AboutToRemove, true)?;
                            let confirm = prompt(&run_opts, &term)?;
                            if confirm {
                                to_remove.notify(&run_opts, &mut term, Action::Remove, false)?;
                                to_remove.remove(&run_opts)?;
                            }
                        }
                        Interactive::Once => {
                            to_remove.notify(&run_opts, &mut term, Action::AboutToRemove, true)?;
                            waiting.push(to_remove);
                        }
                        Interactive::Never => {
                            to_remove.notify(&run_opts, &mut term, Action::Remove, true)?;
                            to_remove.remove(&run_opts)?;
                        }
                    }
                }
                None => log::debug!("keep {link_path:?}"),
            }
        }
    }

    if !waiting.is_empty() && prompt(&run_opts, &term)? {
        for to_remove in &waiting {
            to_remove.notify(&run_opts, &mut term, Action::Remove, false)?;
            to_remove.remove(&run_opts)?;
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

fn prompt(_run_opts: &options::RunOptions, term: &Term) -> anyhow::Result<bool> {
    Confirm::new()
        .with_prompt("Do you want to continue?")
        .report(false)
        .interact_on(term)
        .context("failed to prompt")
}

impl ToRemove {
    fn notify(
        &self,
        _run_opts: &options::RunOptions,
        term: &mut Term,
        action: Action,
        with_reason: bool,
    ) -> anyhow::Result<()> {
        term.write_fmt(format_args!(
            "{} {:?}\n",
            action.format_with_style(term),
            self.link_path
        ))?;
        if with_reason {
            let indented: Vec<String> = self
                .reason
                .format_with_style(term)
                .lines()
                .map(|l| format!("  {l}"))
                .collect();
            term.write_line(&indented.join("\n"))?;
        }
        Ok(())
    }

    fn remove(&self, run_opts: &options::RunOptions) -> anyhow::Result<()> {
        if !run_opts.dry_run {
            fs::remove_file(&self.link_path)
                .with_context(|| format!("failed to remove {:?}", self.link_path))?;
        }
        Ok(())
    }
}

impl Action {
    fn format_with_style(&self, term: &Term) -> String {
        match self {
            Action::Remove => term.style().green().bold().apply_to("Remove").to_string(),
            Action::AboutToRemove => term
                .style()
                .blue()
                .bold()
                .apply_to("About to remove")
                .to_string(),
        }
    }
}

impl Reason {
    fn format_with_style(&self, term: &Term) -> String {
        match self {
            Reason::Expired { target, elapsed } => format!(
                "target {:?}\nwas last modified {} ago",
                term.style().underlined().apply_to(target),
                term.style().bold().apply_to(format_duration(*elapsed))
            ),
            Reason::TargetNotFound => "target not found".to_string(),
        }
    }
}

fn generate_shell_completions(gen_options: options::CompletionOptions) -> anyhow::Result<()> {
    let mut cli = options::Options::command();
    let mut stdout = std::io::stdout();
    clap_complete::generate(gen_options.shell, &mut cli, "angrr", &mut stdout);
    Ok(())
}
