use std::{fs, path::PathBuf, time::Duration};

pub mod profile;
pub mod temporary;

#[derive(Clone, Debug)]
pub struct GcRoot {
    pub path: PathBuf,
    pub path_metadata: fs::Metadata,
    pub link_path: PathBuf,
    pub age: Duration,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub path: PathBuf,
    pub generations: Vec<GcRoot>,
}
