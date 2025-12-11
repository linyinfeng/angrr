use anyhow::Context;
use dialoguer::{Confirm, console::Term};
use regex::bytes::Regex;
use std::cmp;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::{Arc, LazyLock};
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
use crate::profile::{Generation, Profile};
use crate::utils::{dry_run_indicator, format_duration_short};
use crate::{
    command::RunOptions, config::RunConfig, current::Current, gc_root::GcRoot,
    policy::temporary::TemporaryRootPolicy, statistics::Statistics, utils::validate_store_path,
};

#[derive(Debug)]
pub struct RunContext {
    options: RunOptions,
    config: RunConfig,

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

static PROFILE_GENERATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*)-([0-9]+)-link$").unwrap());

impl RunContext {
    pub fn new(options: RunOptions, config: RunConfig) -> anyhow::Result<Self> {
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
        let gc_roots = self.gc_roots()?;
        let mut waiting = Vec::new();
        self.run_temporary_root_policies(&gc_roots, &mut waiting)?;
        self.run_profile_policies(&gc_roots, &mut waiting)?;
        if !waiting.is_empty() && self.prompt_continue()? {
            for (policy_name, root) in &waiting {
                self.remove(policy_name, root)?;
            }
        }
        Ok(())
    }

    fn gc_roots(&self) -> anyhow::Result<Vec<Arc<GcRoot>>> {
        let mut result = Vec::new();
        for path in &self.config.directory {
            let directory =
                fs::read_dir(path).with_context(|| format!("failed to open directory {path:?}"))?;
            for entry in directory {
                let entry = entry.with_context(|| {
                    format!("failed to read directory entry from directory {path:?}")
                })?;
                self.statistic.traversed.increase();
                let link_path = entry.path();
                if let Some(root) = self.filter_and_map_to_gc_root(&link_path)? {
                    result.push(Arc::new(root));
                }
            }
        }
        Ok(result)
    }
    fn run_temporary_root_policies(
        &self,
        gc_roots: &[Arc<GcRoot>],
        waiting: &mut Vec<(String, Arc<GcRoot>)>,
    ) -> anyhow::Result<()> {
        let policies = &self.temporary_root_policies;

        for gc_root in gc_roots {
            let mut matched = None;
            for (name, policy) in policies {
                if policy.monitored(gc_root)? {
                    matched = Some((name, policy));
                    break;
                }
            }
            let (policy_name, policy) = match matched {
                Some(m) => m,
                None => {
                    log::trace!("keep {:?}, no matching temporary root policy", gc_root.path);
                    continue;
                }
            };

            self.statistic.monitored.increase();

            if policy.expired(gc_root) {
                self.statistic.expired.increase();
                match self.options.interactive {
                    Interactive::Always => {
                        self.notify_action(policy_name, gc_root, Action::AboutToRemove)?;
                        let yes = self.prompt_continue()?;
                        if yes {
                            self.remove(policy_name, gc_root)?;
                        } else {
                            self.notify_action(policy_name, gc_root, Action::Ignored)?;
                        }
                    }
                    Interactive::Once => {
                        self.notify_action(policy_name, gc_root, Action::AboutToRemove)?;
                        waiting.push((policy_name.clone(), gc_root.clone()));
                    }
                    Interactive::Never => {
                        self.remove(policy_name, gc_root)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn run_profile_policies(
        &self,
        gc_roots: &[Arc<GcRoot>],
        waiting: &mut Vec<(String, Arc<GcRoot>)>,
    ) -> anyhow::Result<()> {
        let gc_roots_lookup_map: BTreeMap<&PathBuf, Arc<GcRoot>> = gc_roots
            .iter()
            .map(|root| (&root.path, root.clone()))
            .collect();
        for (policy_name, profile_policy) in &self.profile_policies {
            let path = &profile_policy.config.profile_path;
            let profile = self
                .filter_and_read_profile(path, &gc_roots_lookup_map)
                .with_context(|| format!("failed to read profile {path:?}"))?;
            let profile = match profile {
                Some(p) => p,
                None => break,
            };
            self.statistic.monitored.add(profile.generations.len());
            let generations = profile_policy.run(&profile)?;
            self.statistic.expired.add(generations.len());
            match self.options.interactive {
                Interactive::Always => {
                    for g in &generations {
                        self.notify_action(policy_name, &g.root, Action::AboutToRemove)?;
                    }
                    let yes = self.prompt_continue()?;
                    for g in &generations {
                        if yes {
                            self.remove(policy_name, &g.root)?;
                        } else {
                            self.notify_action(policy_name, &g.root, Action::Ignored)?;
                        }
                    }
                }
                Interactive::Once => {
                    for g in &generations {
                        self.notify_action(policy_name, &g.root, Action::AboutToRemove)?;
                        waiting.push((policy_name.clone(), g.root.clone()));
                    }
                }
                Interactive::Never => {
                    for g in &generations {
                        self.remove(policy_name, &g.root)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn filter_and_map_to_gc_root<P: AsRef<Path>>(
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
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && self.config.owned_only => {
                log::debug!(
                    "ignore profile {target:?} in owned only mode, not owned by the current user"
                );
                return Ok(None);
            }
            Err(e) => {
                log::warn!("ignore {target:?}, can not read metadata: {e}");
                return Ok(None);
            }
        };

        // filter with owned_only
        if self.config.owned_only {
            let file_uid = metadata.uid();
            if file_uid != self.uid {
                log::debug!(
                    "ignore profile {target:?} in owned only mode, not owned by the current user"
                );
                return Ok(None);
            }
        }

        // filter with store path validation
        let store_path = match validate_store_path(&self.config.store, &target) {
            Some(store_path) => store_path,
            None => {
                log::warn!("ignore {target:?}, not a link into store");
                return Ok(None);
            }
        };

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
            store_path,
            path_metadata: metadata,
            age: elapsed,
        }))
    }

    fn filter_and_read_profile<P>(
        &self,
        path: P,
        gc_roots_lookup_map: &BTreeMap<&PathBuf, Arc<GcRoot>>,
    ) -> anyhow::Result<Option<Profile>>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                log::info!("ignore profile {path:?}, path not found");
                return Ok(None);
            }
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && self.config.owned_only => {
                log::info!(
                    "ignore profile {path:?} in owned only mode, not owned by the current user"
                );
                return Ok(None);
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("failed to read metadata of profile path {path:?}"));
            }
        };

        // filter with owned_only
        if self.config.owned_only {
            let file_uid = metadata.uid();
            if file_uid != self.uid {
                log::info!(
                    "ignore profile {path:?} in owned only mode, not owned by the current user",
                );
                return Ok(None);
            }
        }

        let profile_name = path
            .file_name()
            .with_context(|| format!("failed to get file name of profile path {path:?}"))?;
        let current_generation = fs::read_link(path)
            .with_context(|| format!("failed to read symbolic link {path:?}"))?;
        let mut generations = Vec::new();
        let directory = path
            .parent()
            .with_context(|| format!("profile {path:?} has no parent directory"))?;
        for entry in fs::read_dir(directory)
            .with_context(|| format!("failed to read directory {directory:?}"))?
        {
            let entry = entry.with_context(|| {
                format!("failed to read directory entry from directory {directory:?}")
            })?;
            let path = entry.path();
            let gc_root = match gc_roots_lookup_map.get(&path) {
                Some(root) => root.clone(),
                None => continue,
            };
            let filename = entry.file_name();
            if let Some(captures) = PROFILE_GENERATION_REGEX.captures(filename.as_bytes()) {
                let name_bytes = &captures[1];
                let number_bytes = &captures[2];
                if name_bytes != profile_name.as_bytes() {
                    continue;
                }
                let number: usize =
                    String::from_utf8_lossy(number_bytes)
                        .parse()
                        .with_context(|| {
                            format!("failed to parse generation number in {:?}", entry.path())
                        })?;
                let generation = Generation {
                    number,
                    root: gc_root,
                };
                generations.push(generation);
            }
        }
        generations.sort_by_key(|g| cmp::Reverse(g.number));
        Ok(Some(Profile {
            path: path.to_path_buf(),
            path_metadata: metadata,
            current_generation,
            generations,
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
