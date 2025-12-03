mod filter;
mod options;

use std::{
    ffi::OsStr,
    fmt::Debug,
    fs::{self, File},
    io::{self, BufWriter, Write, sink, stdout},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime},
};

use anyhow::Context;
use clap::{CommandFactory, Parser, crate_name};
use dialoguer::{Confirm, console::Term};
use humantime::format_duration;
use nix::{
    fcntl::AT_FDCWD,
    sys::{
        stat::{UtimensatFlags, utimensat},
        time::TimeSpec,
    },
};
use options::{Interactive, Options, RunOptions};
use uzers::{get_user_by_uid, os::unix::UserExt};

use crate::{
    filter::Filter,
    options::{CommonOptions, TouchOptions},
};

fn main() -> anyhow::Result<()> {
    let crate_name = crate_name!();

    let options = Options::parse();

    let mut logger_builder = pretty_env_logger::formatted_builder();

    // default logger configuration
    let default_log_level = log::LevelFilter::Info;
    logger_builder.filter_module(crate_name, default_log_level);
    // overrides
    if let Ok(filter) = std::env::var("RUST_LOG") {
        logger_builder.parse_filters(&filter);
    };
    if options.common.verbose != 0 {
        let mut iter = log::LevelFilter::iter().fuse();
        // find the default log level
        iter.find(|level| *level == default_log_level);
        for _ in 0..(options.common.verbose - 1) {
            iter.next();
        }
        // since our iter is a Fuse, it must return None if we already reach the max level
        let level = match iter.next() {
            Some(l) => l,
            None => log::LevelFilter::max(),
        };
        logger_builder.filter_module(crate_name, level);
    }
    if let Some(level) = options.common.log_level {
        logger_builder.filter_module(crate_name, level);
    }
    let builder_debug_info = format!("{logger_builder:?}");
    logger_builder.try_init()?;
    log::debug!("logger initialized with configuration: {builder_debug_info}");

    match options.command {
        options::Commands::Run(run_opts) => {
            let context = RunContext::new(options.common, run_opts)?;
            log::trace!("context = {context:#?}");
            context.run()?;
            context.finish()
        }
        options::Commands::Touch(touch_opts) => {
            let context = TouchContext::new(options.common, touch_opts);
            log::trace!("context = {context:#?}");
            context.touch()
        }
        options::Commands::Completion(gen_options) => {
            generate_shell_completions(gen_options, crate_name)
        }
    }
}

#[derive(Debug)]
struct RunContext {
    common_options: CommonOptions,
    options: RunOptions,
    external_filter: Option<Filter>,
    uid: u32,
    now: SystemTime,
    term: Term,
    output: Mutex<Output>,
    statistic: Statistics,
}

#[derive(Debug)]
struct Item {
    link_path: PathBuf,
    reason: Reason,
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Remove,
    AboutToRemove,
    Ignored,
}

#[derive(Debug, Clone)]
struct Reason {
    target: PathBuf,
    elapsed: Duration,
}

#[derive(Debug, Default)]
struct Statistics {
    traversed: Counter,
    candidate: Counter,
    invalid: Counter,
    removed: Counter,
}

#[derive(Debug)]
struct Output {
    writer: Box<dyn OutputWriter>,
    first_output: bool,
}

trait OutputWriter: Write + Debug {}
impl<T> OutputWriter for T where T: Write + Debug {}

#[derive(Debug, Default)]
struct Counter(AtomicUsize);

