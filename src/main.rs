mod options;

use std::{
    fs,
    io::{self, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, SystemTime},
};

use anyhow::Context;
use clap::{crate_name, CommandFactory, Parser};
use console::Term;
use dialoguer::Confirm;
use humantime::format_duration;
use options::{Interactive, Options, RunOptions};

fn main() -> anyhow::Result<()> {
    let carte_name = crate_name!();

    let mut builder = pretty_env_logger::formatted_builder();
    let filters = match std::env::var("RUST_LOG") {
        Ok(f) => f,
        Err(_) => format!("{carte_name}=info"),
    };
    builder.parse_filters(&filters);
    builder.try_init()?;

    let options = Options::parse();

    match options.command {
        options::Commands::Run(run_opts) => {
            let context = RunContext::new(run_opts)?;
            context.run()?;
            context.finish()
        }
        options::Commands::Completion(gen_options) => {
            generate_shell_completions(gen_options, carte_name)
        }
    }
}

#[derive(Debug)]
struct RunContext {
    options: RunOptions,
    uid: u32,
    now: SystemTime,
    term: Term,
    statistic: Statistics,
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
struct ToRemove<'c> {
    context: &'c RunContext,
    link_path: PathBuf,
    reason: Reason,
}

#[derive(Debug, Default)]
struct Statistics {
    traversed: Counter,
    candidate: Counter,
    removed: Counter,
}

#[derive(Debug, Default)]
struct Counter(AtomicUsize);

impl RunContext {
    fn new(options: RunOptions) -> anyhow::Result<Self> {
        let uid = uzers::get_current_uid();
        let now = SystemTime::now();
        let term = Term::stderr();
        let statistic = Default::default();
        let mut context = Self {
            options,
            uid,
            now,
            term,
            statistic,
        };
        context.adjust_options()?;
        log::debug!("options: {:#?}", context.options);
        Ok(context)
    }

    fn run(&self) -> anyhow::Result<()> {
        let mut waiting = Vec::new();

        for path in &self.options.directory {
            let directory =
                fs::read_dir(path).with_context(|| format!("failed to open directory {path:?}"))?;
            for entry in directory {
                self.statistic.traversed.increase();
                let link = entry.with_context(|| {
                    format!("failed to read directory entry from directory {path:?}")
                })?;
                let link_path = link.path();
                match self.check(&link_path)? {
                    Some(reason) => {
                        self.statistic.candidate.increase();
                        let to_remove = ToRemove {
                            context: self,
                            link_path,
                            reason,
                        };
                        match self.options.interactive {
                            Interactive::Always => {
                                to_remove.notify(Action::AboutToRemove, true)?;
                                let yes = self.prompt()?;
                                if yes {
                                    to_remove.notify(Action::Remove, false)?;
                                    to_remove.remove()?;
                                }
                            }
                            Interactive::Once => {
                                to_remove.notify(Action::AboutToRemove, true)?;
                                waiting.push(to_remove);
                            }
                            Interactive::Never => {
                                to_remove.notify(Action::Remove, true)?;
                                to_remove.remove()?;
                            }
                        }
                    }
                    None => log::debug!("keep {link_path:?}"),
                }
            }
        }

        if !waiting.is_empty() && self.prompt()? {
            for to_remove in &waiting {
                to_remove.notify(Action::Remove, false)?;
                to_remove.remove()?;
            }
        }

        Ok(())
    }

    fn finish(mut self) -> anyhow::Result<()> {
        if !self.options.no_statistic {
            writeln!(
                self.term,
                "{}",
                self.term.style().bold().underlined().apply_to("Statistics")
            )?;
            self.term
                .write_line(&self.statistic.format_with_style(&self.term))?;
        }
        Ok(())
    }

