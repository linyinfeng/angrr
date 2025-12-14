use crate::gc_root::GcRoot;
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Clone, Debug)]
pub struct Profile {
    pub path: PathBuf,
    pub path_metadata: fs::Metadata,
    pub current_generation: PathBuf,
    /// All generations, sorted by number (newest first)
    pub generations: Vec<Generation>,
}

#[derive(Clone, Debug)]
pub struct Generation {
    pub number: usize,
    pub root: Arc<GcRoot>,
}
