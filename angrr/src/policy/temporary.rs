use anyhow::Context;
use std::os::unix::ffi::OsStrExt;
use std::{fs, os::unix::fs::MetadataExt, path::Path};
use uzers::{get_user_by_uid, os::unix::UserExt};

use crate::{config::TemporaryRootConfig, filter, policy::GcRoot};

#[derive(Clone, Debug)]
pub struct TemporaryRootPolicy {
    pub name: String,
    pub config: TemporaryRootConfig,
}

impl TemporaryRootPolicy {
    pub fn new(name: String, config: TemporaryRootConfig) -> Self {
        Self { name, config }
    }
}

impl TemporaryRootPolicy {
    pub fn monitored(&self, root: &GcRoot) -> anyhow::Result<bool> {
        if self.ignored_by_prefix(&root.path)
            || self.ignored_by_prefix_in_home(&root.path, &root.path_metadata)
        {
            log::debug!(
                "[{}] ignore {:?}, path in ignore prefixes",
                self.name,
                root.path,
            );
            return Ok(false);
        }

        if !self
            .config
            .path_regex
            .is_match(root.path.as_os_str().as_bytes())
        {
            log::debug!(
                "[{}] ignore {:?}, path does not match regex {:?}",
                self.name,
                root.path,
                self.config.path_regex,
            );
            return Ok(false);
        }

        if let Some(filter) = &self.config.filter {
            let input = filter::Input {
                path: root.path.clone(),
                gc_root: root.link_path.clone(),
            };
            let not_ignored = filter
                .run(&input)
                .with_context(|| format!("failed to run filter on input: {input:?}"))?;
            if !not_ignored {
                log::debug!(
                    "[{}] ignore {:?}, filtered out by external filter",
                    self.name,
                    root.path,
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn expired(&self, root: &GcRoot) -> bool {
        root.age > self.config.period
    }
}

impl TemporaryRootPolicy {
    fn ignored_by_prefix<P: AsRef<Path>>(&self, target: P) -> bool {
        let p = target.as_ref();
        for prefix in &self.config.ignore_prefixes {
            if p.starts_with(prefix) {
                return true;
            }
        }
        false
    }

    fn ignored_by_prefix_in_home<P: AsRef<Path>>(
        &self,
        target: P,
        metadata: &fs::Metadata,
    ) -> bool {
        let p = target.as_ref();
        let uid = metadata.uid();
        let user = match get_user_by_uid(uid) {
            None => return false,
            Some(user) => user,
        };
        let home = user.home_dir();
        for prefix in &self.config.ignore_prefixes_in_home {
            let full_prefix = home.join(prefix);
            if p.starts_with(full_prefix) {
                return true;
            }
        }
        false
    }
}
