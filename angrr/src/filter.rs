use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub fn run(&self, input: &Input) -> anyhow::Result<bool> {
        let json = serde_json::to_string(input)
            .with_context(|| format!("failed to serialize input for filter: {input:?}"))?;
        log::trace!(
            "starting filter program {:?} with arguments: {:?}",
            self.program,
            self.arguments
        );
        let mut child = Command::new(&self.program)
            .args(&self.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to invoke external filter {:?} with arguments: {:?}",
                    self.program, self.arguments
                )
            })?;
        {
            let mut stdin = child
                .stdin
                .take()
                .with_context(|| "failed to take stdin of external filter")?;
            stdin
                .write_all(json.as_bytes())
                .context("can not write to stdin of external filter")?;
            // stdin drop and closed here
        }
        let output = child
            .wait_with_output()
            .context("failed to wait on external filter")?;
        // log stdout and stderr for debugging
        if !output.stdout.is_empty() {
            log::debug!(
                "external filter stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        if !output.stderr.is_empty() {
            log::warn!(
                "external filter stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output.status.success())
    }
}
