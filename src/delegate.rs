use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use which::which_global;

pub struct OpDelegate {
    command: PathBuf,
}

impl OpDelegate {
    const ENV_VAR_NAME: &'static str = "OP_FAST_COMMAND";

    pub fn new() -> Result<Self> {
        if let Ok(var) = env::var(Self::ENV_VAR_NAME) {
            let path = fs::canonicalize(&var)
                .with_context(|| format!("Failed to resolve {}: {}", Self::ENV_VAR_NAME, var))?;
            return Ok(Self { command: path });
        }

        let command = which_global("op")
            .context("Could not find `op` command in PATH")?
            .canonicalize()
            .context("Failed to canonicalize `op` path")?;

        let command = match env::current_exe() {
            Ok(current_path) if current_path == command => {
                bail!(
                    "Detected self-reference. Set {} to the original `op` binary path",
                    Self::ENV_VAR_NAME
                )
            }
            _ => command,
        };

        Ok(Self { command })
    }

    pub fn read(&self, reference: &str) -> Result<String> {
        let output = Command::new(&self.command)
            .arg("read")
            .arg(reference)
            .output()
            .context("Failed to execute `op read`")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("op read failed: {}", stderr);
        }

        let value = String::from_utf8(output.stdout)
            .context("op read returned non-UTF8 output")?
            .trim_end()
            .to_string();

        Ok(value)
    }

    pub fn read_batch(&self, references: &[&str]) -> Result<HashMap<String, String>> {
        if references.is_empty() {
            return Ok(HashMap::new());
        }

        let template: String = references
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let mut child = Command::new(&self.command)
            .arg("inject")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn `op inject`")?;

        let stdin = child.stdin.as_mut().context("Failed to open stdin")?;
        stdin
            .write_all(template.as_bytes())
            .context("Failed to write to op inject stdin")?;
        let _ = stdin;

        let output = child
            .wait_with_output()
            .context("Failed to wait for op inject")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("op inject failed: {}", stderr);
        }

        let stdout =
            String::from_utf8(output.stdout).context("op inject returned non-UTF8 output")?;

        let values: Vec<&str> = stdout.lines().collect();

        if values.len() != references.len() {
            bail!(
                "op inject returned {} values for {} references",
                values.len(),
                references.len()
            );
        }

        let mut result = HashMap::new();
        for (reference, value) in references.iter().zip(values.iter()) {
            result.insert(reference.to_string(), value.to_string());
        }

        Ok(result)
    }

    pub fn command(&self) -> Command {
        Command::new(&self.command)
    }
}
