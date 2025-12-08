use anyhow::Context;
use dialoguer::{Confirm, console::Term};
use std::fmt::Debug;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    io::{self, BufWriter, Write, sink, stdout},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use crate::command::Interactive;
use crate::policy::profile::ProfilePolicy;
use crate::utils::{dry_run_indicator, format_duration_short};
use crate::{
    command::RunOptions,
    config::Config,
    current::Current,
    policy::{GcRoot, temporary::TemporaryRootPolicy},
    statistics::Statistics,
    utils::validate_store_path,
};

#[derive(Debug)]
pub struct RunContext {
    options: RunOptions,
    config: Config,

    /// Order of temporary root policies are meaningful
    temporary_root_policies: Vec<(String, TemporaryRootPolicy)>,
    profile_policies: HashMap<String, ProfilePolicy>,
    policy_name_max_len: usize,

    current: Current,
    uid: u32,
    term: Term,
    output: Mutex<Output>,
    statistic: Statistics,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Action {
    Remove,
    AboutToRemove,
    Ignored,
}

#[derive(Debug)]
pub struct Output {
    writer: Box<dyn OutputWriter>,
    first_output: bool,
}

trait OutputWriter: Write + Debug {}
impl<T> OutputWriter for T where T: Write + Debug {}

impl RunContext {
    pub fn new(options: RunOptions, config: Config) -> anyhow::Result<Self> {
        let temporary_root_policies = config.enabled_temporary_root_policies();
        let profile_policies = config.enabled_profile_policies();
        let current = Current::new();
        let uid = uzers::get_current_uid();
        let term = Term::stderr();
        let output = Mutex::new(Output {
            writer: Self::output_writer(&options)?,
            first_output: true,
        });
        let statistic = Default::default();

        let policy_name_max_len = temporary_root_policies
            .iter()
            .map(|(name, _)| name.len())
            .chain(profile_policies.keys().map(|name| name.len()))
            .max()
            .unwrap_or(0);

        let context = Self {
            options,
            config,
            temporary_root_policies,
            profile_policies,
            policy_name_max_len,
            current,
            uid,
            term,
            output,
            statistic,
        };
        log::debug!("options: {:#?}", context.options);
        Ok(context)
    }

    pub fn run(&self) -> anyhow::Result<()> {
        self.run_temporary_root_policies()?;
        self.run_profile_policies()?;
        Ok(())
    }

    pub fn finish(mut self) -> anyhow::Result<()> {
        if !self.options.no_statistic {
            writeln!(
                self.term,
                "{}",
                self.term.style().bold().underlined().apply_to("Statistics")
            )?;
            self.term.write_line(
                &self
                    .statistic
                    .format_with_style(&self.term, self.options.dry_run),
            )?;
        }
        let mut output = self.output.lock().unwrap();
        output.writer.flush().context("failed to flush output")?;
        Ok(())
    }

    fn run_temporary_root_policies(&self) -> anyhow::Result<()> {
        let policies = &self.temporary_root_policies;
        let mut waiting = Vec::new();

        for path in &self.config.directory {
            let directory = fs::read_dir(path).with_context(|| {
                format!(
                    "failed to open directory
        {path:?}"
                )
            })?;
            for entry in directory {
                self.statistic.traversed.increase();
                let link = entry.with_context(|| {
                    format!("failed to read directory entry from directory {path:?}")
                })?;
                let link_path = link.path();
                match self.filter_and_map_to_status(&link_path)? {
                    Some(gc_root) => {
                        let mut matched = None;
                        for (name, policy) in policies {
                            if policy.monitored(&gc_root)? {
                                matched = Some((name, policy));
                                break;
                            }
                        }
                        let (policy_name, policy) = match matched {
                            Some(m) => m,
                            None => {
                                log::trace!(
                                    "keep {:?}, no matching temporary root policy",
                                    link_path
                                );
                                continue;
                            }
                        };

                        self.statistic.monitored.increase();

                        if policy.expired(&gc_root) {
                            self.statistic.expired.increase();
                            match self.options.interactive {
                                Interactive::Always => {
                                    self.notify_action(
                                        policy_name,
                                        &gc_root,
                                        Action::AboutToRemove,
                                    )?;
                                    let yes = self.prompt_continue()?;
                                    if yes {
                                        self.remove(policy_name, &gc_root)?;
                                    } else {
                                        self.notify_action(policy_name, &gc_root, Action::Ignored)?;
                                    }
                                }
                                Interactive::Once => {
                                    self.notify_action(
                                        policy_name,
                                        &gc_root,
                                        Action::AboutToRemove,
                                    )?;
                                    waiting.push((policy_name.clone(), gc_root));
                                }
                                Interactive::Never => {
                                    self.remove(policy_name, &gc_root)?;
                                }
                            }
                        }
                    }
                    None => log::trace!("keep {link_path:?}"),
                }
            }
        }

        if !waiting.is_empty() && self.prompt_continue()? {
            for (policy_name, root) in &waiting {
                self.remove(policy_name, root)?;
            }
        }

        Ok(())
    }

