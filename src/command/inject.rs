use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueHint};

use crate::cache::Cache;
use crate::delegate::OpDelegate;
use crate::template;

#[derive(Debug, Parser)]
pub struct InjectArgs {
    #[clap(value_hint = ValueHint::FilePath)]
    pub file: Option<PathBuf>,

    #[clap(short = 'f', long = "force")]
    pub force: bool,
}

pub fn execute(args: InjectArgs) -> Result<()> {
    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            s
        }
    };

    let resolved = template::resolve_variables(&input);

    let references: Vec<String> = template::extract_references(&resolved)
        .into_iter()
        .collect();

    if references.is_empty() {
        print!("{}", resolved);
        return Ok(());
    }

    let cache = Cache::open();
    let delegate = OpDelegate::new()?;

    let mut cached_values = std::collections::HashMap::new();
    let mut uncached_refs = Vec::new();

    match &cache {
        Ok(cache) => {
            for ref_str in &references {
                match cache.get(ref_str)? {
                    Some(value) => {
                        cached_values.insert(ref_str.clone(), value);
                    }
                    None => {
                        uncached_refs.push(ref_str.as_str());
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Cache unavailable: {}", e);
            uncached_refs = references.iter().map(|s| s.as_str()).collect();
        }
    }

    if !uncached_refs.is_empty() {
        log::debug!("Fetching {} uncached references", uncached_refs.len());
        let fetched = delegate.read_batch(&uncached_refs)?;

        if let Ok(cache) = &cache {
            for (ref_str, value) in &fetched {
                cache.put(ref_str, value)?;
            }
        }

        cached_values.extend(fetched);
    }

    let output =
        template::substitute_references(&resolved, |ref_str| cached_values.get(ref_str).cloned());

    print!("{}", output);
    Ok(())
}