impl RunContext {
    fn new(common_options: CommonOptions, options: RunOptions) -> anyhow::Result<Self> {
        let external_filter = options.filter.as_ref().map(|program| Filter {
            program: program.clone(),
            arguments: options.filter_args.clone(),
        });
        let uid = uzers::get_current_uid();
        let now = SystemTime::now();
        let term = Term::stderr();
        let output = Mutex::new(Output {
            writer: Self::output_writer(&options)?,
            first_output: true,
        });
        let statistic = Default::default();
        let context = Self {
            common_options,
            options,
            external_filter,
            uid,
            now,
            term,
            output,
            statistic,
        };
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
                        let item = Item { link_path, reason };
                        match self.options.interactive {
                            Interactive::Always => {
                                self.notify(&item, Action::AboutToRemove, true)?;
                                let yes = self.prompt()?;
                                if yes {
                                    self.remove(&item)?;
                                } else {
                                    self.notify(&item, Action::Ignored, true)?;
                                }
                            }
                            Interactive::Once => {
                                self.notify(&item, Action::AboutToRemove, true)?;
                                waiting.push(item);
                            }
                            Interactive::Never => {
                                self.remove(&item)?;
                            }
                        }
                    }
                    None => log::trace!("keep {link_path:?}"),
                }
            }
        }

        if !waiting.is_empty() && self.prompt()? {
            for item in &waiting {
                self.remove(item)?;
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
        let mut output = self.output.lock().unwrap();
        output.writer.flush().context("failed to flush output")?;
        Ok(())
    }

    fn check<P: AsRef<Path>>(&self, link_path: P) -> anyhow::Result<Option<Reason>> {
        let link_path = link_path.as_ref();
        let target = fs::read_link(link_path)
            .with_context(|| format!("failed to read symbolic link {link_path:?}"))?;
        log::trace!("processing {link_path:?} -> {target:?}");
        let metadata = match fs::symlink_metadata(&target) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                log::debug!("target of {link_path:?} not found, skip");
                return Ok(None);
            }
            Err(e) => {
                log::warn!("ignore {target:?}, can not read metadata: {e}");
                return Ok(None);
            }
        };
        if self.ignored(&target) || self.ignored_in_home(&target, &metadata) {
            log::debug!("ignore {target:?}");
            return Ok(None);
        }
        if self.options.owned_only {
            let file_uid = metadata.uid();
            if file_uid != self.uid {
                log::debug!(
                    "ignore {target:?} due to uid mismatch: file uid == {file_uid}, process uid == {process_uid}",
                    process_uid = self.uid
                );
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
        log::trace!("elapsed: {}", humantime::format_duration(elapsed));
        if elapsed <= self.options.period {
            return Ok(None);
        }

        // finally call the external filter
        if let Some(filter) = &self.external_filter {
            let input = filter::Input {
                path: target.clone(),
                gc_root: link_path.to_path_buf(),
            };
            let not_ignored = filter
                .run(&input)
                .with_context(|| format!("failed to run filter on input: {input:?}"))?;
            if !not_ignored {
                log::debug!("ignore {target:?} due to external filter decision");
                return Ok(None);
            }
        }

        Ok(Some(Reason { target, elapsed }))
    }

    fn validate_and_prompt<P: AsRef<Path>>(&self, target: P) -> anyhow::Result<bool> {
        let target = target.as_ref();
        if !validate_store_path(&self.common_options.store, target) {
            self.statistic.invalid.increase();
            let mut term = self.term.clone();
            let fail_message_style = if self.options.force {
                term.style().bold().yellow()
            } else {
                term.style().bold().red()
            };
            writeln!(
                term,
                "{}, target {:?} does not point into store {:?}",
                fail_message_style.apply_to("Validation failed"),
                term.style().underlined().apply_to(&target),
                self.common_options.store
            )?;
            if self.options.force {
                Ok(true)
            } else if self.options.interactive == Interactive::Never {
                Ok(false)
            } else if self.prompt()? {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(true)
        }
    }

    fn prompt(&self) -> anyhow::Result<bool> {
        Confirm::new()
            .with_prompt("Do you want to continue?")
            .report(false)
            .interact_on(&self.term)
            .context("failed to prompt")
    }

    fn notify(&self, item: &Item, action: Action, with_reason: bool) -> anyhow::Result<()> {
        let mut term = &self.term;
        let reason_indent = 2;
        if self.options.remove_root {
            // remove link
            writeln!(
                term,
                "{} {:?}",
                action.format_with_style(term),
                item.link_path
            )?;
            if with_reason {
                term.write_line(&add_indent(
                    &item.reason.format_with_style(term),
                    reason_indent,
                ))?;
            }
        } else {
            // remove target
            writeln!(
                term,
                "{} {:?}",
                action.format_with_style(term),
                item.reason.target
            )?;
            if with_reason {
                term.write_line(&add_indent(
                    &item.reason.format_with_style_no_target(term),
                    reason_indent,
                ))?;
            }
        }
        Ok(())
    }

    fn remove(&self, item: &Item) -> anyhow::Result<()> {
        let path_to_remove = if self.options.remove_root {
            &item.link_path
        } else {
            // validate before remove target
            let target = &item.reason.target;
            if !self.validate_and_prompt(target)? {
                self.notify(item, Action::Ignored, false)?;
                return Ok(());
            }
            target
        };
        self.notify(item, Action::Remove, false)?;
        if !self.options.dry_run {
            fs::remove_file(path_to_remove)
                .with_context(|| format!("failed to remove {path_to_remove:?}"))?;
        }
        self.statistic.removed.increase();
        let mut out = self.output.lock().unwrap();
        out.output(path_to_remove, &self.options.output_delimiter)?;
        Ok(())
    }

    fn output_writer(options: &RunOptions) -> anyhow::Result<Box<dyn OutputWriter>> {
        match &options.output {
            Some(path) => {
                let mut writer: Box<dyn OutputWriter> = if path == &PathBuf::from("-") {
                    Box::new(stdout())
                } else {
                    Box::new(
                        File::create(path)
                            .with_context(|| format!("failed to create output file {path:?}"))?,
                    )
                };
                if !options.output_unbuffered {
                    writer = Box::new(BufWriter::new(writer));
                }
                Ok(writer)
            }
            None => Ok(Box::new(sink())),
        }
    }

    fn ignored<P: AsRef<Path>>(&self, target: P) -> bool {
        let p = target.as_ref();
        for prefix in &self.options.ignore_directories {
            if p.starts_with(prefix) {
                return true;
            }
        }
        false
    }

    fn ignored_in_home<P: AsRef<Path>>(&self, target: P, metadata: &fs::Metadata) -> bool {
        let p = target.as_ref();
        let uid = metadata.uid();
        let user = match get_user_by_uid(uid) {
            None => return false,
            Some(user) => user,
        };
        let home = user.home_dir();
        for prefix in &self.options.ignore_directories_in_home {
            let full_prefix = home.join(prefix);
            if p.starts_with(full_prefix) {
                return true;
            }
        }
        false
    }
}

fn validate_store_path<P1: AsRef<Path>, P2: AsRef<Path>>(store: P1, target: P2) -> bool {
    let store = store.as_ref();
    let target = target.as_ref();
    match fs::canonicalize(target) {
        Ok(path) => path.starts_with(store),
        Err(e) => {
            log::warn!("failed to canonicalize {target:?} for validation: {e}");
            false
        }
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
            Action::Ignored => term.style().cyan().bold().apply_to("Ignore").to_string(),
        }
    }
}

