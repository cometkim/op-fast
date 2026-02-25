use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::cache::Cache;

#[derive(Debug, Parser)]
pub struct CacheCommand {
    #[clap(subcommand)]
    pub command: CacheSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CacheSubcommand {
    List,
    Clear { reference: Option<String> },
}

pub fn execute(cmd: CacheCommand) -> Result<()> {
    let cache = Cache::open()?;

    match cmd.command {
        CacheSubcommand::List => {
            let entries = cache.list()?;
            if entries.is_empty() {
                println!("Cache is empty");
                return Ok(());
            }

            println!("{:<60} {:<20} STATUS", "REFERENCE", "CACHED_AT");
            println!("{}", "-".repeat(90));

            for (reference, entry) in entries {
                let status = if entry.is_expired() {
                    "expired"
                } else {
                    "valid"
                };
                let cached_at = chrono_conversion(entry.cached_at);
                println!("{:<60} {:<20} {}", reference, cached_at, status);
            }
        }
        CacheSubcommand::Clear { reference } => match reference {
            Some(ref_str) => {
                let deleted = cache.delete(&ref_str)?;
                if deleted {
                    println!("Deleted: {}", ref_str);
                } else {
                    println!("Not found: {}", ref_str);
                }
            }
            None => {
                cache.clear()?;
                println!("Cache cleared");
            }
        },
    }

    Ok(())
}

fn chrono_conversion(timestamp: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let elapsed = now.saturating_sub(timestamp);

    if elapsed < 60 {
        format!("{}s ago", elapsed)
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else if elapsed < 86400 {
        format!("{}h ago", elapsed / 3600)
    } else {
        format!("{}d ago", elapsed / 86400)
    }
}
