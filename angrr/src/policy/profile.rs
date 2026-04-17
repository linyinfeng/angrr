use std::collections::BTreeSet;
use std::{fs, path::PathBuf};

use crate::{
    config::KeepNPerBucket, config::ProfileConfig, profile::Generation, profile::Profile,
    utils::format_duration_short,
};
use anyhow::Context;

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

        // Retain `n` generations every `bucket-window` duration for `bucket-amount` buckets.
        let sorted_generations: Vec<(usize, &Generation)> = {
            let mut vec = profile.generations.iter().enumerate().collect::<Vec<_>>();
            vec.sort_by_key(|(_idx, generation)| generation.root.age);
            vec
        };
        // Keep track of what was processed and skip them.
        let mut processed: BTreeSet<usize> = BTreeSet::new();
        for &KeepNPerBucket {
            n,
            bucket_window,
            bucket_amount,
        } in &self.config.keep_n_per_bucket
        {
            for i in 0..bucket_amount {
                let mut processed_curr: BTreeSet<usize> = BTreeSet::new();
                sorted_generations.iter().filter(|(_, generation)| {
                        let within_window = bucket_window * i <= generation.root.age
                            && generation.root.age < bucket_window * (i + 1);
                        let not_processed = !processed.contains(&generation.number);
                        within_window && not_processed
                    })
                    .take(n)
                    .for_each(|(gen_index, generation)| {
                        processed_curr.insert(generation.number);
                        keep_generation[*gen_index] = true;
                        log::debug!(
                            "[{}] keep generation {} by keep_n_per_bucket, namely {} generation each bucket spanning {} for {} buckets",
                            self.name,
                            &generation.number,
                            n,
                            format_duration_short(bucket_window),
                            bucket_amount,
                        );
                });

                processed.extend(processed_curr);
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
