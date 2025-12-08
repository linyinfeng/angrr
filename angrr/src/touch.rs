use std::{fs, path::Path};

use dialoguer::console::Term;
use nix::fcntl::AT_FDCWD;
use nix::sys::stat::UtimensatFlags;
use nix::sys::stat::utimensat;
use nix::sys::time::TimeSpec;

use crate::{command::TouchOptions, config::Config, utils::validate_store_path};

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

    pub fn touch(&self) -> anyhow::Result<()> {
        self.touch_path(&self.options.path)
    }

    pub fn touch_path<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
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
            if validate_store_path(&self.config.store, target) {
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
