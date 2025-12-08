use crate::config::ProfileConfig;

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
