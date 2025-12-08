use std::time::SystemTime;

/// Current system state useful in policy decisions
///
/// Currently this struct only contains the current system time.
#[derive(Clone, Debug)]
pub struct Current {
    pub now: SystemTime,
}

impl Default for Current {
    fn default() -> Self {
        Self::new()
    }
}

impl Current {
    pub fn new() -> Self {
        Self {
            now: SystemTime::now(),
        }
    }
}
