use std::path::PathBuf;

use anyhow::Context;
use log::Level;
use serde::{Deserialize, Serialize};
use subprocess::Exec;

use crate::command::RunOptions;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Filter {
    pub program: PathBuf,
    #[serde(default)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Input {
    pub path: PathBuf,
    pub gc_root: PathBuf,
}

impl Filter {
    pub fn run(&self, options: &RunOptions, input: &Input) -> anyhow::Result<bool> {
        let json = serde_json::to_string(input)
            .with_context(|| format!("failed to serialize input for filter: {input:?}"))?;
        log::trace!(
            "starting filter program {:?} with arguments: {:?}",
            self.program,
            self.arguments
        );
        let job = Exec::cmd(&self.program)
            .args(&self.arguments)
            .stdin(json.into_bytes())
            .start()
            .with_context(|| {
                format!(
                    "failed to invoke external filter {:?} with arguments: {:?}",
                    self.program, self.arguments
                )
            })?;
        let capture = job.capture_timeout(options.filter_timeout.into())?;
        if !capture.stdout.is_empty() {
            log::debug!("external filter stdout: {}", capture.stdout_str());
        }
        if !capture.stderr.is_empty() {
            log::warn!("external filter stderr: {}", capture.stderr_str());
        }
        log::log!(
            if capture.exit_status.success() {
                Level::Trace
            } else {
                Level::Debug
            },
            "external filter exit with status: {}",
            capture.exit_status
        );
        Ok(capture.exit_status.success())
    }
}
