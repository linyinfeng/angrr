use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use dialoguer::console::Term;
use uzers::{get_user_by_uid, os::unix::UserExt};

use crate::gc_root::GcRoot;

pub fn validate_store_path<P1: AsRef<Path>, P2: AsRef<Path>>(
    store: P1,
    target: P2,
) -> Option<PathBuf> {
    let store = store.as_ref();
    let target = target.as_ref();
    match fs::canonicalize(target) {
        Ok(path) => {
            if path.starts_with(store) {
                Some(path)
            } else {
                None
            }
        }
        Err(e) => {
            log::warn!("failed to canonicalize {target:?} for validation: {e}");
            None
        }
    }
}

pub fn discover_users(roots: &[Arc<GcRoot>]) -> anyhow::Result<Vec<uzers::User>> {
    let uids: BTreeSet<_> = roots.iter().map(|root| root.path_metadata.uid()).collect();
    let mut users = Vec::new();
    for uid in uids {
        match get_user_by_uid(uid) {
            Some(user) => users.push(user),
            None => anyhow::bail!("failed to get user by uid {}", uid),
        }
    }
    log::trace!("user discovery result: {users:?}");
    Ok(users)
}

pub fn user_homes(users: &[uzers::User]) -> Vec<PathBuf> {
    users
        .iter()
        .map(|user| user.home_dir().to_path_buf())
        .collect()
}

pub fn current_user_home() -> anyhow::Result<PathBuf> {
    let uid = uzers::get_current_uid();
    match get_user_by_uid(uid) {
        Some(user) => Ok(user.home_dir().to_path_buf()),
        None => anyhow::bail!("failed to get current user by uid {}", uid),
    }
}

pub fn format_duration_short(duration: std::time::Duration) -> String {
    let s = humantime::format_duration(duration).to_string();
    s.split(' ').take(2).collect::<Vec<_>>().join(" ")
}

pub fn dry_run_indicator(term: &Term, show: bool) -> String {
    if show {
        term.style().bold().apply_to(" (dry-run)").to_string()
    } else {
        "".to_string()
    }
}
