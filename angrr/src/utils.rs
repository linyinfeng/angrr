use std::{fs, path::Path};

use dialoguer::console::Term;

pub fn validate_store_path<P1: AsRef<Path>, P2: AsRef<Path>>(store: P1, target: P2) -> bool {
    let store = store.as_ref();
    let target = target.as_ref();
    match fs::canonicalize(target) {
        Ok(path) => path.starts_with(store),
        Err(e) => {
            log::warn!("failed to canonicalize {target:?} for validation: {e}");
            false
        }
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
