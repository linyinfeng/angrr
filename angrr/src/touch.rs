use std::{fs, path::Path};

use dialoguer::console::Term;
use ignore::WalkBuilder;
use ignore::WalkParallel;
use ignore::WalkState;
use nix::fcntl::AT_FDCWD;
use nix::sys::stat::UtimensatFlags;
use nix::sys::stat::utimensat;
use nix::sys::time::TimeSpec;

use crate::config::Config;
use crate::config::globs_to_override;
use crate::{command::TouchOptions, utils::validate_store_path};

#[derive(Debug)]
pub struct TouchContext {
    options: TouchOptions,
    config: Config,
    term: Term,
}

impl TouchContext {
    pub fn new(options: TouchOptions, config: Config) -> Self {
        let term = Term::stderr();
        Self {
            options,
            config,
            term,
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        self.touch()?;
        let duration = start.elapsed();
        if self.options.output_runtime {
            let runtime_secs = duration.as_secs_f64();
            println!("{}", runtime_secs);
        }
        Ok(())
    }

    pub fn touch(&self) -> anyhow::Result<()> {
        let walk = self.walk()?;
        walk.run(|| {
            Box::new(|result| {
                match result {
                    Ok(entry) => {
                        let path = entry.path();
                        self.touch_path(path);
                        // always continue
                        WalkState::Continue
                    }
                    Err(e) => {
                        log::warn!("failed to read directory entry: {e}, skip");
                        WalkState::Skip
                    }
                }
            })
        });
        Ok(())
    }

    pub fn walk(&self) -> anyhow::Result<WalkParallel> {
        let mut builder = WalkBuilder::new(&self.options.path);
        builder.standard_filters(false).follow_links(false);
        builder.max_depth(self.options.max_depth);
        if self.options.no_recurse {
            builder.max_depth(Some(1));
        }
        if self.options.project {
            let over = globs_to_override(&self.options.path, &self.config.touch.project_globs)?;
            builder.overrides(over);
        }
        Ok(builder.build_parallel())
    }

    pub fn touch_path<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref();
        log::trace!("processing {path:?}");
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("ignore {path:?}, can not read metadata: {e}");
                return;
            }
        };
        if metadata.is_symlink() {
            if let Some(_store_path) = validate_store_path(&self.config.store, path) {
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
    }
}