    fn check<P: AsRef<Path>>(&self, link_path: P) -> anyhow::Result<Option<Reason>> {
        let link_path = link_path.as_ref();
        let target = fs::read_link(link_path)
            .with_context(|| format!("failed to read symbolic link {link_path:?}"))?;
        log::debug!("processing {link_path:?} -> {target:?}");
        let metadata = match fs::symlink_metadata(&target) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if self.options.include_not_found {
                    return Ok(Some(Reason::TargetNotFound));
                } else {
                    return Ok(None);
                }
            }
            e => e.with_context(|| format!("failed to read metadata of file {target:?}"))?,
        };
        if self.options.owned_only {
            let file_uid = metadata.uid();
            if file_uid != self.uid {
                log::debug!("ignore {target:?} due to uid mismatch: file uid == {file_uid}, process uid == {process_uid}",
                  process_uid = self.uid);
                return Ok(None);
            }
        }
        let target_mtime = metadata
            .modified()
            .with_context(|| format!("failed to get modified time of file {target:?}"))?;
        let elapsed = self
            .now
            .duration_since(target_mtime)
            .unwrap_or_else(|_| Duration::new(0, 0));
        log::debug!("elapsed: {}", humantime::format_duration(elapsed));
        if elapsed > self.options.period {
            Ok(Some(Reason::Expired { target, elapsed }))
        } else {
            Ok(None)
        }
    }

    fn prompt(&self) -> anyhow::Result<bool> {
        Confirm::new()
            .with_prompt("Do you want to continue?")
            .report(false)
            .interact_on(&self.term)
            .context("failed to prompt")
    }

    fn adjust_options(&mut self) -> anyhow::Result<()> {
        if self.options.interactive != Interactive::Never {
            let all_directory_is_owned = self.all_directory_is_owned()?;
            if !all_directory_is_owned && (!self.options.owned_only || !self.options.remove_target)
            {
                let yes = Confirm::new()
                    .with_prompt("Some directory is not owned by you, would you like to turn on the `--owned-only` and `--remove-target` options?")
                    .interact_on(&self.term)
                    .context("failed to prompt")?;
                if yes {
                    self.options.owned_only = true;
                    self.options.remove_target = true;
                }
            }
        }

        if self.options.include_not_found && (self.options.owned_only || self.options.remove_target)
        {
            log::warn!("the `--include-not-found` option will be ignored because it is conflict with `--owned-only` and `--remove-target`");
            self.options.include_not_found = false;
        }
        Ok(())
    }

    fn all_directory_is_owned(&self) -> anyhow::Result<bool> {
        let mut is_uid_match = true;
        for path in &self.options.directory {
            let metadata = fs::metadata(path)
                .with_context(|| format!("failed to read metadata of directory {path:?}"))?;
            is_uid_match &= self.uid == metadata.uid();
        }
        Ok(is_uid_match)
    }
}

impl<'c> ToRemove<'c> {
    fn options(&self) -> &RunOptions {
        &self.context.options
    }

    fn notify(&self, action: Action, with_reason: bool) -> anyhow::Result<()> {
        let mut term = self.context.term.clone();
        let reason_indent = 2;
        if self.options().remove_target {
            // remove target
            writeln!(
                term,
                "{} {:?}",
                action.format_with_style(&term),
                self.reason.target()?
            )?;
            if with_reason {
                term.write_line(&add_indent(
                    &self.reason.format_with_style_no_target(&term),
                    reason_indent,
                ))?;
            }
        } else {
            // remove link
            writeln!(
                term,
                "{} {:?}",
                action.format_with_style(&term),
                self.link_path
            )?;
            if with_reason {
                term.write_line(&add_indent(
                    &self.reason.format_with_style(&term),
                    reason_indent,
                ))?;
            }
        }

        Ok(())
    }

    fn remove(&self) -> anyhow::Result<()> {
        self.context.statistic.removed.increase();
        if !self.options().dry_run {
            if self.options().remove_target {
                fs::remove_file(self.reason.target()?)
                    .with_context(|| format!("failed to remove {:?}", self.reason.target()))?;
            } else {
                fs::remove_file(&self.link_path)
                    .with_context(|| format!("failed to remove {:?}", self.link_path))?;
            }
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

    fn format_with_style_no_target(&self, term: &Term) -> String {
        match self {
            Reason::Expired { elapsed, .. } => format!(
                "was last modified {} ago",
                term.style().bold().apply_to(format_duration(*elapsed))
            ),
            Reason::TargetNotFound => "target not found".to_string(),
        }
    }

    fn target(&self) -> anyhow::Result<&Path> {
        match self {
            Reason::Expired { target, .. } => Ok(target),
            Reason::TargetNotFound => anyhow::bail!("failed to determine target"),
        }
    }
}

impl Statistics {
    fn format_with_style(self, term: &Term) -> String {
        let traversed = self.traversed.done();
        let candidate = self.candidate.done();
        let removed = self.removed.done();
        let kept = traversed - removed;
        let num_style = |n| term.style().bold().apply_to(n);
        [
            format!("traversed: {}", num_style(traversed)),
            format!("candidate: {}", num_style(candidate)),
            format!("removed:   {}", num_style(removed)),
            format!("kept:      {}", num_style(kept)),
        ]
        .join("\n")
    }
}

impl Counter {
    fn increase(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    fn done(self) -> usize {
        self.0.into_inner()
    }
}

fn add_indent(text: &str, indent: usize) -> String {
    let indented_lines: Vec<_> = text
        .lines()
        .map(|l| format!("{:indent$}{}", "", l))
        .collect();
    indented_lines.join("\n")
}

fn generate_shell_completions(
    gen_options: options::CompletionOptions,
    command_name: &str,
) -> anyhow::Result<()> {
    let mut cli = options::Options::command();
    let mut stdout = std::io::stdout();
    clap_complete::generate(gen_options.shell, &mut cli, command_name, &mut stdout);
    Ok(())
}
