use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Result;
use clap::Parser;

use crate::delegate::OpDelegate;
use crate::store::Store;
use crate::template;

const MASK: &str = "<concealed by 1Password>";

#[derive(Debug, Parser)]
#[clap(
    about = "Pass secrets as environment variables to a process",
    long_about = "Pass secrets as environment variables to an application or script.

Scans environment variables for secret references, loads the corresponding
secrets from 1Password, then runs the provided command with the secrets
made available as environment variables.

Precedence order (highest to lowest):
  1. Environment files (--env-file)
  2. Shell environment variables

Secrets printed to stdout and stderr are concealed by default.
Use --no-masking to disable masking.",
    after_help = "Examples:
  Run with environment variable:
    DB_PASSWORD='op://app-prod/db/password' op-offline run -- printenv DB_PASSWORD

  Use an environment file:
    echo 'DB_PASSWORD=op://app-dev/db/password' > .env
    op-offline run --env-file .env -- printenv DB_PASSWORD

  Use variables to switch environments:
    cat .env
    DB_PASSWORD=op://$APP_ENV/db/password

    APP_ENV=prod op-offline run --env-file .env -- printenv DB_PASSWORD

  Show secrets without masking:
    DB_PASSWORD='op://app-prod/db/password' op-offline run --no-masking -- printenv DB_PASSWORD

  Run a subshell to expand variables:
    MY_VAR='op://vault/item/field' op-offline run --no-masking -- sh -c 'echo \"$MY_VAR\"'
"
)]
pub struct RunArgs {
    #[clap(
        long = "env-file",
        value_name = "FILE",
        help = "Environment file to load (can be specified multiple times)"
    )]
    pub env_files: Vec<PathBuf>,

    #[clap(
        long = "no-masking",
        help = "Disable masking of secrets on stdout and stderr"
    )]
    pub no_masking: bool,

    #[clap(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Command to run (use -- to separate flags)"
    )]
    pub command: Vec<String>,
}

pub fn execute(args: RunArgs) -> Result<()> {
    if args.command.is_empty() {
        anyhow::bail!("No command specified");
    }

    let mut env_vars: HashMap<String, String> = HashMap::new();

    for (key, value) in std::env::vars() {
        env_vars.insert(key, value);
    }

    for env_file in &args.env_files {
        if env_file.exists() {
            let contents = std::fs::read_to_string(env_file)?;
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    let value = value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                        .unwrap_or(value);
                    env_vars.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    let mut all_refs: Vec<String> = Vec::new();

    for value in env_vars.values() {
        all_refs.extend(template::extract_references(value));
    }

    for arg in &args.command {
        all_refs.extend(template::extract_references(arg));
    }

    let references: Vec<String> = all_refs
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let store = Store::open();
    let delegate = OpDelegate::new()?;

    let mut stored_values: HashMap<String, String> = HashMap::new();
    let mut uncached_refs: Vec<&str> = Vec::new();

    match &store {
        Ok(store) => {
            for ref_str in &references {
                match store.get(ref_str)? {
                    Some(value) => {
                        stored_values.insert(ref_str.clone(), value);
                    }
                    None => {
                        uncached_refs.push(ref_str);
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Store unavailable: {}", e);
            uncached_refs = references.iter().map(|s| s.as_str()).collect();
        }
    }

    if !uncached_refs.is_empty() {
        log::debug!("Fetching {} uncached references", uncached_refs.len());
        let fetched = delegate.read_batch(&uncached_refs)?;

        if let Ok(store) = &store {
            for (ref_str, value) in &fetched {
                store.put(ref_str, value)?;
            }
        }

        stored_values.extend(fetched);
    }

    let resolver = |ref_str: &str| stored_values.get(ref_str).cloned();

    let mut resolved_env: HashMap<String, String> = HashMap::new();
    for (key, value) in &env_vars {
        let resolved = template::resolve_variables(value);
        let substituted = template::substitute_references(&resolved, resolver);
        resolved_env.insert(key.clone(), substituted);
    }

    let command_args: Vec<String> = args
        .command
        .iter()
        .map(|arg| {
            let resolved = template::resolve_variables(arg);
            template::substitute_references(&resolved, resolver)
        })
        .collect();

    let (program, cmd_args) = command_args
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("Empty command"))?;

    let secrets: Vec<String> = stored_values.into_values().collect();

    if args.no_masking {
        let status = std::process::Command::new(program)
            .args(cmd_args)
            .envs(&resolved_env)
            .status()?;
        std::process::exit(status.code().unwrap_or(1));
    }

    let mut child = std::process::Command::new(program)
        .args(cmd_args)
        .envs(&resolved_env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let secrets = secrets.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                println!("{}", mask_secrets(&line, &secrets));
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("{}", mask_secrets(&line, &secrets));
            }
        });
    }

    let status = child.wait()?;
    std::process::exit(status.code().unwrap_or(1));
}

fn mask_secrets(input: &str, secrets: &[String]) -> String {
    let mut result = input.to_string();
    for secret in secrets {
        if secret.is_empty() {
            continue;
        }
        result = result.replace(secret, MASK);
    }
    result
}
