use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use regex::bytes::Regex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    filter::Filter,
    policy::{profile::ProfilePolicy, temporary::TemporaryRootPolicy},
};

pub trait Validate {
    fn validate(&self) -> anyhow::Result<()>;
}

/// Main angrr settings structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunConfig {
    /// Store path for validation
    ///
    /// Only GC roots pointing to store will be monitored.
    #[serde(default = "default_store_path")]
    pub store: PathBuf,

    /// Only monitors owned symbolic link target of GC roots.
    #[serde(default)]
    pub owned_only: OwnedOnly,

    /// Remove GC root in `directory` instead of the symbolic link target of them.
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OwnedOnly {
    #[default]
    Auto,
    True,
    False,
}

impl OwnedOnly {
    pub fn instantiate(&self, current_uid: u32) -> bool {
        match self {
            OwnedOnly::Auto => {
                let mode = current_uid != 0;
                match mode {
                    true => log::info!("running as non-root user, only monitoring owned GC roots"),
                    false => log::info!("running as root user, monitoring all GC roots"),
                }
                mode
            }
            OwnedOnly::True => true,
            OwnedOnly::False => false,
        }
    }
}

impl Validate for RunConfig {
    fn validate(&self) -> anyhow::Result<()> {
        let mut seen_profiles = HashSet::new();
        for cfg in self.profile_policies.values() {
            if seen_profiles.contains(&cfg.profile_paths) {
                anyhow::bail!(
                    "duplicate profile path in profile policies: {:?}",
                    cfg.profile_paths
                );
            } else {
                seen_profiles.insert(cfg.profile_paths.clone());
            }
        }
        for (name, policy) in &self.temporary_root_policies {
            policy.validate(name)?;
        }
        for (name, policy) in &self.profile_policies {
            policy.validate(name)?;
        }
        Ok(())
    }
}

impl RunConfig {
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
#[serde(rename_all = "kebab-case")]
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
    #[serde(default)]
    pub filter: Option<Filter>,

    /// Path prefixes to ignore
    #[serde(default = "default_temporary_root_ignored_prefixes")]
    pub ignore_prefixes: Vec<PathBuf>,

    /// Path prefixes to ignore under home directory
    #[serde(default = "default_temporary_root_ignored_prefixes_in_home")]
    pub ignore_prefixes_in_home: Vec<PathBuf>,

    /// Retention period
    #[serde(with = "humantime_serde")]
    #[serde(default)]
    pub period: Option<Duration>,
}

impl TemporaryRootConfig {
    fn validate(&self, name: &str) -> anyhow::Result<()> {
        if self.common.enable && self.period.is_none() {
            anyhow::bail!(
                "invalid temporary root policy {name}: period must be set for the temporary root policy",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProfileConfig {
    #[serde(flatten)]
    common: CommonPolicyConfig,

    /// Path to the profile
    ///
    /// When `owned_only = true`, if the option begins with `~`,
    /// it will be expanded to the home directory of the current user.
    ///
    /// When `owned_only = false`, if the options begins with `~`,
    /// it will be expanded to the home of all users discovered respectively.
    pub profile_paths: Vec<PathBuf>,

    /// Retention period
    #[serde(with = "humantime_serde")]
    #[serde(default)]
    pub keep_since: Option<Duration>,

    /// Keep the latest N GC roots in this profile
    #[serde(default)]
    pub keep_latest_n: Option<usize>,

    /// Whether to keep the current activated system generation
    ///
    /// Only useful for system profiles.
    #[serde(default = "default_keep_current_system")]
    pub keep_current_system: bool,

    /// Whether to keep the currently booted generation
    ///
    /// Only useful for system profiles.
    #[serde(default = "default_keep_booted_system")]
    pub keep_booted_system: bool,
}

impl ProfileConfig {
    fn validate(&self, name: &str) -> anyhow::Result<()> {
        if self.common.enable
            && let (None, None) = (self.keep_since, self.keep_latest_n)
        {
            anyhow::bail!(
                "invalid profile policy {name}: at least one of keep-since and keep-latest-n must be set for the profile policy",
            );
        }
        for path in &self.profile_paths {
            if !(path.starts_with("~") || path.is_absolute()) {
                anyhow::bail!(
                    "invalid profile policy {name}: profile path \"{path:?}\" must be absolute or start with `~`",
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommonPolicyConfig {
    /// Enable this policy
    #[serde(default = "default_policy_enable")]
    pub enable: bool,
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

fn default_keep_current_system() -> bool {
    true
}

fn default_keep_booted_system() -> bool {
    true
}

/// Configuration for `touch` command
///
/// Must be part of `Config`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TouchConfig {
    #[serde(default = "default_store_path")]
    pub store: PathBuf,
}

impl Validate for TouchConfig {
    fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub fn load_config<P, C>(path: &Option<P>) -> anyhow::Result<C>
where
    P: AsRef<Path>,
    C: Serialize + DeserializeOwned + Validate + Debug,
{
    let mut figment = Figment::new();
    let mut file_loaded = false;
    if let Some(p) = global_config_file() {
        figment = figment.merge(Toml::file(&p));
        file_loaded = true;
    }
    if let Some(p) = path {
        let p = p.as_ref();
        if !p.exists() {
            anyhow::bail!("configuration file {:?} does not exist", p);
        }
        figment = figment.merge(Toml::file(p));
        file_loaded = true;
    }
    if !file_loaded {
        log::info!("no configuration file found, using empty configuration");
    }
    let config: C = figment.merge(Env::prefixed("ANGRR_")).extract()?;
    config.validate()?;
    Ok(config)
}

pub fn display_config<C: Serialize>(config: &C) -> anyhow::Result<String> {
    toml::to_string_pretty(config).context("failed to serialize config to TOML for display")
}

pub fn global_config_file() -> Option<PathBuf> {
    let path = PathBuf::from("/etc/angrr/config.toml");
    if path.exists() { Some(path) } else { None }
}
