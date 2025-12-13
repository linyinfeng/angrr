use std::{fs, path::PathBuf};

use anyhow::Context;

use crate::{config::ProfileConfig, profile::Profile, utils::format_duration_short};

#[derive(Clone, Debug)]
pub struct ProfilePolicy {
    pub name: String,
    pub config: ProfileConfig,
}

impl ProfilePolicy {
    pub fn new(name: String, config: ProfileConfig) -> Self {
        Self { name, config }
    }
}

impl ProfilePolicy {
    pub fn run(&self, profile: &Profile) -> anyhow::Result<Vec<bool>> {
        let mut keep_generation = vec![false; profile.generations.len()];

        // keep current
        for (i, generation) in profile.generations.iter().enumerate() {
            if generation.root.path.file_name() == Some(profile.current_generation.as_os_str()) {
                log::debug!(
                    "[{}] keep generation {} since it is the current generation",
                    self.name,
                    generation.number,
                );
                keep_generation[i] = true;
                break;
            }
        }

        // keep booted-system
        if self.config.keep_booted_system {
            let booted_system = Self::booted_system()?;
            for (i, generation) in profile.generations.iter().enumerate() {
                if generation.root.store_path == booted_system {
                    log::debug!(
                        "[{}] keep generation {} since it is the booted system",
                        self.name,
                        generation.number,
                    );
                    keep_generation[i] = true;
                    break;
                }
            }
        }

        // keep current-system
        if self.config.keep_current_system {
            let current_system = Self::current_system()?;
            for (i, generation) in profile.generations.iter().enumerate() {
                if generation.root.store_path == current_system {
                    log::debug!(
                        "[{}] keep generation {} since it is the current system",
                        self.name,
                        generation.number,
                    );
                    keep_generation[i] = true;
                    break;
                }
            }
        }

        // keep since
        if let Some(keep_since) = &self.config.keep_since {
            for (i, generation) in profile.generations.iter().enumerate() {
                if &generation.root.age <= keep_since {
                    log::debug!(
                        "[{}] keep generation {} since age {} <= {}",
                        self.name,
                        generation.number,
                        format_duration_short(generation.root.age),
                        format_duration_short(*keep_since),
                    );
                    keep_generation[i] = true;
                }
            }
        }

        // keep num
        if let Some(keep_latest_n) = &self.config.keep_latest_n {
            for i in 0..(*keep_latest_n).min(keep_generation.len()) {
                log::debug!(
                    "[{}] keep generation {} by keep_latest_n",
                    self.name,
                    profile.generations[i].number,
                );
                keep_generation[i] = true;
            }
        }

        Ok(keep_generation)
    }

    fn booted_system() -> anyhow::Result<PathBuf> {
        fs::read_link("/run/booted-system")
            .with_context(|| "failed to read booted system profile link")
    }

    fn current_system() -> anyhow::Result<PathBuf> {
        fs::read_link("/run/current-system")
            .with_context(|| "failed to read current system profile link")
    }
}
