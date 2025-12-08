use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Duration,
};

use regex::bytes::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    filter::Filter,
    policy::{profile::ProfilePolicy, temporary::TemporaryRootPolicy},
};

/// Main angrr settings structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Default log level, can be overridden by command line options or
    /// environment variables
    #[serde(default = "default_log_filter")]
    pub log_level: log::LevelFilter,

    /// Store path for validation
    ///
    /// Only GC roots pointing to store will be deleted unless `force` is true.
    #[serde(default = "default_store_path")]
    pub store: PathBuf,

    /// Force delete targets of GC roots that do not point to store.
    ///
    /// Validation only happens when `remove_root` is not true.
    #[serde(default)]
    pub force: bool,

    /// Only monitors owned symbolic link target of GC roots.
    #[serde(default = "normal_user")]
    pub owned_only: bool,

    /// Remove GC root in `directory` instead of the symbolic link target of the
    /// root.
    #[serde(default)]
    pub remove_root: bool,

    /// Directories containing auto GC roots
    #[serde(default = "default_auto_gcroots_dirs")]
    pub directory: Vec<PathBuf>,

    #[serde(default)]
    pub temporary_root_policies: HashMap<String, TemporaryRootConfig>,
    #[serde(default)]
    pub profile_policies: HashMap<String, ProfileConfig>,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        let mut seen_profiles = HashSet::new();
        for cfg in self.profile_policies.values() {
            if seen_profiles.contains(&cfg.profile_path) {
                anyhow::bail!(
                    "duplicate profile path in profile policies: {:?}",
                    cfg.profile_path
                );
            } else {
                seen_profiles.insert(cfg.profile_path.clone());
            }
        }
        Ok(())
    }

    pub fn enabled_temporary_root_policies(&self) -> Vec<(String, TemporaryRootPolicy)> {
        let mut result: Vec<_> = self
            .temporary_root_policies
            .iter()
            .filter(|(_, cfg)| cfg.common.enable)
            .map(|(name, cfg)| {
                (
                    name.clone(),
                    TemporaryRootPolicy::new(name.clone(), cfg.clone()),
                )
            })
            .collect();
        result.sort_by_cached_key(|(name, policy)| (policy.config.priority, name.clone()));
        result
    }

    pub fn enabled_profile_policies(&self) -> HashMap<String, ProfilePolicy> {
        self.profile_policies
            .iter()
            .filter(|(_, cfg)| cfg.common.enable)
            .map(|(name, cfg)| (name.clone(), ProfilePolicy::new(name.clone(), cfg.clone())))
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporaryRootConfig {
    #[serde(flatten)]
    common: CommonPolicyConfig,

    /// Priority
    ///
    /// Lower number means higher priority, if multiple policies monitor the
    /// same path, the one with higher priority will be applied.
    /// If multiple policies have the same priority, name in lexicographical
    /// order will be applied. That is, a policy named "A" with priority 100
    /// will have higher priority than a policy named "B" with priority 100.
    #[serde(default = "default_temporary_policy_priority_default")]
    pub priority: usize,

    /// Only paths (absolute) matching the regex will be monitored
    #[serde(with = "serde_regex")]
    pub path_regex: Regex,

    /// An external program to filter paths that will be applied after all the
    /// other filter options
    ///
    /// A JSON object containing the path information will be passed to the
    /// stdin of the program. If the program exits with code 0, then the
    /// path will be monitored; otherwise it will be ignored.
    pub filter: Option<Filter>,

    /// Path prefixes to ignore
    #[serde(default = "default_temporary_root_ignored_prefixes")]
    pub ignore_prefixes: Vec<PathBuf>,

    /// Path prefixes to ignore under home directory
    #[serde(default = "default_temporary_root_ignored_prefixes_in_home")]
    pub ignore_prefixes_in_home: Vec<PathBuf>,

    /// Retention period
    #[serde(with = "humantime_serde")]
    pub period: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(flatten)]
    common: CommonPolicyConfig,

    /// Path to the profile
    pub profile_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommonPolicyConfig {
    /// Enable this policy
    #[serde(default = "default_policy_enable")]
    pub enable: bool,
}

fn default_log_filter() -> log::LevelFilter {
    log::LevelFilter::Info
}

fn default_store_path() -> PathBuf {
    PathBuf::from("/nix/store")
}

fn default_auto_gcroots_dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("/nix/var/nix/gcroots/auto")]
}

fn default_temporary_root_ignored_prefixes() -> Vec<PathBuf> {
    // This ignores both /nix/var/nix/profiles/system and
    // /nix/var/nix/profiles/per-user/root
    vec![PathBuf::from("/nix/var/nix/profiles")]
}

fn default_temporary_root_ignored_prefixes_in_home() -> Vec<PathBuf> {
    vec![
        PathBuf::from(".local/state/nix/profiles"),
        PathBuf::from(".local/state/home-manager/gcroots"),
        PathBuf::from(".cache/nix/flake-registry.json"),
    ]
}

fn default_policy_enable() -> bool {
    true
}

fn default_temporary_policy_priority_default() -> usize {
    100
}

/// Returns true if the current user is not root.
fn normal_user() -> bool {
    let uid = uzers::get_current_uid();
    uid != 0
}
