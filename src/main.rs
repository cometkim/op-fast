use std::env;
use std::ffi::OsString;
use std::fs::canonicalize;
use std::hash::BuildHasher;
use anyhow::Context;
use foldhash::fast::FixedState;
use keyring::Entry;
use log::debug;
use which::which;
use clap::{Parser, Subcommand, ValueHint};

use self::command::Command;

mod command;
mod delegate;
mod vault;

/// `op-offline` CLI
#[derive(Parser)]
#[clap(name = "op-offline", version, about = "Offline version of 1Password CLI")]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

fn key_from_input(input: &str) -> String {
    let state = FixedState::default();
    format!("{:x}", state.hash_one(input))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let op_path =
        which("op").and_then(|path| {
            let op_path = canonicalize(path)?;
            let current_path = env::current_exe()?;
            if current_path == op_path {
            }
        })
        .map_err(|_| which("/opt/homebrew/bin/op"))
        .expect("Cannot find `op` command in current shell. Please set `OP_OFFLINE_COMMAND` to absolute path of the binary.");

    let service_name = "1password-offline";

    debug!("Use 1Password Command: {}", op_path.display());

    let cli = Cli::parse();
    match cli.command {
        Command::Read(args) => {
            debug!("{}", args.reference);

            let entry = Entry::new(
                service_name,
                key_from_input(&args.reference).as_str(),
            )?;
            let secret = match entry.get_password() {
                Ok(secret) => {
                    debug!("Found secret from keychain");
                    secret
                },
                Err(keyring::Error::NoEntry) => {
                    let delegate_args: Vec<OsString> = env::args_os().skip(1).collect();
                    let output = Command::new(&op_path)
                        .args(&delegate_args)
                        .output()
                        .context("`op read` 실행 실패")?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        anyhow::bail!("op read 에러: {}", stderr);
                    }

                    let secret = String::from_utf8(output.stdout)?
                        .trim()
                        .to_string();

                    entry.set_password(secret.as_str())?;

                    secret
                },
                Err(err) => anyhow::bail!(err),
            };

            println!("{}", secret);
        }
    }

    Ok(())
}
