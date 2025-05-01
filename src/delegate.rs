use std::path::PathBuf;
use std::env;
use std::fs;
use std::process::Command;

use anyhow::bail;
use anyhow::Context;
use which::which_global;

struct DelagateContext {
    command: PathBuf,
}

impl DelagateContext {
    const ENV_VAR_NAME: &str = "OP_OFFLINE_COMMAND";

    fn new() -> anyhow::Result<DelagateContext> {
        if let Ok(var) = env::var(Self::ENV_VAR_NAME) {
            let path = fs::canonicalize(var)
                .context(format!("Failed to resolve {}", Self::ENV_VAR_NAME))?;
            return Ok(Self {
                command: path,
            })
        }
        let command = which_global("op")?.canonicalize()?;
        let command = match env::current_exe() {
            Ok(current_path) if current_path == command => {
                bail!(format!("Specify {} to original `op` binary path when using alias mode", Self::ENV_VAR_NAME))
            },
            _ => command,
        };
        Ok(Self { command })
    }

    fn delegate(&self) -> Command {
        Command::new(self.command.clone())
    }
}