    fn run_profile_policies(&self) -> anyhow::Result<()> {
        // TODO
        if self.profile_policies.is_empty() {
            return Ok(());
        }
        todo!()
    }

    fn filter_and_map_to_status<P: AsRef<Path>>(
        &self,
        link_path: P,
    ) -> anyhow::Result<Option<GcRoot>> {
        let link_path = link_path.as_ref();
        let target = fs::read_link(link_path)
            .with_context(|| format!("failed to read symbolic link {link_path:?}"))?;
        log::trace!("filtering {link_path:?} -> {target:?}");
        let metadata = match fs::symlink_metadata(&target) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                log::debug!("target of {link_path:?} not found, skip");
                return Ok(None);
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::PermissionDenied && self.config.owned_only {
                    log::debug!(
                        "ignore {target:?} due to permission denied and in owned only mode"
                    );
                } else {
                    log::warn!("ignore {target:?}, can not read metadata: {e}");
                }
                return Ok(None);
            }
        };

        // filter with owned_only
        if self.config.owned_only {
            let file_uid = metadata.uid();
            if file_uid != self.uid {
                log::debug!(
                    "ignore {target:?} due to uid mismatch: file uid == {file_uid}, process uid == {process_uid}",
                    process_uid = self.uid
                );
                return Ok(None);
            }
        }

        // filter with store path validation
        if !validate_store_path(&self.config.store, &target) {
            log::warn!("ignore {target:?}, not a link into store");
            return Ok(None);
        }

        let target_mtime = metadata
            .modified()
            .with_context(|| format!("failed to get modified time of file {target:?}"))?;
        let elapsed = self
            .current
            .now
            .duration_since(target_mtime)
            .unwrap_or_else(|_| Duration::new(0, 0));

        Ok(Some(GcRoot {
            path: target,
            link_path: link_path.to_path_buf(),
            path_metadata: metadata,
            age: elapsed,
        }))
    }

    fn prompt_continue(&self) -> anyhow::Result<bool> {
        Confirm::new()
            .with_prompt("Do you want to continue?")
            .report(false)
            .default(false)
            .interact_on(&self.term)
            .context("failed to prompt")
    }

    fn notify_action(
        &self,
        policy_name: &str,
        item: &GcRoot,
        action: Action,
    ) -> anyhow::Result<()> {
        let mut term = &self.term;
        let path = self.path_to_remove(item);
        writeln!(
            term,
            "[{policy_name:name_width$}] {action}{dry_run_indicator} {path:?} ({age} ago)",
            dry_run_indicator =
                dry_run_indicator(term, action == Action::Remove && self.options.dry_run),
            action = action.format_with_style(term),
            age = term
                .style()
                .bold()
                .apply_to(format_duration_short(item.age)),
            name_width = self.policy_name_max_len,
        )?;
        Ok(())
    }

    fn remove(&self, policy_name: &str, root: &GcRoot) -> anyhow::Result<()> {
        self.notify_action(policy_name, root, Action::Remove)?;
        let path_to_remove = if self.config.remove_root {
            &root.link_path
        } else {
            &root.path
        };
        if !self.options.dry_run {
            fs::remove_file(path_to_remove)
                .with_context(|| format!("failed to remove {path_to_remove:?}"))?;
        }
        self.statistic.removed.increase();
        let mut out = self.output.lock().unwrap();
        out.output(path_to_remove, &self.options.output_delimiter)?;
        Ok(())
    }

    fn path_to_remove<'a>(&self, item: &'a GcRoot) -> &'a PathBuf {
        if self.config.remove_root {
            &item.link_path
        } else {
            &item.path
        }
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
