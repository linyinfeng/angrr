use std::{fs, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct GcRoot {
    pub path: PathBuf,
    pub path_metadata: fs::Metadata,
    pub link_path: PathBuf,
    pub store_path: PathBuf,
    pub age: Duration,
}
