use std::sync::atomic::{AtomicUsize, Ordering};

use dialoguer::console::Term;

use crate::utils::dry_run_indicator;

#[derive(Debug, Default)]
pub struct Statistics {
    pub traversed: Counter,
    pub monitored: Counter,
    pub expired: Counter,
    pub invalid: Counter,
    pub removed: Counter,
}

impl Statistics {
    pub fn format_with_style(self, term: &Term, dry_run: bool) -> String {
        let traversed = self.traversed.done();
        let monitored = self.monitored.done();
        let expired = self.expired.done();
        let removed = self.removed.done();
        let invalid = self.invalid.done();
        let kept = traversed - removed;
        let num_style = |n| term.style().bold().apply_to(n);
        [
            format!("traversed: {}", num_style(traversed)),
            format!("monitored: {}", num_style(monitored)),
            format!("expired:   {}", num_style(expired)),
            format!(
                "removed:   {}{}",
                num_style(removed),
                dry_run_indicator(term, removed != 0 && dry_run)
            ),
            format!("invalid:   {}", num_style(invalid)),
            format!("kept:      {}", num_style(kept)),
        ]
        .join("\n")
    }
}

#[derive(Debug, Default)]
pub struct Counter(AtomicUsize);

impl Counter {
    pub fn increase(&self) {
        self.add(1);
    }

    pub fn add(&self, n: usize) {
        self.0.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    pub fn done(self) -> usize {
        self.0.into_inner()
    }
}
