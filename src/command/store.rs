use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::store::Store;

#[derive(Debug, Parser)]
pub struct StoreCommand {
    #[clap(subcommand)]
    pub command: StoreSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum StoreSubcommand {
    List,
    Clear { reference: Option<String> },
}

pub fn execute(cmd: StoreCommand) -> Result<()> {
    let store = Store::open()?;

    match cmd.command {
        StoreSubcommand::List => {
            let entries = store.list()?;
            if entries.is_empty() {
                println!("Store is empty");
                return Ok(());
            }

            println!("{:<60} {:<20} STATUS", "REFERENCE", "STORED_AT");
            println!("{}", "-".repeat(90));

            for (reference, meta) in entries {
                let status = if meta.is_expired() {
                    "expired"
                } else {
                    "valid"
                };
                let stored_at = chrono_conversion(meta.stored_at);
                println!("{:<60} {:<20} {}", reference, stored_at, status);
            }
        }
        StoreSubcommand::Clear { reference } => match reference {
            Some(ref_str) => {
                let deleted = store.delete(&ref_str)?;
                if deleted {
                    println!("Deleted: {}", ref_str);
                } else {
                    println!("Not found: {}", ref_str);
                }
            }
            None => {
                store.clear()?;
                println!("Store cleared");
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
