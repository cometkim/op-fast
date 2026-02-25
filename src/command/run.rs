use std::collections::HashMap;

use anyhow::Result;
use clap::Parser;

use crate::delegate::OpDelegate;
use crate::store::Store;
use crate::template;

#[derive(Debug, Parser)]
pub struct RunArgs {
    #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

pub fn execute(args: RunArgs) -> Result<()> {
    if args.command.is_empty() {
        anyhow::bail!("No command specified");
    }

    let mut all_refs: Vec<String> = Vec::new();

    for (_, value) in std::env::vars() {
        all_refs.extend(template::extract_references(&value));
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

    let mut env_vars: HashMap<String, String> = HashMap::new();
    for (key, value) in std::env::vars() {
        let resolved = template::resolve_variables(&value);
        let substituted = template::substitute_references(&resolved, resolver);
        env_vars.insert(key, substituted);
    }

    let command_args: Vec<String> = args
        .command
        .iter()
        .map(|arg| {
            let resolved = template::resolve_variables(arg);
            template::substitute_references(&resolved, resolver)
        })
        .collect();

    let (program, args) = command_args
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("Empty command"))?;

    let mut cmd = std::process::Command::new(program);
    cmd.args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}