impl Reason {
    fn format_with_style(&self, term: &Term) -> String {
        let Self { target, elapsed } = self;
        format!(
            "target {:?}\nwas last modified {} ago",
            term.style().underlined().apply_to(target),
            term.style().bold().apply_to(format_duration(*elapsed))
        )
    }

    fn format_with_style_no_target(&self, term: &Term) -> String {
        let Self { elapsed, .. } = self;
        format!(
            "was last modified {} ago",
            term.style().bold().apply_to(format_duration(*elapsed))
        )
    }
}

impl Statistics {
    fn format_with_style(self, term: &Term) -> String {
        let traversed = self.traversed.done();
        let candidate = self.candidate.done();
        let removed = self.removed.done();
        let invalid = self.invalid.done();
        let kept = traversed - removed;
        let num_style = |n| term.style().bold().apply_to(n);
        [
            format!("traversed: {}", num_style(traversed)),
            format!("candidate: {}", num_style(candidate)),
            format!("removed:   {}", num_style(removed)),
            format!("invalid:   {}", num_style(invalid)),
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

impl Output {
    fn output<P: AsRef<Path>>(&mut self, path: P, delimiter: &OsStr) -> anyhow::Result<()> {
        let p = path.as_ref();
        if !self.first_output {
            self.writer.write_all(delimiter.as_bytes())?;
        } else {
            self.first_output = false;
        }
        self.writer.write_all(p.as_os_str().as_bytes())?;
        Ok(())
    }
}

fn add_indent(text: &str, indent: usize) -> String {
    let indented_lines: Vec<_> = text
        .lines()
        .map(|l| format!("{:indent$}{}", "", l))
        .collect();
    indented_lines.join("\n")
}

#[derive(Debug)]
struct TouchContext {
    common_options: CommonOptions,
    options: TouchOptions,
    term: Term,
}

impl TouchContext {
    fn new(common_options: CommonOptions, options: TouchOptions) -> Self {
        let term = Term::stderr();
        Self {
            common_options,
            options,
            term,
        }
    }

    fn touch(&self) -> anyhow::Result<()> {
        self.touch_path(&self.options.path)
    }

    fn touch_path<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        // though this function returns Result<()>, it currently never fail
        let path = path.as_ref();
        log::trace!("processing {path:?}");
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("ignore {path:?}, can not read metadata: {e}");
                return Ok(());
            }
        };
        if metadata.is_symlink() {
            let target = match fs::read_link(path) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("ignore {path:?}, failed to read symbolic link: {e}");
                    return Ok(());
                }
            };
            if validate_store_path(&self.common_options.store, target) {
                // touch
                if self.options.silent {
                    log::debug!("touch {path:?}");
                } else {
                    println!(
                        "{} {path:?}",
                        self.term.style().green().bold().apply_to("Touch")
                    );
                }
                if !self.options.dry_run {
                    let result = utimensat(
                        AT_FDCWD,
                        path,
                        &TimeSpec::UTIME_OMIT,
                        &TimeSpec::UTIME_NOW,
                        UtimensatFlags::NoFollowSymlink,
                    );
                    if let Err(e) = result {
                        log::error!("failed to touch {path:?}: {e}");
                    }
                }
            } else {
                log::debug!("ignore {path:?}, not a link into store");
            }
        }
        if metadata.is_dir() {
            let directory = match fs::read_dir(path) {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("ignore {path:?}, failed to read directory {e}");
                    return Ok(());
                }
            };
            for result in directory {
                let entry = match result {
                    Ok(e) => e,
                    Err(e) => {
                        log::warn!(
                            "ignore a directory entry in {path:?}, failed to read the directory entry: {e}"
                        );
                        return Ok(());
                    }
                };
                self.touch_path(entry.path())?;
            }
        }
        Ok(())
    }
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
